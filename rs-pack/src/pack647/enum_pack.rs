#![allow(clippy::unnecessary_get_then_check)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Enum config packer.
//!
//! Two flavors:
//! - [`pack_enums`] — legacy server-side overlay (`enum.dat`/`enum.idx`).
//!   Only emits the keys 2004scape's `EnumConfig.ts` recognises.
//! - [`pack_enum_def`] — full cache-targeting packer. Builds a complete
//!   [`EnumDefinition`] (with `emissions` in source order) from the text
//!   representation. Combined with [`DefinitionEncode::encode`], this
//!   produces byte-perfect cache bytes for archive 17.
//!
//! The cache packer is the path to "build the cache from source"; the
//! overlay packer remains during the transition until every type has a
//! cache packer.

use super::{collect_config_files, parse_config_sections};
use crate::cache647::{EnumType, EnumValue};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

/// Walk an enum's emissions and produce the (key, value) pairs that
/// [`pack_enum_def`] reads back into an identical [`EnumDefinition`].
/// Source order is preserved; duplicate emissions become duplicate lines.
/// This is the inverse of `pack_enum_def` and lives here so pack/unpack
/// stay symmetric in one module.
pub fn emit_enum_props(def: &EnumType) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();

    // Emit opcodes in the original decode order. This is critical because
    // the Jagex cache sometimes has entries (op 5/6) before defaults (op 3/4),
    // and the packer must reproduce the exact same byte order.
    let mut entries_emitted = false;
    for &op in &def.opcode_order {
        match op {
            1 => {
                if def.key_type != 0 {
                    props.push(("inputtype".into(), char_to_type_name(def.key_type)));
                }
            }
            2 => {
                if def.value_type != 0 {
                    props.push(("outputtype".into(), char_to_type_name(def.value_type)));
                }
            }
            3 => {
                // Client defaults to "null", so any op 3 present in the binary
                // is meaningful — even if the string is empty.
                props.push(("defaultstring".into(), def.default_string.to_string()));
            }
            4 => {
                // Op 4 was explicitly present — always emit.
                props.push(("defaultint".into(), def.default_int.to_string()));
            }
            5 | 6 if !entries_emitted => {
                entries_emitted = true;
                // Entries — emit in decode order, using ordered_entries
                // which preserves duplicates with their original values.
                for (key, value) in &def.ordered_entries {
                    match value {
                        EnumValue::String(s) => {
                            props.push(("val".into(), format!("{key},{s}")));
                        }
                        EnumValue::Int(v) => {
                            props.push(("val".into(), format!("{key},{v}")));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Fallback: if opcode_order is empty (e.g. hand-written configs), use
    // the legacy fixed order.
    if def.opcode_order.is_empty() {
        if def.key_type != 0 {
            props.push(("inputtype".into(), char_to_type_name(def.key_type)));
        }
        if def.value_type != 0 {
            props.push(("outputtype".into(), char_to_type_name(def.value_type)));
        }
        if !def.default_string.is_empty() {
            props.push(("defaultstring".into(), def.default_string.to_string()));
        }
        if def.default_int != 0 {
            props.push(("defaultint".into(), def.default_int.to_string()));
        }
        for (key, value) in &def.ordered_entries {
            match value {
                EnumValue::String(s) => {
                    props.push(("val".into(), format!("{key},{s}")));
                }
                EnumValue::Int(v) => {
                    props.push(("val".into(), format!("{key},{v}")));
                }
            }
        }
    }
    props
}

/// Inverse of `pack::param::type_name_to_char`. Returns the canonical
/// name when known, otherwise the `byte:0xNN` numeric escape so unknown
/// type bytes round-trip exactly.
fn char_to_type_name(c: u8) -> String {
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

fn parse_enum_key(s: &str) -> Option<i32> {
    s.parse::<i32>()
        .ok()
        .or_else(|| s.parse::<u32>().ok().map(|v| v as i32))
}

/// Resolve an authored enum key/value to an int: numeric literal, `null`
/// (-1), boolean constants, or a registry symbol looked up by type char.
/// Unresolvable values pack as -1, matching Jordan's rev-225 packer
/// (`get_param_value_for_type`'s `unwrap_or(-1)`), with a warning for
/// visibility — e.g. `[stat_members] default=stat` (int output) is -1.
fn resolve_enum_symbol(
    value: &str,
    type_char: char,
    registry: &PackRegistry,
    enum_name: &str,
) -> Option<i32> {
    if let Some(v) = parse_enum_key(value) {
        return Some(v);
    }
    match value {
        "null" => return Some(-1),
        "yes" | "true" | "^true" => return Some(1),
        "no" | "false" | "^false" => return Some(0),
        _ => {}
    }
    // Coords (`level_mx_mz_lx_lz`, as 289's fishing spots list their tiles),
    // `iface:comp` component hashes (raw hashes are caught by
    // parse_enum_key above) and every registry, as params and dbrows have
    // them.
    let resolved = super::dbrow::lookup_typed(value, type_char as u8, registry);
    if resolved.is_none() {
        tracing::warn!(
            "  enum [{}]: unresolved '{}' (type '{}') — packed as -1 (rev-225 parity)",
            enum_name,
            value,
            type_char
        );
    }
    Some(resolved.unwrap_or(-1))
}

pub fn pack_enums(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let files = collect_config_files(source_dir, "enum");
    info!("  Found {} .enum files", files.len());
    let configs = parse_config_sections(&files, constants);

    let all_file = source_dir.join("_unpack").join("all.enum");
    let client_configs = if all_file.exists() {
        parse_config_sections(&[all_file], constants)
    } else {
        HashMap::new()
    };
    info!(
        "  Parsed {} enum configs ({} client)",
        configs.len(),
        client_configs.len()
    );

    let mut server = PackedData::new(registry.r#enum.max);
    let mut client = PackedData::new(registry.r#enum.max);

    for id in 0..registry.r#enum.max {
        let Some(name) = registry.r#enum.get_by_id(id) else {
            server.skip_entry();
            client.skip_entry();
            continue;
        };

        server.start_entry();
        client.start_entry();

        // Server: cache binary format (batched op 5/6 + op 250 debugname),
        // with symbol resolution for typed keys/values — so authored enums
        // like `val=1,attack` (outputtype=stat) pack correctly and the
        // standard `EnumType` decoder can read `enum.dat` back for the
        // server-only enum overlay in `load_packed_config`.
        if let Some(props) = configs.get(name) {
            let mut input_type: Option<char> = None;
            let mut output_type: Option<char> = None;
            let mut default_val: Option<&str> = None;
            let mut vals: Vec<&str> = Vec::new();
            for (key, value) in props {
                match key.as_str() {
                    "inputtype" => input_type = Some(crate::pack::param::type_name_to_char(value)),
                    "outputtype" => {
                        output_type = Some(crate::pack::param::type_name_to_char(value))
                    }
                    "default" | "defaultint" | "defaultstring" => default_val = Some(value),
                    "val" => vals.push(value),
                    _ => {}
                }
            }
            let in_char = input_type.unwrap_or('i');
            let out_char = output_type.unwrap_or('i');

            if let Some(c) = input_type {
                server.p1(1);
                server.p1(c as u8);
            }
            if let Some(c) = output_type {
                server.p1(2);
                server.p1(c as u8);
            }
            if let Some(d) = default_val {
                if out_char == 's' {
                    if d != "null" {
                        server.p1(3);
                        server.pjstr(d);
                    }
                } else if let Some(v) = resolve_enum_symbol(d, out_char, registry, name) {
                    server.p1(4);
                    server.p4(v);
                }
            }
            if !vals.is_empty() {
                if out_char == 's' {
                    let entries: Vec<(i32, &str)> = vals
                        .iter()
                        .filter_map(|val| {
                            let (k, v) = val.split_once(',')?;
                            let key = resolve_enum_symbol(k.trim(), in_char, registry, name)?;
                            Some((key, v.trim()))
                        })
                        .collect();
                    server.p1(5);
                    server.p2(entries.len() as u16);
                    for (k, v) in entries {
                        server.p4(k);
                        server.pjstr(v);
                    }
                } else {
                    let entries: Vec<(i32, i32)> = vals
                        .iter()
                        .filter_map(|val| {
                            let (k, v) = val.split_once(',')?;
                            let key = resolve_enum_symbol(k.trim(), in_char, registry, name)?;
                            let value = resolve_enum_symbol(v.trim(), out_char, registry, name)?;
                            Some((key, value))
                        })
                        .collect();
                    server.p1(6);
                    server.p2(entries.len() as u16);
                    for (k, v) in entries {
                        server.p4(k);
                        server.p4(v);
                    }
                }
            }
            server.p1(250);
            server.pjstr(name);
        }

        // Client: cache binary format (batched val entries, source order)
        //
        // Consecutive `val=` lines are batched into a single opcode 5/6
        // block, but blocks are flushed *in the order they appear* relative
        // to other opcodes.  This preserves the source cache order (often
        // entries appear before default values).
        //
        // Server-only enums (registered in enum.pack but absent from
        // all.enum) must be SKIPPED here, not emitted empty — an empty
        // entry still writes a terminator byte, which adds a file to
        // client archive 17 and breaks the crc-gate.
        if client_configs.get(name).is_none() {
            server.finish_entry();
            client.skip_entry();
            continue;
        }
        if let Some(props) = client_configs.get(name) {
            let mut output_type: u8 = b'i';
            let mut string_entries: Vec<(i32, String)> = Vec::new();
            let mut int_entries: Vec<(i32, i32)> = Vec::new();

            // Helper closure: flush any accumulated val entries to `client`.
            let flush_entries =
                |client: &mut PackedData,
                 string_entries: &mut Vec<(i32, String)>,
                 int_entries: &mut Vec<(i32, i32)>| {
                    if !string_entries.is_empty() {
                        client.p1(5);
                        client.p2(string_entries.len() as u16);
                        for (k, v) in string_entries.drain(..) {
                            client.p4(k);
                            client.pjstr(&v);
                        }
                    }
                    if !int_entries.is_empty() {
                        client.p1(6);
                        client.p2(int_entries.len() as u16);
                        for (k, v) in int_entries.drain(..) {
                            client.p4(k);
                            client.p4(v);
                        }
                    }
                };

            for (key, value) in props {
                match key.as_str() {
                    "inputtype" => {
                        flush_entries(&mut client, &mut string_entries, &mut int_entries);
                        let c = crate::pack::param::type_name_to_char(value);
                        client.p1(1);
                        client.p1(c as u8);
                    }
                    "outputtype" => {
                        flush_entries(&mut client, &mut string_entries, &mut int_entries);
                        output_type = crate::pack::param::type_name_to_char(value) as u8;
                        client.p1(2);
                        client.p1(output_type);
                    }
                    "defaultstring" => {
                        flush_entries(&mut client, &mut string_entries, &mut int_entries);
                        client.p1(3);
                        client.pjstr(value);
                    }
                    "defaultint" => {
                        flush_entries(&mut client, &mut string_entries, &mut int_entries);
                        if let Ok(v) = value.parse::<i32>() {
                            client.p1(4);
                            client.p4(v);
                        }
                    }
                    "val" => {
                        if let Some((k, v)) = value.split_once(',') {
                            let key_int = parse_enum_key(k.trim());
                            if let Some(key_int) = key_int {
                                if output_type == b's' {
                                    string_entries.push((key_int, v.to_string()));
                                } else if let Some(val_int) = parse_enum_key(v.trim()) {
                                    int_entries.push((key_int, val_int));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Flush any trailing val entries
            flush_entries(&mut client, &mut string_entries, &mut int_entries);
        }

        server.finish_entry();
        client.finish_entry();
    }

    server.save(server_dir, "enum")?;
    config_blob.add_packed_data("enum", client.dat, client.idx);
    Ok(())
}

#[cfg(test)]
mod dialogue_slot_tests {
    use super::*;
    use crate::pack_registry::PackRegistry;
    use std::path::Path;

    // Verifies the interface-symbol resolution added for the dialogue_slot_enum:
    // bare interface names resolve via interface.pack, and the 'I' arm of
    // resolve_enum_symbol returns the interface id (not the -1 fallback).
    #[test]
    fn interface_names_resolve_in_enums() {
        // Unit tests run with cwd = rs-pack/, one level below the repo-root-relative PACK_DIR.
        let reg =
            PackRegistry::load(&Path::new("..").join(crate::PACK_DIR)).expect("load registry");
        assert_eq!(reg.interface.get_by_name("objbox"), Some(519));
        assert_eq!(reg.interface.get_by_name("doubleobjbox"), Some(131));
        // 'I' = interface/component type char (param::type_name_to_char).
        assert_eq!(resolve_enum_symbol("objbox", 'I', &reg, "t"), Some(519));
        assert_eq!(
            resolve_enum_symbol("doubleobjbox", 'I', &reg, "t"),
            Some(131)
        );
    }

    // Verifies symbolic component values (`iface:comp`) resolve to the hash.
    #[test]
    fn component_symbols_resolve_in_enums() {
        let reg =
            PackRegistry::load(&Path::new("..").join(crate::PACK_DIR)).expect("load registry");
        let small = ((752u32 << 16) | 12) as i32;
        assert_eq!(
            reg.component_hashes.get("chatbox:com_dialogue_box_small"),
            Some(&small)
        );
        assert_eq!(
            resolve_enum_symbol("chatbox:com_dialogue_box_small", 'I', &reg, "t"),
            Some(small)
        );
    }
}
