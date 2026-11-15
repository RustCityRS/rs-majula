use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{parse_bigid, parse_number, parse_ref};
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
                    "op2" => {
                        let ids: Vec<u16> = value
                            .split(',')
                            .map(str::trim)
                            .filter(|v| !v.is_empty())
                            .map(|v| {
                                v.parse()
                                    .unwrap_or_else(|_| panic!("Invalid skybox op2 id: {v}"))
                            })
                            .collect();
                        let count = u8::try_from(ids.len())
                            .unwrap_or_else(|_| panic!("Too many skybox op2 ids: {value}"));
                        client.p1(2);
                        client.p1(count);
                        for id in ids {
                            client.p2(id);
                        }
                    }

                    // 3
                    "op3" => parse_number(value, |v: u8| {
                        client.p1(3);
                        client.p1(v);
                    }),

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

                    // not found
                    _ => panic!("Unrecognized skybox config key: {key}"),
                }
            }
        },
    )
}
