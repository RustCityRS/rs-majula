#[cfg(since_647)]
use super::client_table;
use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
#[cfg(all(since_274, before_647))]
use crate::pack::util::load_order;
use crate::pack::util::parse_ref;
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_mesanims(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.mesanim;

    let configs = parse_config_sections_cached(file_cache, "mesanim", constants)?;

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("mesanim", pack, &configs, id) else {
            server.finish_entry();
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                // 1-4
                _ if key.starts_with("len") => parse_ref(&registry.seq, "seq", value, |v| {
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

    #[cfg(before_274)]
    let client = None;

    #[cfg(all(since_274, before_647))]
    let client = {
        let ids = load_order(&registry.pack_dir().join("mesanim.order"));
        let count = ids.iter().max().map_or(0, |id| id + 1);
        let mut client = PackedData::new(count);
        for id in 0..count {
            if ids.contains(&id) {
                client.start_entry();
                client.finish_entry();
            } else {
                client.skip_entry();
            }
        }
        Some(client)
    };

    #[cfg(since_647)]
    let client = Some(client_table(file_cache, "mesanim", pack, constants)?);

    Ok(PackedFile { server, client })
}
