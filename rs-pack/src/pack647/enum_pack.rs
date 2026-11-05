//! Enum config packer.

use super::param::{char_to_type_name, type_name_to_char};
use super::{collect_config_files, parse_config_sections};
use crate::cache::r#enum::EnumTypeRaw;
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;
use crate::types::{ParamValue, ScriptVarType};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

pub fn emit_enum_props(def: &EnumTypeRaw) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();

    // Emit opcodes in the original decode order. This is critical because
    // the Jagex cache sometimes has entries (op 5/6) before defaults (op 3/4),
    // and the packer must reproduce the exact same byte order.
    let mut entries_emitted = false;
    for &op in &def.opcode_order {
        match op {
            1 => {
                props.push(("inputtype".into(), char_to_type_name(def.inputtype as u8)));
            }
            2 => {
                props.push(("outputtype".into(), char_to_type_name(def.outputtype as u8)));
            }
            3 => {
                // Client defaults to "null", so any op 3 present in the binary
                // is meaningful — even if the string is empty.
                props.push((
                    "defaultstring".into(),
                    def.default_str.as_deref().unwrap_or("").to_string(),
                ));
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
                        ParamValue::String(s) => {
                            props.push(("val".into(), format!("{key},{s}")));
                        }
                        ParamValue::Int(v) => {
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
        if def.inputtype != ScriptVarType::Int {
            props.push(("inputtype".into(), char_to_type_name(def.inputtype as u8)));
        }
        if def.outputtype != ScriptVarType::Int {
            props.push(("outputtype".into(), char_to_type_name(def.outputtype as u8)));
        }
        if def.default_str.as_deref().is_some_and(|s| !s.is_empty()) {
            props.push((
                "defaultstring".into(),
                def.default_str.as_deref().unwrap_or("").to_string(),
            ));
        }
        if def.default_int != 0 {
            props.push(("defaultint".into(), def.default_int.to_string()));
        }
        for (key, value) in &def.ordered_entries {
            match value {
                ParamValue::String(s) => {
                    props.push(("val".into(), format!("{key},{s}")));
                }
                ParamValue::Int(v) => {
                    props.push(("val".into(), format!("{key},{v}")));
                }
            }
        }
    }
    props
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
) -> i32 {
    if let Some(v) = parse_enum_key(value) {
        return v;
    }
    match value {
        "null" => return -1,
        "yes" | "true" | "^true" => return 1,
        "no" | "false" | "^false" => return 0,
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
    resolved.unwrap_or(-1)
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
        // standard `EnumTypeRaw` decoder can load the server provider.
        if let Some(props) = configs.get(name) {
            let mut input_type: Option<char> = None;
            let mut input_is_autoint = false;
            let mut output_type: Option<char> = None;
            let mut default_val: Option<&str> = None;
            let mut vals: Vec<&str> = Vec::new();
            for (key, value) in props {
                match key.as_str() {
                    "inputtype" => {
                        input_is_autoint = value == "autoint";
                        input_type = Some(if input_is_autoint {
                            'i'
                        } else {
                            type_name_to_char(value)
                        });
                    }
                    "outputtype" => output_type = Some(type_name_to_char(value)),
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
                    server.p1(3);
                    server.pjstr(d);
                } else {
                    server.p1(4);
                    server.p4(resolve_enum_symbol(d, out_char, registry, name));
                }
            }
            if !vals.is_empty() {
                server.p1(if out_char == 's' { 5 } else { 6 });
                server.p2(vals.len() as u16);
                for (index, val) in vals.iter().enumerate() {
                    let key = if input_is_autoint {
                        index as i32
                    } else {
                        let key = val.split_once(',').map_or(*val, |(key, _)| key);
                        resolve_enum_symbol(key.trim(), in_char, registry, name)
                    };
                    let value = val.split_once(',').map_or(*val, |(_, value)| value);
                    server.p4(key);
                    if out_char == 's' {
                        server.pjstr(value);
                    } else {
                        server.p4(resolve_enum_symbol(value.trim(), out_char, registry, name));
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
        if !client_configs.contains_key(name) {
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
                        let c = type_name_to_char(value);
                        client.p1(1);
                        client.p1(c as u8);
                    }
                    "outputtype" => {
                        flush_entries(&mut client, &mut string_entries, &mut int_entries);
                        output_type = type_name_to_char(value) as u8;
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
        assert_eq!(resolve_enum_symbol("objbox", 'I', &reg, "t"), 519);
        assert_eq!(resolve_enum_symbol("doubleobjbox", 'I', &reg, "t"), 131);
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
            small
        );
    }
}
