use super::client_config_table;
use crate::pack::config::param::parse_params;
use crate::pack::pack::{ConfigProperties, FileCache};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_colour, parse_number, parse_numbers, parse_ref, placeholder_prefix};
use anyhow::Result;
use std::collections::HashMap;

/// One var, a varbit or a varp, and the range it must be in.
fn write_condition(client: &mut PackedData, registry: &PackRegistry, value: &str) {
    let (var, range) = value.split_once(',').expect("Missing condition range");
    let [min, max] = parse_numbers::<i32, 2>(range);
    if var == "null" {
        client.p2(u16::MAX);
        client.p2(u16::MAX);
    } else if registry.varbit.get_by_debugname(var).is_some()
        || var.starts_with(placeholder_prefix("varbit"))
    {
        parse_ref(&registry.varbit, "varbit", var, |v| client.p2(v));
        client.p2(u16::MAX);
    } else {
        client.p2(u16::MAX);
        parse_ref(&registry.varp, "varp", var, |v| client.p2(v));
    }
    client.p4(min);
    client.p4(max);
}

fn write_polygon(client: &mut PackedData, props: &ConfigProperties) {
    let find = |key: &str| {
        props
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let points: Vec<[i16; 2]> = props
        .iter()
        .filter(|(k, _)| k.starts_with("polygonpoint"))
        .map(|(_, v)| parse_numbers::<i16, 2>(v))
        .collect();
    let mut palette = Vec::new();
    let mut outline = Vec::new();
    if let Some(colour) = find("polygonoutline") {
        palette.push(parse_colour(colour));
        outline.resize(points.len(), 0);
    } else {
        for index in 0..points.len() {
            let colour = find(&format!("polygonoutline{index}")).expect("Missing polygon outline");
            let colour = parse_colour(colour);
            let slot = palette
                .iter()
                .position(|c| *c == colour)
                .unwrap_or_else(|| {
                    palette.push(colour);
                    palette.len() - 1
                });
            outline.push(slot as u8);
        }
    }
    client.p1(15);
    client.p1(points.len() as u8);
    for [x, y] in points {
        client.p2(x as u16);
        client.p2(y as u16);
    }
    client.p4(parse_colour(
        find("polygonfill").expect("Missing polygon fill"),
    ));
    client.p1(palette.len() as u8);
    for colour in palette {
        client.p4(colour);
    }
    for slot in outline {
        client.p1(slot);
    }
}

pub fn pack_mels(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
) -> Result<PackedFile> {
    let sprites = &registry.sprites;
    client_config_table(
        file_cache,
        "mel",
        &registry.mel,
        constants,
        |client, name, props| {
            let mut wrote_params = false;
            for (key, value) in props {
                match key.as_str() {
                    // 1
                    "sprite" => parse_ref(sprites, "sprites", value, |v| {
                        client.p1(1);
                        client.p2(v);
                    }),

                    // 2
                    "mouseovergraphic" => parse_ref(sprites, "sprites", value, |v| {
                        client.p1(2);
                        client.p2(v);
                    }),

                    // 3
                    "text" => {
                        client.p1(3);
                        client.pjstr(value);
                    }

                    // 4
                    "textcolour" => {
                        client.p1(4);
                        client.p3(parse_colour(value));
                    }

                    // 5
                    "textmouseovercolour" => {
                        client.p1(5);
                        client.p3(parse_colour(value));
                    }

                    // 6
                    "textsize" => parse_number(value, |v: u8| {
                        client.p1(6);
                        client.p1(v);
                    }),

                    // 7
                    "show" => {
                        client.p1(7);
                        client.p1(match value.as_str() {
                            "none" => 0,
                            "map" => 1,
                            "minimap" => 2,
                            "both" => 3,
                            _ => panic!("Invalid mel show value: {value}"),
                        });
                    }

                    // 8
                    "mapfunction" => {
                        client.p1(8);
                        client.p1(match value.as_str() {
                            "no" => 0,
                            "yes" => 1,
                            _ => panic!("Invalid mel mapfunction value: {value}"),
                        });
                    }

                    // 9
                    "condition" => {
                        client.p1(9);
                        write_condition(client, registry, value);
                    }

                    // 10-14
                    "op1" | "op2" | "op3" | "op4" | "op5" => {
                        let index: u8 = key[2..].parse().expect("Invalid mel op");
                        client.p1(9 + index);
                        client.pjstr(value);
                    }

                    // 15
                    "polygonpoint0" => write_polygon(client, props),
                    "polygonoutlinedash" => {
                        // 23
                        let [a, b, c] = parse_numbers::<u8, 3>(value);
                        client.p1(23);
                        client.p1(a);
                        client.p1(b);
                        client.p1(c);
                    }
                    _ if key.starts_with("polygon") => {}

                    // 16
                    "listable" => client.p1(16),

                    // 17
                    "opbase" => {
                        client.p1(17);
                        client.pjstr(value);
                    }

                    // 18
                    "worldmaparrow" => parse_ref(sprites, "sprites", value, |v| {
                        client.p1(18);
                        client.p2(v);
                    }),

                    // 19
                    "category" => parse_number(
                        value.strip_prefix("category_").unwrap_or(value),
                        |v: u16| {
                            client.p1(19);
                            client.p2(v);
                        },
                    ),

                    // 20
                    "condition2" => {
                        client.p1(20);
                        write_condition(client, registry, value);
                    }

                    // 21
                    "textbackgroundoutline" => {
                        client.p1(21);
                        client.p4(parse_colour(value));
                    }

                    // 22
                    "textbackgroundfill" => {
                        client.p1(22);
                        client.p4(parse_colour(value));
                    }

                    // 24
                    "textoffset" => {
                        let [x, y] = parse_numbers::<i16, 2>(value);
                        client.p1(24);
                        client.p2(x as u16);
                        client.p2(y as u16);
                    }

                    // 249
                    "param" => {
                        if !wrote_params {
                            wrote_params = true;
                            parse_params(registry, param_types, client, props, name);
                        }
                    }

                    // not found
                    _ => panic!("Unrecognized mel config key: {key}"),
                }
            }
        },
    )
}
