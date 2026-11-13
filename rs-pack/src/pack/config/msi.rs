use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{parse_colour, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_msis(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "msi",
        &registry.msi,
        constants,
        |client, _, props| {
            for (key, value) in props {
                match key.as_str() {
                    // 1
                    "graphic" if value == "null" => client.p1(4),
                    "graphic" => parse_ref(&registry.sprites, "sprites", value, |v| {
                        client.p1(1);
                        client.p2(v);
                    }),

                    // 2
                    "colour" => {
                        client.p1(2);
                        client.p3(parse_colour(value));
                    }

                    // 3
                    "resize" => client.p1(3),

                    // not found
                    _ => panic!("Unrecognized msi config key: {key}"),
                }
            }
        },
    )
}
