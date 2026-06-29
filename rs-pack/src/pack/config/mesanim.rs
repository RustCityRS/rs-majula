#[cfg(since_274)]
use crate::config_crc;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_seq;
use anyhow::Result;
#[cfg(since_274)]
use rs_io::crc;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_mesanims(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    #[cfg_attr(before_274, allow(unused_variables))] verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.mesanim;

    let files = file_cache.collect("mesanim");
    debug!("  Found {} .mesanim files", files.len());

    let configs = parse_config_sections_cached(file_cache, "mesanim", constants);
    debug!("  Parsed {} mesanim configs", configs.len());

    let mut server = PackedData::new(pack.max);
    #[cfg(since_274)]
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        #[cfg(since_274)]
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown mesanim id: {}", id);
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown mesanim config: {}", debugname);
        };

        for (key, value) in props {
            match key.as_str() {
                // 1-4
                _ if key.starts_with("len") => parse_seq(registry, value, |v| {
                    if let Some(rest) = key.strip_prefix("len")
                        && let Ok(idx) = rest.parse::<u8>()
                    {
                        server.p1(idx);
                        server.p2(v);
                    }
                }),

                // not found
                _ => panic!("Unrecognized mesanim config key: {key}"),
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        #[cfg(since_274)]
        client.finish_entry();
    }

    #[cfg(before_274)]
    let client = None;

    #[cfg(since_274)]
    let client = {
        if verify {
            let crc = crc::getcrc(&client.dat, 0, client.dat.len());
            let expected = config_crc::MESANIM;
            if crc != expected {
                panic!("CRC mismatch ['mesanim']: Got: {crc}, Expected: {expected}");
            }
        }
        Some(client)
    };

    Ok(PackedFile { server, client })
}
