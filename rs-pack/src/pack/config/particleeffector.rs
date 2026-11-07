use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{parse_bool, parse_number};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_particleeffectors(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "particleeffector",
        &registry.particleeffector,
        constants,
        |client, _, props| {
            for (key, value) in props {
                match key.as_str() {
                    // 1
                    "angle" => parse_number(value, |v: u16| {
                        client.p1(1);
                        client.p2(v);
                    }),

                    // 2
                    "unknown2" => parse_number(value, |v: u8| {
                        client.p1(2);
                        client.p1(v);
                    }),

                    // 3
                    "forcedirection" => {
                        client.p1(3);
                        for part in value.split(',') {
                            parse_number(part.trim(), |v: i32| client.p4(v));
                        }
                    }

                    // 4
                    "falloff" => {
                        let (kind, falloff) = value
                            .split_once(',')
                            .unwrap_or_else(|| panic!("Invalid falloff: {value}"));
                        client.p1(4);
                        parse_number(kind.trim(), |v: u8| client.p1(v));
                        parse_number(falloff.trim(), |v: i32| client.p4(v));
                    }

                    // 6
                    "forcetype" => parse_number(value, |v: u8| {
                        client.p1(6);
                        client.p1(v);
                    }),

                    // 8
                    "unknown8" => parse_bool(value, |v| {
                        if v {
                            client.p1(8);
                        }
                    }),

                    // 9
                    "unknown9" => parse_bool(value, |v| {
                        if v {
                            client.p1(9);
                        }
                    }),

                    // 10
                    "unknown10" => parse_bool(value, |v| {
                        if v {
                            client.p1(10);
                        }
                    }),

                    // not found
                    _ => panic!("Unrecognized effector config key: {key}"),
                }
            }
        },
    )
}
