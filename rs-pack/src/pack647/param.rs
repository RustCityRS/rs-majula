//! Param config packer.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::{info, warn};

use crate::cache::param::ParamTypeRaw;

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;
use crate::types::ScriptVarType;

pub fn emit_param_props(def: &ParamTypeRaw, _registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        p.push((1, "type".into(), char_to_type_name(def.var_type as u8)));
    }
    if def.opcode_order.contains(&2) {
        p.push((2, "defaultint".into(), def.default_int.to_string()));
    }
    if !def.autodisable {
        p.push((4, "autodisable".into(), "no".into()));
    }
    if let Some(ds) = &def.default_str {
        p.push((5, "defaultstring".into(), ds.to_string()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}

pub(super) fn char_to_type_name(c: u8) -> String {
    let name: Option<&'static str> = match c {
        b'i' => Some("int"),
        b's' => Some("string"),
        b'c' => Some("coord"),
        b'o' => Some("obj"),
        b'O' => Some("namedobj"),
        b'n' => Some("npc"),
        b'l' => Some("loc"),
        b'I' => Some("component"),
        b'1' => Some("boolean"),
        b'g' => Some("enum"),
        b'J' => Some("struct"),
        b'S' => Some("stat"),
        b'A' => Some("seq"),
        b'P' => Some("synth"),
        b'v' => Some("inv"),
        b't' => Some("spotanim"),
        b'V' => Some("varp"),
        b'm' => Some("model"),
        0xb5 => Some("mapelement"),
        b'y' => Some("category"),
        b'K' => Some("idkit"),
        b'p' => Some("player_uid"),
        b'N' => Some("npc_uid"),
        0xef => Some("dbrow"),
        _ => None,
    };
    match name {
        Some(n) => n.to_string(),
        None => format!("byte:0x{c:02x}"),
    }
}

pub fn type_name_to_char(name: &str) -> char {
    match name {
        "int" => 'i',
        "string" => 's',
        "coord" => 'c',
        "obj" => 'o',
        "namedobj" => 'O',
        "npc" => 'n',
        "loc" => 'l',
        "component" | "interface" => 'I',
        "boolean" => '1',
        "enum" => 'g',
        "struct" => 'J',
        "stat" | "npc_stat" => 'S',
        "seq" => 'A',
        "synth" => 'P',
        "midi" => 'M',
        "inv" => 'v',
        "spotanim" => 't',
        "varp" => 'V',
        "model" => 'm',
        "mapelement" => '\u{00b5}',
        "category" => 'y',
        "idkit" => 'K',
        "player_uid" => 'p',
        "npc_uid" => 'N',
        "dbrow" => '\u{00ef}',
        other => {
            if let Some(hex) = other.strip_prefix("byte:0x") {
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    return v as char;
                }
            }
            ScriptVarType::from_type_name(other).to_char()
        }
    }
}

