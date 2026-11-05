//! Struct config packers — two flavours sharing one 2004scape-style text
//! format:
//!
//! 1. **Server cache (legacy, untouched binary)**: [`pack_structs`] reads
//!    .struct text and writes `data/pack/server/struct.dat`/`struct.idx`
//!    via `PackedData`. Server-only; binary output is unchanged from prior
//!    revisions.
//! 2. **Client cache (Phase 1 rebuild)**: [`emit_struct_props`] +
//!    [`pack_struct_def`] roundtrip through `StructDefinition` for archive
//!    2 group 26 of the JS5 client cache. `pack_struct_def(...).encode()`
//!    produces byte-perfect cache bytes.
//!
//! Both paths read the SAME `.struct` text format. Mirrors 2004scape's
//! `engine/tools/pack/config/StructConfig.ts`. Struct has only opcode 249
//! (params bag with `u8 count` + `count × (g1 type, g3 key, g4 int /
//! gjstr str)`). There is NO opcode 250 / DebugName for struct — the
//! cache stores no debugname, and `StructDefinition::parse` has no 250
//! arm. The `[section]` header is therefore a content-side identifier
//! only; `pack_struct_def` does NOT push a DebugName emission.
//!
//! Param references resolve through `PackRegistry::param` — human-readable
//! param names in, numeric ids out. To preserve byte-perfect roundtrip
//! without forcing a param-type lookup at unpack time, we emit DISTINCT
//! text keys per `StructValue` variant:
//!
//! - `paramint=<param_name>,<i32>` for `Integer` value
//! - `paramstr=<param_name>,<string>` for `String` value (preserves
//!   trailing whitespace verbatim — caller must NOT trim)
//!
//! Unknown param ids (no entry in `param.pack`) fall back to `param_<id>`
//! on the emit side; `pack_struct_def` parses that fallback by stripping
//! the `param_` prefix and parsing the trailing integer.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::{info, warn};

use crate::cache::r#struct::StructTypeRaw;
use crate::types::ParamValue;

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

/// Walk a struct's emissions and produce `param=name,value` pairs matching
/// 2004scape's `.struct` text format. Type (int vs string) is determined
/// by the param definition, not the key name.
pub fn emit_struct_props(def: &StructTypeRaw, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    for (key, val) in def.params.as_deref().unwrap_or_default() {
        let pname = registry
            .param
            .get_by_id(*key as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("param_{key}"));
        match val {
            ParamValue::Int(i) => {
                props.push(("param".into(), format!("{pname},{i}")));
            }
            ParamValue::String(s) => {
                props.push(("param".into(), format!("{pname},{s}")));
            }
        }
    }
    props
}

// ─── Legacy server-cache packer (data/pack/server/struct.dat) ────────────
//
// Same .struct text format as the cache packer above; binary output is
// the rs-server's startup-loadable PackedData blob. Server cache is a
// separate store from the JS5 client cache and is never transmitted to
// the client.
pub fn pack_structs(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    output_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let files = collect_config_files(source_dir, "struct");
    info!("  Found {} .struct files", files.len());
    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} struct configs", configs.len());

    let param_types = super::param::param_types(source_dir, registry, constants);

    // Client reads from all.struct (cache-derived entries only).
    // Server reads from .struct content files which may add server-only structs.
    let all_file = source_dir.join("_unpack").join("all.struct");
    let client_configs = if all_file.exists() {
        parse_config_sections(&[all_file], constants)
    } else {
        HashMap::new()
    };
    info!(
        "  Parsed {} client struct configs from all.struct",
        client_configs.len()
    );

    let client_count = client_configs
        .keys()
        .filter_map(|name| registry.r#struct.get_by_name(name))
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    let mut packed = PackedData::new(registry.r#struct.max);
    let mut client = PackedData::new(client_count);

    for id in 0..registry.r#struct.max {
        packed.start_entry();

        let Some(name) = registry.r#struct.get_by_id(id) else {
            packed.skip_entry();
            if id < client_count {
                client.skip_entry();
            }
            continue;
        };

        // ── Server stream (from .struct content files) ──────────────────
        if let Some(props) = configs.get(name) {
            let params: Vec<(&str, &str)> = props
                .iter()
                .filter(|(k, _)| k == "param")
                .filter_map(|(_, v)| v.split_once(','))
                .collect();
            if !params.is_empty() {
                packed.p1(249);
                packed.p1(params.len() as u8);
                for &(pname, pval) in &params {
                    let pname = pname.trim();
                    let Some(pid) = registry.param.get_by_name(pname) else {
                        warn!("Unknown param '{}' for struct [{name}]", pname);
                        packed.pbool(false);
                        packed.p3(0);
                        packed.p4(0);
                        continue;
                    };
                    let type_char = param_types.get(&pid).copied().unwrap_or('i');
                    if type_char == 's' {
                        packed.pbool(true);
                        packed.p3(pid);
                        packed.pjstr(pval);
                    } else {
                        packed.pbool(false);
                        packed.p3(pid);
                        packed.p4(super::param::param_int(pval, type_char, registry));
                    }
                }
            }
            packed.p1(250);
            packed.pjstr(name);
        }
        packed.finish_entry();

        // ── Client stream (from all.struct — cache-derived) ─────────────
        if id < client_count {
            if let Some(props) = client_configs.get(name) {
                client.start_entry();
                let params: Vec<(&str, &str)> = props
                    .iter()
                    .filter(|(k, _)| k == "param")
                    .filter_map(|(_, v)| v.split_once(','))
                    .collect();
                if !params.is_empty() {
                    client.p1(249);
                    client.p1(params.len() as u8);
                    for &(pname, pval) in &params {
                        let pname = pname.trim();
                        let Some(pid) = registry.param.get_by_name(pname) else {
                            warn!("Unknown param '{}' for struct [{name}] (client)", pname);
                            client.pbool(false);
                            client.p3(0);
                            client.p4(0);
                            continue;
                        };
                        let is_string = param_types.get(&pid) == Some(&'s');
                        if is_string {
                            client.pbool(true);
                            client.p3(pid);
                            client.pjstr(pval);
                        } else {
                            let int_val: i32 = pval.parse().unwrap_or(0);
                            client.pbool(false);
                            client.p3(pid);
                            client.p4(int_val);
                        }
                    }
                }
                client.finish_entry();
            } else {
                client.skip_entry();
            }
        }
    }

    packed.save(output_dir, "struct")?;
    config_blob.add_packed_data("struct", client.dat, client.idx);
    Ok(())
}
