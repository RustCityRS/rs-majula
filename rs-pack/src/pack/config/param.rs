use super::config_entry;
use crate::ParamValue;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_bool, parse_coord, parse_script_var_type};
use crate::types::ScriptVarType;
#[cfg(before_647)]
use crate::types::{NpcStat, PlayerStat};
use anyhow::Result;
use std::collections::HashMap;
#[cfg(since_647)]
use tracing::warn;

pub fn pack_params(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.param;

    let configs = parse_config_sections_cached(file_cache, "param", constants);

    let mut server = PackedData::new(pack.max);
    #[cfg(since_647)]
    let client_configs = file_cache.client_configs("param", constants);
    #[cfg(since_647)]
    let client_count = client_configs
        .keys()
        .filter_map(|name| pack.get_by_debugname(name))
        .max()
        .map_or(0, |id| id + 1);
    #[cfg(since_647)]
    let mut client = PackedData::new(client_count);

    for id in 0..pack.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("param", pack, &configs, id) else {
            server.finish_entry();
            #[cfg(since_647)]
            if id < client_count {
                client.skip_entry();
            }
            continue;
        };

        encode_param(&mut server, props, registry, debugname);
        server.p1(250);
        server.pjstr(debugname);

        #[cfg(since_647)]
        if id < client_count {
            if let Some(props) = client_configs.get(debugname) {
                client.start_entry();
                encode_param(&mut client, props, registry, debugname);
                client.finish_entry();
            } else {
                client.skip_entry();
            }
        }

        // done
        server.finish_entry();
    }

    #[cfg(before_274)]
    let client = None;

    #[cfg(all(since_274, before_647))]
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

        Some(client)
    };

    #[cfg(since_647)]
    let client = Some(client);

    Ok(PackedFile { server, client })
}

fn encode_param(
    packed: &mut PackedData,
    props: &[(String, String)],
    registry: &PackRegistry,
    debugname: &str,
) {
    let ptype = props
        .iter()
        .find(|(key, _)| key == "type")
        .map(|(_, value)| value.as_str());
    #[cfg(before_647)]
    let ptype = ptype.unwrap_or_else(|| panic!("Param config: {debugname} is missing type"));
    #[cfg(since_647)]
    let ptype = ptype.unwrap_or("int");
    for (key, value) in props {
        match key.as_str() {
            "type" => parse_script_var_type(value, |value| {
                packed.p1(1);
                packed.p1(value as u8);
            }),
            "default" => {
                match get_param_value_for_type(ptype, registry, value)
                    .unwrap_or_else(|| panic!("Unknown param type '{ptype}' for [{debugname}]"))
                {
                    ParamValue::Int(value) => {
                        packed.p1(2);
                        packed.p4(value);
                    }
                    ParamValue::String(value) => {
                        packed.p1(5);
                        packed.pjstr(&value);
                    }
                }
            }
            #[cfg(since_647)]
            "defaultint" => {
                packed.p1(2);
                packed.p4(value.parse().expect("Invalid param default"));
            }
            "autodisable" => parse_bool(value, |value| {
                if !value {
                    packed.p1(4);
                }
            }),
            #[cfg(since_647)]
            "defaultstring" => {
                packed.p1(5);
                packed.pjstr(value);
            }
            _ => panic!("Unrecognized param config key: {key}"),
        }
    }
}

pub fn load_param_types(
    file_cache: &FileCache,
    constants: &HashMap<String, String>,
) -> HashMap<String, String> {
    parse_config_sections_cached(file_cache, "param", constants)
        .into_iter()
        .map(|(name, props)| {
            let ty = props
                .into_iter()
                .find(|(key, _)| key == "type")
                .map_or_else(|| "int".to_string(), |(_, ty)| ty);
            (name, ty)
        })
        .collect()
}

