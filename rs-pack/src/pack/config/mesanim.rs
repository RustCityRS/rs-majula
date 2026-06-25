use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_seq;
use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_mesanims(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.mesanim;

    let files = file_cache.collect("mesanim");
    debug!("  Found {} .mesanim files", files.len());

    let configs = parse_config_sections_cached(file_cache, "mesanim", constants);
    debug!("  Parsed {} mesanim configs", configs.len());

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

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
    }

    Ok(PackedFile {
        server,
        client: None,
    })
}
