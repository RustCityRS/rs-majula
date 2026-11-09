use super::config_entry;
use super::param::get_param_value_for_type;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::PackRegistry;
use crate::pack::packed_data::PackedData;
use crate::types::ParamValue;
use anyhow::{Result, bail};
use std::collections::HashMap;

/// The server's controller table: each controller type's name and the
/// values its `varcon=<varcon>,<value>` lines give its varcons, read as
/// each varcon's type. The cache's own table holds them empty.
pub fn pack_controllers(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedData> {
    let pack = &registry.controller;
    let configs = parse_config_sections_cached(file_cache, "controller", constants)?;
    let varcons = parse_config_sections_cached(file_cache, "varcon", constants)?;

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("controller", pack, &configs, id) else {
            server.finish_entry();
            continue;
        };

        let mut vars = Vec::new();
        for (key, value) in props {
            match key.as_str() {
                "varcon" => {
                    let Some((name, value)) = value.split_once(',') else {
                        bail!("[{debugname}]: varcon={value} needs a varcon and a value");
                    };
                    let name = name.trim();
                    let Some(varcon) = registry.varcon.get_by_debugname(name) else {
                        bail!("[{debugname}]: no varcon {name}");
                    };
                    let kind = varcons
                        .get(name)
                        .and_then(|props| props.iter().find(|(key, _)| key == "type"))
                        .map_or("int", |(_, kind)| kind.as_str());
                    let Some(value) = get_param_value_for_type(kind, registry, value.trim()) else {
                        bail!("[{debugname}]: varcon {name} has no {kind} {value}");
                    };
                    vars.push((varcon, value));
                }

                // not found
                _ => panic!("Unrecognized controller config key: {key}"),
            }
        }

        // 1
        if !vars.is_empty() {
            server.p1(1);
            server.p1(u8::try_from(vars.len()).expect("Too many controller varcons"));
            for (varcon, value) in vars {
                server.p2(varcon as u16);
                match value {
                    ParamValue::Int(value) => {
                        server.p1(0);
                        server.p4(value);
                    }
                    ParamValue::String(value) => {
                        server.p1(1);
                        server.pjstr(&value);
                    }
                }
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    Ok(server)
}
