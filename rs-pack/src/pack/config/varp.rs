use std::collections::HashMap;

use crate::config_crc;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_bool, parse_number, parse_script_var_type};
use crate::types::VarPlayerScope;
use anyhow::Result;
use rs_io::crc;
use tracing::debug;

pub fn pack_varps(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.varp;

    let files = file_cache.collect("varp");
    debug!("  Found {} .varp files", files.len());

    let configs = parse_config_sections_cached(file_cache, "varp", constants);
    debug!("  Parsed {} varp configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown varp id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown varp config: {debugname}");
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
                    client.p1(5);
                    client.p2(v);
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

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        client.finish_entry();
    }

    if verify {
        let crc = crc::getcrc(&client.dat, 0, client.dat.len());
        let expected = config_crc::VARP;
        if crc != expected {
            panic!("CRC mismatch ['varp']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
