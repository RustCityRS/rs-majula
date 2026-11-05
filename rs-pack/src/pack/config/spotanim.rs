use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::RecolType;
#[cfg(before_647)]
use crate::pack::util::colour::rgb15_to_hsl16;
#[cfg(before_647)]
use crate::pack::util::parse_bool;
#[cfg(since_647)]
use crate::pack::util::{pack_pairs, parse_retex};
use crate::pack::util::{parse_model, parse_number, parse_recol, parse_seq};
use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_spotanims(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.spotanim;

    let files = file_cache.collect("spotanim");
    debug!("  Found {} .spotanim files", files.len());

    let configs = parse_config_sections_cached(file_cache, "spotanim", constants);
    debug!("  Parsed {} spotanim configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            #[cfg(before_647)]
            panic!("Unknown spotanim id: {id}");
            #[cfg(since_647)]
            {
                server.skip_entry();
                client.skip_entry();
                continue;
            }
        };

        let Some(props) = configs.get(debugname) else {
            #[cfg(before_647)]
            panic!("Unknown spotanim config: {debugname}");
            #[cfg(since_647)]
            {
                server.skip_entry();
                client.skip_entry();
                continue;
            }
        };

        #[cfg(since_647)]
        let (mut recol_s, mut recol_d, mut retex_s, mut retex_d) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());

        for (key, value) in props {
            match key.as_str() {
                // 1
                "model" => parse_model(registry, value, |v| {
                    client.p1(1);
                    client.p2(v);
                    server.p1(1);
                    server.p2(v);
                }),

                // 2
                "anim" => parse_seq(registry, value, |v| {
                    client.p1(2);
                    client.p2(v);
                    server.p1(2);
                    server.p2(v);
                }),

                // 3
                #[cfg(before_647)]
                "hasalpha" => parse_bool(value, |v| {
                    if v {
                        client.p1(3);
                        server.p1(3);
                    }
                }),

                // 4
                "resizeh" => parse_number(value, |v| {
                    client.p1(4);
                    client.p2(v);
                    server.p1(4);
                    server.p2(v);
                }),

                // 5
                "resizev" => parse_number(value, |v| {
                    client.p1(5);
                    client.p2(v);
                    server.p1(5);
                    server.p2(v);
                }),

                // 6
                "angle" => parse_number(value, |v| {
                    client.p1(6);
                    client.p2(v);
                    server.p1(6);
                    server.p2(v);
                }),

                // 7
                "ambient" => parse_number(value, |v| {
                    client.p1(7);
                    client.p1(v);
                    server.p1(7);
                    server.p1(v);
                }),

                // 8
                "contrast" => parse_number(value, |v| {
                    client.p1(8);
                    client.p1(v);
                    server.p1(8);
                    server.p1(v);
                }),

                #[cfg(since_647)]
                "op9_animmode3_param8224" => {
                    server.p1(9);
                    client.p1(9);
                }

                #[cfg(since_647)]
                "op10_flag" => {
                    server.p1(10);
                    client.p1(10);
                }

                #[cfg(since_647)]
                "op11_animmode1" => {
                    server.p1(11);
                    client.p1(11);
                }

                #[cfg(since_647)]
                "op12_animmode4" => {
                    server.p1(12);
                    client.p1(12);
                }

                #[cfg(since_647)]
                "op13_animmode5" => {
                    server.p1(13);
                    client.p1(13);
                }

                #[cfg(since_647)]
                "op14_animmode2_u8" => {
                    let v: u8 = value.parse().expect("Invalid spotanim value");
                    server.p1(14);
                    server.p1(v);
                    client.p1(14);
                    client.p1(v);
                }

                #[cfg(since_647)]
                "op15_animmode3_u16" => {
                    let v: u16 = value.parse().expect("Invalid spotanim value");
                    server.p1(15);
                    server.p2(v);
                    client.p1(15);
                    client.p2(v);
                }

                #[cfg(since_647)]
                "op16_animmode3_i32" => {
                    let v: i32 = value.parse().expect("Invalid spotanim value");
                    server.p1(16);
                    server.p4(v);
                    client.p1(16);
                    client.p4(v);
                }

                // 40-60
                #[cfg(before_647)]
                _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                    RecolType::S(idx, v) => {
                        client.p1((39 + idx) as u8);
                        client.p2(rgb15_to_hsl16(v));
                        server.p1((39 + idx) as u8);
                        server.p2(rgb15_to_hsl16(v));
                    }
                    RecolType::D(idx, v) => {
                        client.p1((49 + idx) as u8);
                        client.p2(rgb15_to_hsl16(v));
                        server.p1((49 + idx) as u8);
                        server.p2(rgb15_to_hsl16(v));
                    }
                }),

                #[cfg(since_647)]
                _ if key.starts_with("recol") => parse_recol(key, value, |value| match value {
                    RecolType::S(index, value) => recol_s.push((index, value)),
                    RecolType::D(index, value) => recol_d.push((index, value)),
                }),

                #[cfg(since_647)]
                _ if key.starts_with("retex") => {
                    parse_retex(registry, key, value, |value| match value {
                        RecolType::S(index, value) => retex_s.push((index, value)),
                        RecolType::D(index, value) => retex_d.push((index, value)),
                    })
                }

                // not found
                _ => panic!("Unrecognized spotanim config key: {key}"),
            }
        }

        #[cfg(since_647)]
        {
            recol_s.sort_by_key(|(index, _)| *index);
            recol_d.sort_by_key(|(index, _)| *index);
            retex_s.sort_by_key(|(index, _)| *index);
            retex_d.sort_by_key(|(index, _)| *index);
            for stream in [&mut server, &mut client] {
                pack_pairs(stream, 40, &recol_d, &recol_s);
                pack_pairs(stream, 41, &retex_d, &retex_s);
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        client.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
