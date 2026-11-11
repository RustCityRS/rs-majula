#[cfg(since_647)]
use super::client_table_with;
use super::config_entry;
use crate::ParamValue;
use crate::pack::config::param::get_param_value_for_type;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_script_var_type;
use crate::types::ScriptVarType;
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_enums(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.r#enum;

    let configs = parse_config_sections_cached(file_cache, "enum", constants)?;

    let mut server = PackedData::new(pack.max);
    for id in 0..pack.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("enum", pack, &configs, id) else {
            server.finish_entry();
            continue;
        };

        encode_enum(&mut server, props, registry, debugname);

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    #[cfg(before_647)]
    let client = None;
    #[cfg(since_647)]
    let client = Some(client_table_with(
        file_cache,
        "enum",
        pack,
        constants,
        |client, name, props| encode_enum(client, props, registry, name),
    )?);

    Ok(PackedFile { server, client })
}

/// An enum's key or value type as packed. As in 2004scape, namedobj keeps its
/// own type ('O'), the one the compiler pushes for `enum(int, namedobj, ...)`,
/// where other configs pack it as obj.
fn enum_var_type(name: &str) -> ScriptVarType {
    if name == "namedobj" {
        return ScriptVarType::NamedObj;
    }
    let mut ty = ScriptVarType::Int;
    parse_script_var_type(name, |c| {
        ty = ScriptVarType::try_from(c as u8).expect("Invalid enum type");
    });
    ty
}

fn enum_value(ty: &str, registry: &PackRegistry, value: &str) -> Option<ParamValue> {
    if ty == "string" {
        return get_param_value_for_type(ty, registry, value);
    }
    match value.trim() {
        "yes" | "true" => Some(ParamValue::Int(1)),
        "no" | "false" => Some(ParamValue::Int(0)),
        value => get_param_value_for_type(ty, registry, value),
    }
}

fn encode_enum(
    packed: &mut PackedData,
    props: &[(String, String)],
    registry: &PackRegistry,
    debugname: &str,
) {
    let type_of = |key: &str| {
        props
            .iter()
            .find(|(k, _)| k == key)
            .map_or("int", |(_, v)| v.as_str())
    };
    let inputtype = type_of("inputtype");
    let outputtype = type_of("outputtype");

    for group in props.chunk_by(|a, b| (a.0 == "val") == (b.0 == "val")) {
        if group[0].0 == "val" {
            let vals: Vec<_> = group.iter().map(|(_, value)| value.as_str()).collect();
            pack_values(packed, &vals, inputtype, outputtype, registry, debugname);
            continue;
        }
        for (key, value) in group {
            match key.as_str() {
                // 1
                "inputtype" => {
                    packed.p1(1);
                    if inputtype == "autoint" {
                        packed.p1(ScriptVarType::Int as u8);
                    } else {
                        packed.p1(enum_var_type(value) as u8);
                    }
                }

                // 2
                "outputtype" => {
                    packed.p1(2);
                    packed.p1(enum_var_type(value) as u8);
                }

                // 3 or 4
                "default" => {
                    let Some(pval) = enum_value(outputtype, registry, value) else {
                        panic!(
                            "Unknown param type '{outputtype}' for [{debugname}] for value [{value}]"
                        );
                    };
                    match pval {
                        // 3
                        ParamValue::String(v) => {
                            packed.p1(3);
                            packed.pjstr(&v);
                        }
                        // 4
                        ParamValue::Int(v) => {
                            packed.p1(4);
                            packed.p4(v);
                        }
                    }
                }

                // not found
                _ => panic!("Unrecognized enum config key: {key}"),
            }
        }
    }
}

fn pack_values(
    packed: &mut PackedData,
    vals: &[&str],
    inputtype: &str,
    outputtype: &str,
    registry: &PackRegistry,
    debugname: &str,
) {
    // handle 5 or 6
    if outputtype == "string" {
        // 5
        packed.p1(5);
    } else {
        // 6
        packed.p1(6);
    }

    packed.p2(vals.len() as u16);

    let input_is_autoint = inputtype == "autoint";
    let output_is_autoint = outputtype == "autoint";

    for (index, val) in vals.iter().enumerate() {
        // input key
        if input_is_autoint {
            packed.p4(index as i32);
        } else {
            let key = val.split_once(',').map(|(k, _)| k).unwrap_or(val);
            match enum_value(inputtype, registry, key) {
                Some(ParamValue::Int(v)) => packed.p4(v),
                _ => panic!("Param type '{inputtype}' for [{debugname}] value [{key}] must be int"),
            }
        }

        // output value
        let value = if output_is_autoint {
            val
        } else {
            val.find(',').map(|pos| &val[pos + 1..]).unwrap_or(val)
        };
        match enum_value(outputtype, registry, value) {
            Some(ParamValue::Int(v)) => packed.p4(v),
            Some(ParamValue::String(v)) => packed.pjstr(&v),
            None => {
                panic!("Unknown param type '{outputtype}' for [{debugname}] value [{value}]")
            }
        }
    }
}

#[cfg(all(test, since_647))]
mod dialogue_slot_tests {
    use super::*;
    use std::path::Path;

    // Verifies the interface-symbol resolution added for the dialogue_slot_enum:
    // bare interface names resolve via interface.pack, and the 'I' arm of
    // resolve_enum_symbol returns the interface id (not the -1 fallback).
    #[test]
    fn interface_names_resolve_in_enums() {
        // Unit tests run with cwd = rs-pack/, one level below the repo-root-relative PACK_DIR.
        let reg =
            PackRegistry::load(&Path::new("..").join(crate::PACK_DIR)).expect("load registry");
        assert_eq!(reg.interface.get_by_debugname("objbox"), Some(519));
        assert_eq!(reg.interface.get_by_debugname("doubleobjbox"), Some(131));
        // 'I' = interface/component type char (param::type_name_to_char).
        assert_eq!(
            enum_value("component", &reg, "objbox")
                .and_then(|v| if let ParamValue::Int(v) = v {
                    Some(v)
                } else {
                    None
                })
                .unwrap(),
            519
        );
        assert_eq!(
            enum_value("component", &reg, "doubleobjbox")
                .and_then(|v| if let ParamValue::Int(v) = v {
                    Some(v)
                } else {
                    None
                })
                .unwrap(),
            131
        );
    }

    // Verifies symbolic component values (`iface:comp`) resolve to the hash.
    #[test]
    fn component_symbols_resolve_in_enums() {
        let reg =
            PackRegistry::load(&Path::new("..").join(crate::PACK_DIR)).expect("load registry");
        let small = ((752u32 << 16) | 12) as i32;
        assert_eq!(reg.component_hashes.get("chattop:chatmodal"), Some(&small));
        assert_eq!(
            enum_value("component", &reg, "chattop:chatmodal")
                .and_then(|v| if let ParamValue::Int(v) = v {
                    Some(v)
                } else {
                    None
                })
                .unwrap(),
            small
        );
    }
}
