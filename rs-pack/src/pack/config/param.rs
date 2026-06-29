use std::collections::HashMap;

use anyhow::Result;
use tracing::debug;

use crate::ParamValue;
#[cfg(since_274)]
use crate::config_crc;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_bool;
use crate::pack::util::{parse_coord, parse_script_var_type};
use crate::types::{NpcStat, PlayerStat};
#[cfg(since_274)]
use rs_io::crc;

pub fn pack_params(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    #[cfg_attr(before_274, allow(unused_variables))] verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.param;

    let files = file_cache.collect("param");
    debug!("  Found {} .param files", files.len());

    let configs = parse_config_sections_cached(file_cache, "param", constants);
    debug!("  Parsed {} param configs", configs.len());

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown param id: {}", id);
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown param config: {}", debugname);
        };

        let Some(ptype) = props
            .iter()
            .find(|(k, _)| k == "type")
            .map(|(_, v)| v.as_str())
        else {
            panic!("Param config: {debugname} is missing type");
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "type" => parse_script_var_type(value, |v| {
                    server.p1(1);
                    server.p1(v as u8);
                }),

                // 2 or 5
                "default" => {
                    let Some(pval) = get_param_value_for_type(ptype, registry, value) else {
                        panic!(
                            "Unknown param type '{ptype}' for [{debugname}] for value [{value}]"
                        );
                    };
                    match pval {
                        // 3
                        ParamValue::String(v) => {
                            server.p1(5);
                            server.pjstr(&v);
                        }
                        // 4
                        ParamValue::Int(v) => {
                            server.p1(2);
                            server.p4(v);
                        }
                    }
                }

                // 4
                "autodisable" => parse_bool(value, |v| {
                    if !v {
                        server.p1(4);
                    }
                }),

                // not found
                _ => panic!("Unrecognized param config key: {key}"),
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    #[cfg(before_274)]
    let client = None;

    #[cfg(since_274)]
    let client = {
        // TODO: content needs to be rewritten in order to not hard code this ffs...
        #[cfg(rev = "274")]
        const CLIENT_PARAM_COUNT: u16 = 45;
        #[cfg(rev = "289")]
        const CLIENT_PARAM_COUNT: u16 = 54;
        let mut client = PackedData::new(CLIENT_PARAM_COUNT);
        for _ in 0..CLIENT_PARAM_COUNT {
            client.start_entry();
            client.finish_entry();
        }
        if verify {
            let crc = crc::getcrc(&client.dat, 0, client.dat.len());
            let expected = config_crc::PARAM;
            if crc != expected {
                panic!("CRC mismatch ['param']: Got: {crc}, Expected: {expected}");
            }
        }
        Some(client)
    };

    Ok(PackedFile { server, client })
}

pub fn load_param_types(file_cache: &FileCache) -> HashMap<String, String> {
    let files = file_cache.collect("param");
    let mut param_types: HashMap<String, String> = HashMap::new();

    for path in files {
        let Some(content) = file_cache.read(path) else {
            continue;
        };
        let mut current_name: Option<String> = None;
        let mut current_type = "int";

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if line.starts_with('[') {
                if let Some(name) = current_name.take() {
                    param_types.insert(name, current_type.to_string());
                }
                current_name = Some(line[1..line.len() - 1].to_string());
                current_type = "int";
                continue;
            }
            if let Some(eq) = line.find('=') {
                let key = &line[..eq];
                let value = &line[eq + 1..];
                if key == "type" {
                    current_type = value;
                }
            }
        }
        if let Some(name) = current_name {
            param_types.insert(name, current_type.to_string());
        }
    }

    param_types
}

pub fn get_param_value_for_type(
    str: &str,
    registry: &PackRegistry,
    value: &str,
) -> Option<ParamValue> {
    match str {
        "int" => Some(ParamValue::Int(value.parse::<i32>().unwrap_or(-1))),
        "string" => Some(ParamValue::String(if value == "null" {
            String::new().into_boxed_str()
        } else {
            value.to_string().into_boxed_str()
        })),
        "coord" => Some(ParamValue::Int(parse_coord(value).unwrap_or(-1))),
        "obj" | "namedobj" => Some(ParamValue::Int(
            registry
                .obj
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "npc" => Some(ParamValue::Int(
            registry
                .npc
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "loc" => Some(ParamValue::Int(
            registry
                .loc
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "component" => Some(ParamValue::Int(
            registry
                .interface
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "interface" => {
            let index = if value.contains(':') {
                -1i32
            } else {
                registry
                    .interface
                    .get_by_debugname(value)
                    .map(|id| id as i32)
                    .unwrap_or(-1)
            };
            Some(ParamValue::Int(index))
        }
        "boolean" => Some(ParamValue::Int(if value == "null" {
            -1
        } else {
            let mut result = -1i32;
            parse_bool(value, |v| result = v as i32);
            result
        })),
        "enum" => Some(ParamValue::Int(
            registry
                .r#enum
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "struct" => Some(ParamValue::Int(
            registry
                .r#struct
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "stat" => Some(ParamValue::Int(if value == "null" {
            -1
        } else {
            PlayerStat::from_config_str(value) as i32
        })),
        "npc_stat" => Some(ParamValue::Int(if value == "null" {
            -1
        } else {
            NpcStat::from_config_str(value) as i32
        })),
        "seq" => Some(ParamValue::Int(
            registry
                .seq
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "synth" => Some(ParamValue::Int(
            registry
                .synth
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "inv" => Some(ParamValue::Int(
            registry
                .inv
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "spotanim" => Some(ParamValue::Int(
            registry
                .spotanim
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "varp" => Some(ParamValue::Int(
            registry
                .varp
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        // "model" => Some(ParamValue::Int())
        // "mapelement" => {}
        "category" => Some(ParamValue::Int(
            registry
                .category
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        // "idkit" => Some(ParamValue::Int())
        // "player_uid" => Some(ParamValue::Int(-1)),
        // "npc_uid" => Some(ParamValue::Int(-1)),
        "dbrow" => Some(ParamValue::Int(
            registry
                .dbrow
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        "midi" => Some(ParamValue::Int(
            registry
                .midi
                .get_by_debugname(value)
                .map(|id| id as i32)
                .unwrap_or(-1),
        )),
        _ => None,
    }
}

pub fn parse_params(
    registry: &PackRegistry,
    param_types: &HashMap<String, String>,
    packed: &mut PackedData,
    props: &[(String, String)],
    name: &str,
) {
    let params: Vec<(&str, &str)> = props
        .iter()
        .filter(|(k, _)| k == "param")
        .filter_map(|(_, v)| v.split_once(','))
        .collect();
    if !params.is_empty() {
        packed.p1(249);
        packed.p1(params.len() as u8);
        for (pname, pval) in &params {
            let pname = pname.trim();
            let pval = pval.trim();
            let Some(pid) = registry.param.get_by_debugname(pname) else {
                panic!("Unknown param '{pname}' for [{name}]");
            };
            packed.p3(pid as i32);
            let Some(ptype) = param_types.get(pname) else {
                panic!("Unknown param type '{pname}' for [{name}]");
            };
            let Some(value) = get_param_value_for_type(ptype, registry, pval) else {
                panic!("Unknown param type '{ptype}' for [{name}] for value [{pval}]");
            };
            match value {
                ParamValue::Int(value) => {
                    packed.pbool(false);
                    packed.p4(value);
                }
                ParamValue::String(value) => {
                    packed.pbool(true);
                    packed.pjstr(&value);
                }
            }
        }
    }
}
