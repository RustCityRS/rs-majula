use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{parse_numbers, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_cursors(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "cursor",
        &registry.cursor,
        constants,
        |client, _, props| {
            for (key, value) in props {
                match key.as_str() {
                    // 1
                    "graphic" => parse_ref(&registry.sprites, "sprites", value, |v| {
                        client.p1(1);
                        client.p2(v);
                    }),

                    // 2
                    "hotspot" => {
                        let [x, y] = parse_numbers::<u8, 2>(value);
                        client.p1(2);
                        client.p1(x);
                        client.p1(y);
                    }

                    // not found
                    _ => panic!("Unrecognized cursor config key: {key}"),
                }
            }
        },
    )
}
