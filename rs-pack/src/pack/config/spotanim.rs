use crate::config_crc;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::{RecolType, rgb15_to_hsl16};
use crate::pack::util::{parse_bool, parse_model, parse_number, parse_recol, parse_seq};
use anyhow::Result;
use rs_io::crc;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_spotanims(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.spotanim;

    let files = file_cache.collect("spotanim");
    debug!("  Found {} .spotanim files", files.len());

    let configs = parse_config_sections_cached(file_cache, "spotanim", constants);
    debug!("  Parsed {} spotanim configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown spotanim id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown spotanim config: {debugname}");
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "model" => parse_model(registry, value, |v| {
                    client.p1(1);
                    client.p2(v);
                    server.p1(1);
                    server.p2(v);
                }),

                // 2
                "anim" => parse_seq(registry, value, |v| {
                    client.p1(2);
                    client.p2(v);
                    server.p1(2);
                    server.p2(v);
                }),

                // 3
                "hasalpha" => parse_bool(value, |v| {
                    if v {
                        client.p1(3);
                        server.p1(3);
                    }
                }),

                // 4
                "resizeh" => parse_number(value, |v| {
                    client.p1(4);
                    client.p2(v);
                    server.p1(4);
                    server.p2(v);
                }),

                // 5
                "resizev" => parse_number(value, |v| {
                    client.p1(5);
                    client.p2(v);
                    server.p1(5);
                    server.p2(v);
                }),

                // 6
                "angle" => parse_number(value, |v| {
                    client.p1(6);
                    client.p2(v);
                    server.p1(6);
                    server.p2(v);
                }),

                // 7
                "ambient" => parse_number(value, |v| {
                    client.p1(7);
                    client.p1(v);
                    server.p1(7);
                    server.p1(v);
                }),

                // 8
                "contrast" => parse_number(value, |v| {
                    client.p1(8);
                    client.p1(v);
                    server.p1(8);
                    server.p1(v);
                }),

                // 40-60
                _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                    RecolType::S(idx, v) => {
                        client.p1((39 + idx) as u8);
                        client.p2(rgb15_to_hsl16(v));
                        server.p1((39 + idx) as u8);
                        server.p2(rgb15_to_hsl16(v));
                    }
                    RecolType::D(idx, v) => {
                        client.p1((49 + idx) as u8);
                        client.p2(rgb15_to_hsl16(v));
                        server.p1((49 + idx) as u8);
                        server.p2(rgb15_to_hsl16(v));
                    }
                }),

                // not found
                _ => panic!("Unrecognized spotanim config key: {key}"),
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
        let expected = config_crc::SPOTANIM;
        if crc != expected {
            panic!("CRC mismatch ['spotanim']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
