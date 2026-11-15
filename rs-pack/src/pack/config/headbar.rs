use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{parse_number, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_headbars(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "headbar",
        &registry.headbar,
        constants,
        |client, _, props| {
            for (key, value) in props {
                match key.as_str() {
                    // 1 and 5
                    "unknown1" | "sticktime" => parse_number(value, |v: u16| {
                        client.p1(if key == "unknown1" { 1 } else { 5 });
                        client.p2(v);
                    }),

                    // 2, 3 and 6
                    "showpriority" | "hidepriority" | "unknown6" => parse_number(value, |v: u8| {
                        client.p1(match key.as_str() {
                            "showpriority" => 2,
                            "hidepriority" => 3,
                            _ => 6,
                        });
                        client.p1(v);
                    }),

                    // 4 or 11
                    "fadeout" if value == "no" => client.p1(4),
                    "fadeout" => parse_number(value, |v: u16| {
                        client.p1(11);
                        client.p2(v);
                    }),

                    // 7-10
                    "full" | "empty" | "fullplayergroup" | "emptyplayergroup" => {
                        parse_ref(&registry.sprites, "sprites", value, |v| {
                            client.p1(match key.as_str() {
                                "full" => 7,
                                "empty" => 8,
                                "fullplayergroup" => 9,
                                _ => 10,
                            });
                            client.pid(v);
                        })
                    }

                    // not found
                    _ => panic!("Unrecognized headbar config key: {key}"),
                }
            }
        },
    )
}
