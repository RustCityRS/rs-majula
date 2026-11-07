use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{parse_bool, parse_number, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_billboards(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "billboard",
        &registry.billboard,
        constants,
        |client, _, props| {
            for (key, value) in props {
                match key.as_str() {
                    // 1
                    "material" => parse_ref(&registry.texture, "texture", value, |v| {
                        client.p1(1);
                        client.p2(v);
                    }),

                    // 2
                    "size" => {
                        let (width, height) = value
                            .split_once(',')
                            .unwrap_or_else(|| panic!("Invalid billboard size: {value}"));
                        client.p1(2);
                        parse_number(width.trim(), |v: u16| client.p2(v));
                        parse_number(height.trim(), |v: u16| client.p2(v));
                    }

                    // 3
                    "unknown3" => parse_number(value, |v: u8| {
                        client.p1(3);
                        client.p1(v);
                    }),

                    // 4
                    "sourceblendmode" => parse_number(value, |v: u8| {
                        client.p1(4);
                        client.p1(v);
                    }),

                    // 5
                    "destinationblendmode" => parse_number(value, |v: u8| {
                        client.p1(5);
                        client.p1(v);
                    }),

                    // 6
                    "unknown6" => parse_bool(value, |v| {
                        if v {
                            client.p1(6);
                        }
                    }),

                    // 7
                    "unknown7" => parse_bool(value, |v| {
                        if v {
                            client.p1(7);
                        }
                    }),

                    // not found
                    _ => panic!("Unrecognized billboard config key: {key}"),
                }
            }
        },
    )
}
