use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{
    parse_bigid, parse_bool, parse_colour, parse_number, parse_numbers, parse_ref,
};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_skyboxes(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "skybox",
        &registry.skybox,
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
                    "op2" => parse_bool(value, |v| {
                        if v {
                            client.p1(2);
                        }
                    }),

                    // 3 and 7
                    "staticmodels" | "op7" => {
                        client.p1(if key == "staticmodels" { 3 } else { 7 });
                        for v in parse_numbers::<i16, 3>(value) {
                            client.p2(v as u16);
                        }
                    }

                    // 4
                    "fillmode" => parse_number(value, |v: u8| {
                        client.p1(4);
                        client.p1(v);
                    }),

                    // 5
                    "staticmodel" => parse_bigid(&registry.model, "model", value, |v| {
                        client.p1(5);
                        client.pbigid(v);
                    }),

                    // 6
                    "op6" => {
                        client.p1(6);
                        client.p3(parse_colour(value));
                    }

                    // not found
                    _ => panic!("Unrecognized skybox config key: {key}"),
                }
            }
        },
    )
}
