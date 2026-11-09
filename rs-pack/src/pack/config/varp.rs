use super::config_entry;
#[cfg(since_647)]
use super::is_string_var;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_bool, parse_number, parse_script_var_type};
use crate::types::VarPlayerScope;
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_varps(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.varp;

    let configs = parse_config_sections_cached(file_cache, "varp", constants)?;

    #[cfg(since_647)]
    let mut client_configs = file_cache.client_configs("varp", pack, constants)?;
    #[cfg(since_647)]
    client_configs.retain(|_, props| !is_string_var(props));
    let mut server = PackedData::new(pack.max);
    #[cfg(before_647)]
    let client_count = pack.max;
    #[cfg(since_647)]
    let client_count = pack.client_count();
    let mut client = PackedData::new(client_count);

    for id in 0..pack.max {
        server.start_entry();
        if id < client_count {
            client.start_entry();
        }

        let Some((debugname, props)) = config_entry("varp", pack, &configs, id) else {
            server.skip_entry();
            if id < client_count {
                client.skip_entry();
            }
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "scope" => {
                    let v = VarPlayerScope::from_config_str(value);
                    if v != VarPlayerScope::Temp {
                        server.p1(1);
                        server.p1(v as u8);
                    }
                }

                // 2
                "type" => parse_script_var_type(value, |v| {
                    server.p1(2);
                    server.p1(v as u8);
                }),

                // 4
                "protect" => parse_bool(value, |v| {
                    if !v {
                        server.p1(4);
                    }
                }),

                // 5
                "clientcode" => parse_number(value, |v| {
                    #[cfg(before_647)]
                    {
                        client.p1(5);
                        client.p2(v);
                    }
                    server.p1(5);
                    server.p2(v);
                }),

                // 6
                "transmit" => parse_bool(value, |v| {
                    if v {
                        server.p1(6);
                    }
                }),

                // not found
                _ => panic!("Unrecognized varp config key: {key}"),
            }
        }

        #[cfg(since_647)]
        if id < client_count {
            if let Some(props) = client_configs.get(debugname) {
                for (key, value) in props {
                    if key == "clientcode" {
                        parse_number(value, |value| {
                            client.p1(5);
                            client.p2(value);
                        });
                    }
                }
            } else {
                client.skip_entry();
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        #[cfg(before_647)]
        client.finish_entry();
        #[cfg(since_647)]
        if id < client_count && client_configs.contains_key(debugname) {
            client.finish_entry();
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