pub fn get_param_value_for_type(
    kind: &str,
    registry: &PackRegistry,
    value: &str,
) -> Option<ParamValue> {
    if kind == "string" {
        return Some(ParamValue::String(
            if value == "null" { "" } else { value }.into(),
        ));
    }
    if value == "null" {
        return Some(ParamValue::Int(-1));
    }
    if let Ok(value) = value.parse::<i32>() {
        return Some(ParamValue::Int(value));
    }
    if let Ok(value) = value.parse::<u32>() {
        return Some(ParamValue::Int(value as i32));
    }
    let pack = match kind {
        "int" => return Some(ParamValue::Int(-1)),
        "coord" => return Some(ParamValue::Int(parse_coord(value).unwrap_or(-1))),
        "boolean" => {
            let mut result = -1;
            parse_bool(value, |value| result = value as i32);
            return Some(ParamValue::Int(result));
        }
        "obj" | "namedobj" => &registry.obj,
        "npc" => &registry.npc,
        "loc" => &registry.loc,
        #[cfg(since_647)]
        "component" if value.contains(':') => {
            return Some(ParamValue::Int(
                registry.component_hashes.get(value).copied().unwrap_or(-1),
            ));
        }
        "component" => &registry.interface,
        "interface" if value.contains(':') => return Some(ParamValue::Int(-1)),
        "interface" => &registry.interface,
        "enum" => &registry.r#enum,
        "struct" => &registry.r#struct,
        #[cfg(before_647)]
        "stat" => return Some(ParamValue::Int(PlayerStat::from_config_str(value) as i32)),
        #[cfg(before_647)]
        "npc_stat" => return Some(ParamValue::Int(NpcStat::from_config_str(value) as i32)),
        #[cfg(since_647)]
        "stat" | "npc_stat" => &registry.stat,
        "seq" => &registry.seq,
        "synth" => &registry.synth,
        "inv" => &registry.inv,
        "spotanim" => &registry.spotanim,
        "varp" => &registry.varp,
        "model" => {
            let id = registry.model.get_by_debugname(value).or_else(|| {
                value
                    .strip_prefix("model_")
                    .and_then(|rest| rest.split('_').next())
                    .and_then(|id| id.parse().ok())
            });
            return Some(ParamValue::Int(id.map_or(-1, |id| id as i32)));
        }
        "player_uid" | "npc_uid" => return Some(ParamValue::Int(-1)),
        #[cfg(since_647)]
        "mapelement" => return Some(ParamValue::Int(-1)),
        "category" => &registry.category,
        "idkit" => &registry.idk,
        "dbrow" => &registry.dbrow,
        "midi" => &registry.midi,
        _ => return None,
    };
    Some(ParamValue::Int(
        pack.get_by_debugname(value).map_or(-1, |id| id as i32),
    ))
}

pub(super) fn resolve_typed_value(value: &str, type_char: u8, registry: &PackRegistry) -> i32 {
    let ty = ScriptVarType::try_from(type_char).expect("Invalid config type");
    match get_param_value_for_type(ty.type_name(), registry, value) {
        Some(ParamValue::Int(value)) => value,
        _ => panic!("Expected integer value for {}: {value}", ty.type_name()),
    }
}

pub fn parse_params<'a>(
    registry: &PackRegistry,
    param_types: &HashMap<String, String>,
    packed: &mut PackedData,
    props: impl IntoIterator<Item = &'a (String, String)>,
    name: &str,
) {
    let params: Vec<_> = props
        .into_iter()
        .filter(|(key, _)| matches!(key.as_str(), "param" | "paramint" | "paramstr"))
        .filter_map(|(key, value)| {
            value
                .split_once(',')
                .map(|(name, value)| (key.as_str(), name.trim(), value))
        })
        .filter(|(_, _pname, _)| {
            #[cfg(since_647)]
            if registry.param.get_by_debugname(_pname).is_none() && _pname.parse::<u32>().is_err() {
                warn!("Unknown param '{_pname}' for [{name}]");
                return false;
            }
            true
        })
        .collect();
    if !params.is_empty() {
        packed.p1(249);
        packed.p1(u8::try_from(params.len()).expect("Too many params"));
        for (kind, pname, pval) in params {
            #[cfg(before_647)]
            let pval = pval.trim();
            let pid = registry.param.get_by_debugname(pname);
            #[cfg(since_647)]
            let pid = pid.or_else(|| pname.parse().ok());
            let pid = pid.unwrap_or_else(|| panic!("Unknown param '{pname}' for [{name}]"));
            let value = match kind {
                "paramstr" => ParamValue::String(pval.into()),
                "paramint" => ParamValue::Int(pval.trim().parse().expect("Invalid integer param")),
                _ => {
                    let ptype = param_types.get(pname).map(String::as_str);
                    #[cfg(since_647)]
                    let ptype = ptype.or(Some("int"));
                    let ptype = ptype
                        .unwrap_or_else(|| panic!("Unknown param type '{pname}' for [{name}]"));
                    get_param_value_for_type(ptype, registry, pval).unwrap_or_else(|| {
                        panic!("Unknown param type '{ptype}' for [{name}] for value [{pval}]")
                    })
                }
            };
            packed.pbool(matches!(value, ParamValue::String(_)));
            packed.p3(pid as i32);
            match value {
                ParamValue::Int(value) => packed.p4(value),
                ParamValue::String(value) => packed.pjstr(&value),
            }
        }
    }
}

#[cfg(since_647)]
pub fn type_name_to_char(name: &str) -> char {
    if let Some(hex) = name.strip_prefix("byte:0x") {
        return u8::from_str_radix(hex, 16).expect("Invalid script type byte") as char;
    }
    match name {
        "namedobj" => ScriptVarType::NamedObj,
        _ => ScriptVarType::from_type_name(name),
    }
    .to_char()
}
