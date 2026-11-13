use super::{client_table, config_entry};
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_bool, parse_colour};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_areas(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.area;

    let configs = parse_config_sections_cached(file_cache, "area", constants)?;

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("area", pack, &configs, id) else {
            server.finish_entry();
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "colour" => {
                    server.p1(1);
                    server.p3(parse_colour(value));
                }

                // 2
                "trigger" => parse_bool(value, |trigger| {
                    if trigger {
                        server.p1(2);
                    }
                }),

                // not found
                _ => panic!("Unrecognized area config key: {key}"),
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
        client: Some(client_table(file_cache, "area", pack, constants)?),
    })
}
