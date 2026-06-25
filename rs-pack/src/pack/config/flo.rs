use crate::config_crc;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_bool, parse_hex, parse_texture};
use anyhow::Result;
use rs_io::crc;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_flos(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.flo;

    let files = file_cache.collect("flo");
    debug!("  Found {} .flo files", files.len());

    let configs = parse_config_sections_cached(file_cache, "flo", constants);
    debug!("  Parsed {} flo configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown flo id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown flo config: {debugname}");
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "colour" => parse_hex(value, |v| {
                    client.p1(1);
                    client.p3(v);
                    server.p1(1);
                    server.p3(v);
                }),

                // 2
                "texture" => parse_texture(registry, value, |v| {
                    client.p1(2);
                    client.p1(v as u8);
                    server.p1(2);
                    server.p1(v as u8);
                }),

                // 3
                "overlay" => parse_bool(value, |v| {
                    if v {
                        client.p1(3);
                        server.p1(3);
                    }
                }),

                // 5
                "occlude" => parse_bool(value, |v| {
                    if !v {
                        client.p1(5);
                        server.p1(5);
                    }
                }),

                // not found
                _ => panic!("Unrecognized flo config key: {key}"),
            }
        }

        // 6
        if !debugname.starts_with("flo_") {
            // yes, this was originally transmitted!
            client.p1(6);
            client.pjstr(debugname);
            server.p1(6);
            server.pjstr(debugname);
        }

        // done
        server.finish_entry();
        client.finish_entry();
    }

    if verify {
        let crc = crc::getcrc(&client.dat, 0, client.dat.len());
        let expected = config_crc::FLO;
        if crc != expected {
            panic!("CRC mismatch ['flo']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
