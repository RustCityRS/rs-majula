use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{parse_number, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_hitmarks(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let sprites = &registry.sprites;
    client_config_table(
        file_cache,
        "hitmark",
        &registry.hitmark,
        constants,
        |client, _, props| {
            for (key, value) in props {
                match key.as_str() {
                    // 1
                    "damagefont" => parse_ref(&registry.fontmetrics, "fontmetrics", value, |v| {
                        client.p1(1);
                        client.p2(v);
                    }),

                    // 2
                    "damagecolour" => parse_number(value, |v: i32| {
                        client.p1(2);
                        client.p3(v);
                    }),

                    // 3
                    "classgraphic" => parse_ref(sprites, "sprites", value, |v| {
                        client.p1(3);
                        client.p2(v);
                    }),

                    // 4
                    "leftgraphic" => parse_ref(sprites, "sprites", value, |v| {
                        client.p1(4);
                        client.p2(v);
                    }),

                    // 5
                    "middlegraphic" => parse_ref(sprites, "sprites", value, |v| {
                        client.p1(5);
                        client.p2(v);
                    }),

                    // 6
                    "rightgraphic" => parse_ref(sprites, "sprites", value, |v| {
                        client.p1(6);
                        client.p2(v);
                    }),

                    // 7
                    "scrolltooffsetx" => parse_number(value, |v: i16| {
                        client.p1(7);
                        client.p2(v as u16);
                    }),

                    // 8
                    "damageformat" => {
                        client.p1(8);
                        client.p1(0);
                        client.pjstr(value);
                    }

                    // 9
                    "sticktime" => parse_number(value, |v: u16| {
                        client.p1(9);
                        client.p2(v);
                    }),

                    // 10
                    "scrolltooffsety" => parse_number(value, |v: i16| {
                        client.p1(10);
                        client.p2(v as u16);
                    }),

                    // 11
                    "fadeout" if value == "no" => client.p1(11),

                    // 12
                    "replacemode" => parse_number(value, |v: u8| {
                        client.p1(12);
                        client.p1(v);
                    }),

                    // 13
                    "damageyof" => parse_number(value, |v: i16| {
                        client.p1(13);
                        client.p2(v as u16);
                    }),

                    // 14
                    "fadeout" => parse_number(value, |v: u16| {
                        client.p1(14);
                        client.p2(v);
                    }),

                    // not found
                    _ => panic!("Unrecognized hitmark config key: {key}"),
                }
            }
        },
    )
}
