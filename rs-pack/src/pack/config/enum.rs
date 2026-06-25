use crate::ParamValue;
use crate::pack::config::param::get_param_value_for_type;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_script_var_type;
use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_enums(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.r#enum;

    let files = file_cache.collect("enum");
    debug!("  Found {} .enum files", files.len());

    let configs = parse_config_sections_cached(file_cache, "enum", constants);
    debug!("  Parsed {} enum configs", configs.len());

    let mut server = PackedData::new(pack.max);
    for id in 0..pack.max {
        server.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown enum id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown enum config: {debugname}");
        };

        let Some(inputtype) = props
            .iter()
            .find(|(k, _)| k == "inputtype")
            .map(|(_, v)| v.as_str())
        else {
            panic!("Enum config: {debugname} is missing inputtype");
        };

        let Some(outputtype) = props
            .iter()
            .find(|(k, _)| k == "outputtype")
            .map(|(_, v)| v.as_str())
        else {
            panic!("Enum config: {debugname} is missing outputtype");
        };

        // Collected fields written after the loop
        let mut vals: Vec<&str> = Vec::new();

        for (key, value) in props {
            match key.as_str() {
                // 1
                "inputtype" => {
                    server.p1(1);
                    if inputtype == "autoint" {
                        parse_script_var_type("int", |v| {
                            server.p1(v as u8);
                        });
                    } else {
                        parse_script_var_type(value, |v| {
                            server.p1(v as u8);
                        });
                    }
                }

                // 2
                "outputtype" => parse_script_var_type(value, |v| {
                    server.p1(2);
                    server.p1(v as u8);
                }),

                // 3 or 4
                "default" => {
                    let Some(pval) = get_param_value_for_type(outputtype, registry, value) else {
                        panic!(
                            "Unknown param type '{outputtype}' for [{debugname}] for value [{value}]"
                        );
                    };
                    match pval {
                        // 3
                        ParamValue::String(v) => {
                            server.p1(3);
                            server.pjstr(&v);
                        }
                        // 4
                        ParamValue::Int(v) => {
                            server.p1(4);
                            server.p4(v);
                        }
                    }
                }

                // 5 or 6
                "val" => vals.push(value),

                // not found
                _ => panic!("Unrecognized enum config key: {key}"),
            }
        }

        // handle 5 or 6
        if outputtype == "string" {
            // 5
            server.p1(5);
        } else {
            // 6
            server.p1(6);
        }

        server.p2(vals.len() as u16);

        let input_is_autoint = inputtype == "autoint";
        let output_is_autoint = outputtype == "autoint";

        for (index, val) in vals.iter().enumerate() {
            // input key
            if input_is_autoint {
                server.p4(index as i32);
            } else {
                let key = val.split_once(',').map(|(k, _)| k).unwrap_or(val);
                match get_param_value_for_type(inputtype, registry, key) {
                    Some(ParamValue::Int(v)) => server.p4(v),
                    _ => panic!(
                        "Param type '{inputtype}' for [{debugname}] value [{key}] must be int"
                    ),
                }
            }

            // output value
            let value = if output_is_autoint {
                val
            } else {
                val.find(',').map(|pos| &val[pos + 1..]).unwrap_or(val)
            };
            match get_param_value_for_type(outputtype, registry, value) {
                Some(ParamValue::Int(v)) => server.p4(v),
                Some(ParamValue::String(v)) => server.pjstr(&v),
                None => {
                    panic!("Unknown param type '{outputtype}' for [{debugname}] value [{value}]")
                }
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: None,
    })
}
