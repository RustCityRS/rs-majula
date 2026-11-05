#![allow(clippy::if_same_then_else)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Varp (player variable) packers — two flavours sharing one 2004scape-style
//! text format:
//!
//! 1. **Server cache (legacy, untouched binary)**: [`pack_varps`] reads .varp
//!    text and writes `data/pack/server/varp.dat`/`varp.idx` via `PackedData`.
//!    Engine reads this at startup; binary output is unchanged.
//! 2. **Client cache (Phase 1 rebuild)**: [`emit_varp_props`] +
//!    [`pack_varp_def`] roundtrip through `VarPDefinition` for archive 2
//!    group 16 of the JS5 client cache. `pack_varp_def(...).encode()` produces
//!    byte-perfect cache bytes.
//!
//! Both paths read the SAME `.varp` text format, mirroring 2004scape's
//! `engine/tools/pack/config/VarpConfig.ts`: `scope=temp|perm`,
//! `type=<scriptvartype>`, `protect=yes|no` (op 4 sets protect=false),
//! `clientcode=N`, `transmit=yes|no`. The debugname (op 250) is the
//! `[section]` header in pack_varp_def's case; for pack_varps the debugname
//! is the registry-mapped name.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use super::param::char_to_type_name;
use super::{collect_config_files, parse_config_sections};
use crate::cache::varp::{VarPlayerScope, VarPlayerTypeRaw};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

pub fn pack_varps(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let files = collect_config_files(source_dir, "varp");
    info!("  Found {} .varp files", files.len());
    let configs = parse_config_sections(&files, constants);

    // Client reads only from all.varp (pure cache data)
    let all_file = source_dir.join("_unpack").join("all.varp");
    let client_configs = if all_file.exists() {
        parse_config_sections(&[all_file], constants)
    } else {
        HashMap::new()
    };
    info!(
        "  Parsed {} varp configs ({} client)",
        configs.len(),
        client_configs.len()
    );

    // Client count = max cache varp ID + 1 (from all.varp names mapped through registry).
    // Server count may be larger when content .varp files define server-only varps.
    let client_count = client_configs
        .keys()
        .filter_map(|name| registry.varp.get_by_name(name))
        .max()
        .map(|m| m + 1)
        .unwrap_or(registry.varp.max);

    let mut server = PackedData::new(registry.varp.max);
    let mut client = PackedData::new(client_count);

    for id in 0..registry.varp.max {
        server.start_entry();

        let Some(name) = registry.varp.get_by_id(id) else {
            server.skip_entry();
            if id < client_count {
                client.skip_entry();
            }
            continue;
        };

        // Server: all opcodes from merged configs
        if let Some(props) = configs.get(name) {
            for (key, value) in props {
                match key.as_str() {
                    "scope" => {
                        let v = match value.as_str() {
                            "perm" => Some(1u8),
                            "temp" => Some(0u8),
                            _ => value.parse::<u8>().ok(),
                        };
                        if let Some(v) = v {
                            server.p1(1);
                            server.p1(v);
                        }
                    }
                    "type" => {
                        let type_char = crate::pack::param::type_name_to_char(value);
                        server.p1(2);
                        server.p1(type_char as u8);
                    }
                    "protect" => {
                        if value == "no" {
                            server.p1(4);
                        }
                    }
                    "clientcode" => {
                        if let Ok(v) = value.parse::<u16>() {
                            server.p1(5);
                            server.p2(v);
                        }
                    }
                    "transmit" if value == "yes" => {
                        server.p1(6);
                    }
                    _ => {}
                }
            }
            server.p1(250);
            server.pjstr(name);
        }
        server.finish_entry();

        // Client: only clientcode (opcode 5) from all.varp (cache-derived entries only)
        if id < client_count {
            if let Some(props) = client_configs.get(name) {
                let mut has_clientcode = false;
                client.start_entry();
                for (key, value) in props {
                    if key == "clientcode" {
                        if let Ok(v) = value.parse::<u16>() {
                            client.p1(5);
                            client.p2(v);
                            has_clientcode = true;
                        }
                    }
                }
                if has_clientcode {
                    client.finish_entry();
                } else {
                    client.finish_entry();
                }
            } else {
                client.skip_entry();
            }
        }
    }

    server.save(server_dir, "varp")?;
    config_blob.add_packed_data("varp", client.dat, client.idx);
    Ok(())
}

pub fn emit_varp_props(def: &VarPlayerTypeRaw, _registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    match def.scope {
        VarPlayerScope::Perm => p.push((1, "scope".into(), "perm".into())),
        VarPlayerScope::Temp => {}
    }
    if def.opcode_order.contains(&2) {
        p.push((2, "type".into(), char_to_type_name(def.var_type as u8)));
    }
    if !def.protect {
        p.push((4, "protect".into(), "no".into()));
    }
    if def.clientcode != 0 {
        p.push((5, "clientcode".into(), def.clientcode.to_string()));
    }
    if def.transmit {
        p.push((6, "transmit".into(), "yes".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
