use std::collections::HashMap;

use anyhow::Result;
use tracing::debug;

use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_bool, parse_number, parse_obj};
use crate::types::InvScope;

pub fn pack_invs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.inv;

    let files = file_cache.collect("inv");
    debug!("  Found {} .inv files", files.len());

    let configs = parse_config_sections_cached(file_cache, "inv", constants);
    debug!("  Parsed {} inv configs", configs.len());

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown inv id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown inv config: {debugname}");
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "scope" => {
                    server.p1(1);
                    server.p1(InvScope::from_config_str(value) as u8);
                }

                // 2
                "size" => parse_number(value, |v| {
                    server.p1(2);
                    server.p2(v);
                }),

                // 3
                "stackall" => parse_bool(value, |v| {
                    if v {
                        server.p1(3);
                    }
                }),

                // 4
                _ if key.starts_with("stock") => {} // handled at the end

                // 5
                "restock" => parse_bool(value, |v| {
                    if v {
                        server.p1(5);
                    }
                }),

                // 6
                "allstock" => parse_bool(value, |v| {
                    if v {
                        server.p1(6);
                    }
                }),

                // 7
                "protect" => parse_bool(value, |v| {
                    if !v {
                        server.p1(7);
                    }
                }),

                // 8
                "runweight" => parse_bool(value, |v| {
                    if v {
                        server.p1(8);
                    }
                }),

                // 9
                "dummyinv" => parse_bool(value, |v| {
                    if v {
                        server.p1(9);
                    }
                }),

                // not found
                _ => panic!("Unrecognized inv config key: {key}"),
            }
        }

        // handle 4
        let stocks = props
            .iter()
            .filter(|(k, _)| k.starts_with("stock"))
            .map(|(_, v)| v.split(",").map(|v| v.trim()).collect::<Vec<&str>>())
            .collect::<Vec<Vec<&str>>>();

        if !stocks.is_empty() {
            server.p1(4);
            server.p1(stocks.len() as u8);

            for stock in &stocks {
                let obj = stock.first().map(|s| s.trim()).unwrap();
                let count = stock.get(1).map(|s| s.trim()).unwrap();
                let rate = stock.get(2).map(|s| s.trim());

                parse_obj(registry, obj, |v| {
                    server.p2(v);
                    parse_number(count, |v| {
                        server.p2(v);
                    });
                    match rate {
                        Some(r) => parse_number(r, |v| {
                            server.p4(v);
                        }),
                        None => server.p4(0),
                    }
                });
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
