use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::parse_number;
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_lights(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "light",
        &registry.light,
        constants,
        |client, _, props| {
            for (key, value) in props {
                match key.as_str() {
                    // 1
                    "function" => parse_number(value, |v: u8| {
                        client.p1(1);
                        client.p1(v);
                    }),

                    // 2
                    "frequency" => parse_number(value, |v: u16| {
                        client.p1(2);
                        client.p2(v);
                    }),

                    // 3
                    "amplitude" => parse_number(value, |v: u16| {
                        client.p1(3);
                        client.p2(v);
                    }),

                    // 4
                    "offset" => parse_number(value, |v: i16| {
                        client.p1(4);
                        client.p2(v as u16);
                    }),

                    // not found
                    _ => panic!("Unrecognized light config key: {key}"),
                }
            }
        },
    )
}