// ─── Legacy server-cache packer (data/pack/server/param.dat) ─────────────
//
// Same .param text format as 2004scape's `ParamConfig.ts`: `type=<name>`,
// `default=<value>` (overloaded — string or int based on the type),
// `autodisable=yes|no`. Binary output is the rs-server's startup-loadable
// PackedData blob.
pub fn pack_params(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    output_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    // Split param files: specific .param files (hand-written) vs _unpack/all.param.
    // Parsed separately to avoid cross-file duplicate panics — a param can
    // appear in both all.param (empty cache placeholder for the client) and a
    // specific .param (server-only fields like type/default). The server
    // stream prefers the specific file; the client stream reads only all.param.
    let all_files = collect_config_files(source_dir, "param");
    let all_file = source_dir.join("_unpack").join("all.param");
    let specific_files: Vec<_> = all_files
        .into_iter()
        .filter(|p| !p.components().any(|c| c.as_os_str() == "_unpack"))
        .collect();
    info!("  Found {} specific .param files", specific_files.len());

    let client_configs = if all_file.exists() {
        parse_config_sections(&[all_file], constants)
    } else {
        HashMap::new()
    };
    info!(
        "  Parsed {} client param configs from all.param",
        client_configs.len()
    );

    // Server configs: start with all.param, then overlay specific files.
    // Specific-file definitions win over all.param for the server stream.
    let specific_configs = if specific_files.is_empty() {
        HashMap::new()
    } else {
        parse_config_sections(&specific_files, constants)
    };
    let mut configs = client_configs.clone();
    for (name, props) in &specific_configs {
        configs.insert(name.clone(), props.clone());
    }
    info!("  Merged {} total param configs", configs.len());

    let client_count = client_configs
        .keys()
        .filter_map(|name| registry.param.get_by_name(name))
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    let mut packed = PackedData::new(registry.param.max);
    let mut client = PackedData::new(client_count);

    for id in 0..registry.param.max {
        packed.start_entry();

        let Some(name) = registry.param.get_by_id(id) else {
            packed.finish_entry();
            if id < client_count {
                client.skip_entry();
            }
            continue;
        };

        // ── Server stream ───────────────────────────────────────────────
        if let Some(props) = configs.get(name) {
            let type_char = props
                .iter()
                .find(|(k, _)| k == "type")
                .map(|(_, v)| type_name_to_char(v));
            for (key, value) in props {
                match key.as_str() {
                    "type" => {
                        packed.p1(1);
                        packed.p1(type_name_to_char(value) as u8);
                    }
                    "default" => {
                        if let Some(t) = type_char.filter(|t| *t != 's')
                            && value != "null"
                            && value.parse::<i32>().is_err()
                        {
                            // A named default on an int-like param
                            // (`default=bones` on a namedobj, `=no` on a
                            // boolean) is that thing's id. As a string it
                            // read back as 0: every npc without its own
                            // death_drop dropped obj 0, not bones.
                            let v = param_int(value, t, registry);
                            if v == -1 {
                                warn!("param [{name}]: default {value} names nothing");
                            }
                            packed.p1(2);
                            packed.p4(v);
                        } else if value == "null" && type_char == Some('s') {
                            // A string's null is the empty string, as the
                            // older revs pack it (pack::config::param). As an
                            // int -1 it left no default string, and the engine
                            // read the default back as the text "null".
                            packed.p1(5);
                            packed.pjstr("");
                        } else if value == "null" {
                            // Typed null for int-like params (obj/namedobj/loc/
                            // npc/...): RuneScript null is -1. Without this,
                            // "null" fell to DefaultString, so default_int stayed
                            // None and the engine's default_param() returned
                            // Int(unwrap_or(0)) = 0 — i.e. oc_param on an obj
                            // lacking the param returned 0, not null. (Latent for
                            // firemaking [product]; surfaced by next_obj_stage's
                            // terminal bite-stages.)
                            packed.p1(2);
                            packed.p4(-1);
                        } else if let Ok(v) = value.parse::<i32>() {
                            packed.p1(2);
                            packed.p4(v);
                        } else {
                            packed.p1(5);
                            packed.pjstr(value);
                        }
                    }
                    "defaultint" => {
                        let v: i32 = value.parse().unwrap_or(0);
                        packed.p1(2);
                        packed.p4(v);
                    }
                    "defaultstring" => {
                        packed.p1(5);
                        packed.pjstr(value);
                    }
                    "autodisable" if (value == "no" || value == "false") => {
                        packed.p1(4);
                    }
                    _ => {}
                }
            }
            packed.p1(250);
            packed.pjstr(name);
        }
        packed.finish_entry();

        // ── Client stream (from all.param — cache-derived) ──────────────
        if id < client_count {
            if let Some(props) = client_configs.get(name) {
                client.start_entry();
                for (key, value) in props {
                    match key.as_str() {
                        "type" => {
                            client.p1(1);
                            client.p1(type_name_to_char(value) as u8);
                        }
                        "defaultint" => {
                            let v: i32 = value.parse().unwrap_or(0);
                            client.p1(2);
                            client.p4(v);
                        }
                        "defaultstring" => {
                            client.p1(5);
                            client.pjstr(value);
                        }
                        "autodisable" if (value == "no" || value == "false") => {
                            client.p1(4);
                        }
                        _ => {}
                    }
                }
                client.finish_entry();
            } else {
                client.skip_entry();
            }
        }
    }

    packed.save(output_dir, "param")?;
    config_blob.add_packed_data("param", client.dat, client.idx);
    Ok(())
}

/// Each param's type, by id, from every `.param` file: the cache's
/// (`_unpack/all.param`) and the authored ones, which declare the server-only
/// params 289 content uses (`fish_wrong_spot_message` is a string,
/// `baitrequired` a namedobj).
pub(crate) fn param_types(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> HashMap<u32, char> {
    let configs =
        super::parse_config_sections(&super::collect_config_files(source_dir, "param"), constants);
    configs
        .iter()
        .filter_map(|(name, props)| {
            let id = registry.param.get_by_name(name)?;
            let (_, ty) = props.iter().find(|(k, _)| k == "type")?;
            Some((id, type_name_to_char(ty)))
        })
        .collect()
}

/// An int-typed param's value (on a struct, npc, ...): a number, a boolean, or a symbol named
/// in the namespace of the param's type -- `baitrequired,fishing_bait` is the
/// obj id of `fishing_bait`, not 0.
pub(crate) fn param_int(value: &str, type_char: char, registry: &PackRegistry) -> i32 {
    match (type_char, value) {
        ('1', "yes" | "true") => 1,
        ('1', "no" | "false") => 0,
        _ if value == "null" => -1,
        _ => super::dbrow::resolve_typed_value(value, type_char as u8, registry),
    }
}
