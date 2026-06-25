use crate::config_crc;
use crate::pack::config::param::parse_params;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::{RecolType, rgb15_to_hsl16};
use crate::pack::util::parse_coord;
use crate::pack::util::*;
use crate::types::{BlockWalk, MoveRestrict, NpcMode};
use anyhow::Result;
use rs_io::crc;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_npcs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.npc;

    let files = file_cache.collect("npc");
    debug!("  Found {} .npc files", files.len());

    let configs = parse_config_sections_cached(file_cache, "npc", constants);
    debug!("  Parsed {} npc configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown npc id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown npc config: {debugname}");
        };

        // Collected fields written after the loop
        let mut models: Vec<(usize, u16)> = Vec::new();
        let mut head_models: Vec<(usize, u16)> = Vec::new();
        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        let mut recol_d: Vec<(usize, u16)> = Vec::new();
        let mut patrols: Vec<(i32, u8)> = Vec::new();
        let mut has_vislevel = false;

        for (key, value) in props {
            match key.as_str() {
                // 1
                _ if key.starts_with("model") => {
                    parse_model_kind(registry, "model", key, value, |v| models.push((v.0, v.1)))
                }

                // 2
                "name" => {} // handled at the end

                // 3
                "desc" => {
                    client.p1(3);
                    client.pjstr(value);
                    server.p1(3);
                    server.pjstr(value);
                }

                // 12
                "size" => parse_number(value, |v| {
                    client.p1(12);
                    client.p1(v);
                    server.p1(12);
                    server.p1(v);
                }),

                // 13
                "readyanim" => parse_seq(registry, value, |v| {
                    client.p1(13);
                    client.p2(v);
                    server.p1(13);
                    server.p2(v);
                }),

                // 14 or 17
                "walkanim" => {
                    if !value.contains(',') {
                        // 14
                        parse_seq(registry, value, |v| {
                            client.p1(14);
                            client.p2(v);
                            server.p1(14);
                            server.p2(v);
                        });
                    } else {
                        // 17
                        let anims: Vec<&str> = value.split(',').collect();
                        if anims.len() == 4 {
                            let ids: Vec<u16> = anims
                                .iter()
                                .map(|a| {
                                    let mut result = 0u16;
                                    parse_seq(registry, a, |val| result = val);
                                    result
                                })
                                .collect();
                            client.p1(17);
                            server.p1(17);
                            for &id in &ids {
                                client.p2(id);
                                server.p2(id);
                            }
                        }
                    }
                }

                // 16
                "hasalpha" => parse_bool(value, |v| {
                    if v {
                        client.p1(16);
                        server.p1(16);
                    }
                }),

                // 18
                "category" => parse_category(registry, value, |v| {
                    server.p1(18);
                    server.p2(v);
                }),

                // 30-39
                "op1" | "op2" | "op3" | "op4" | "op5" => parse_number(&key[2..], |v: u8| {
                    client.p1(29 + v);
                    client.pjstr(value);
                    server.p1(29 + v);
                    server.pjstr(value);
                }),

                // 40
                _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                    RecolType::S(idx, v) => recol_s.push((idx, v)),
                    RecolType::D(idx, v) => recol_d.push((idx, v)),
                }),

                // 60
                _ if key.starts_with("head") => {
                    parse_model_kind(registry, "head", key, value, |v| {
                        head_models.push((v.0, v.1))
                    })
                }

                // 74
                "attack" => parse_number(value, |v| {
                    server.p1(74);
                    server.p2(v);
                }),

                // 75
                "defence" => parse_number(value, |v| {
                    server.p1(75);
                    server.p2(v);
                }),

                // 76
                "strength" => parse_number(value, |v| {
                    server.p1(76);
                    server.p2(v);
                }),

                // 77
                "hitpoints" => parse_number(value, |v| {
                    server.p1(77);
                    server.p2(v);
                }),

                // 78
                "ranged" => parse_number(value, |v| {
                    server.p1(78);
                    server.p2(v);
                }),

                // 79
                "magic" => parse_number(value, |v| {
                    server.p1(79);
                    server.p2(v);
                }),

                // 90
                "resizex" => parse_number(value, |v| {
                    client.p1(90);
                    client.p2(v);
                    server.p1(90);
                    server.p2(v);
                }),

                // 91
                "resizey" => parse_number(value, |v| {
                    client.p1(91);
                    client.p2(v);
                    server.p1(91);
                    server.p2(v);
                }),

                // 92
                "resizez" => parse_number(value, |v| {
                    client.p1(92);
                    client.p2(v);
                    server.p1(92);
                    server.p2(v);
                }),

                // 93
                "minimap" => parse_bool(value, |v| {
                    if !v {
                        client.p1(93);
                        server.p1(93);
                    }
                }),

                // 95
                "vislevel" => {
                    let val = if value == "hide" {
                        0u16
                    } else {
                        let mut result = 0u16;
                        parse_number(value, |v| result = v);
                        result
                    };
                    client.p1(95);
                    client.p2(val);
                    server.p1(95);
                    server.p2(val);
                    has_vislevel = true;
                }

                // 97
                "resizeh" => parse_number(value, |v| {
                    client.p1(97);
                    client.p2(v);
                    server.p1(97);
                    server.p2(v);
                }),

                // 98
                "resizev" => parse_number(value, |v| {
                    client.p1(98);
                    client.p2(v);
                    server.p1(98);
                    server.p2(v);
                }),

                // 99
                #[cfg(since_244)]
                "alwaysontop" => parse_bool(value, |v| {
                    if v {
                        client.p1(99);
                        server.p1(99);
                    }
                }),

                // 100
                #[cfg(since_244)]
                "ambient" => parse_number(value, |v: i8| {
                    client.p1(100);
                    client.p1(v as u8);
                    server.p1(100);
                    server.p1(v as u8);
                }),

                // 101
                #[cfg(since_244)]
                "contrast" => parse_number(value, |v: i8| {
                    client.p1(101);
                    client.p1(v as u8);
                    server.p1(101);
                    server.p1(v as u8);
                }),

                // 102
                #[cfg(since_244)]
                "headicon" => parse_number(value, |v| {
                    client.p1(102);
                    client.p2(v);
                    server.p1(102);
                    server.p2(v);
                }),

                // 200
                "wanderrange" => parse_number(value, |v| {
                    server.p1(200);
                    server.p2(v);
                }),

                // 201
                "maxrange" => parse_number(value, |v| {
                    server.p1(201);
                    server.p2(v);
                }),

                // 202
                "huntrange" => parse_number(value, |v| {
                    server.p1(202);
                    server.p1(v);
                }),

                // 203
                "timer" => parse_number(value, |v| {
                    server.p1(203);
                    server.p2(v);
                }),

                // 204
                "respawnrate" => parse_number(value, |v| {
                    server.p1(204);
                    server.p2(v);
                }),

                // 206
                "moverestrict" => {
                    let val = MoveRestrict::from_config_str(value);
                    server.p1(206);
                    server.p1(val as u8);
                }

                // 207
                "attackrange" => parse_number(value, |v| {
                    server.p1(207);
                    server.p2(v);
                }),

                // 208
                "blockwalk" => {
                    let val = BlockWalk::from_config_str(value);
                    server.p1(208);
                    server.p1(val as u8);
                }

                // 209
                "huntmode" => parse_hunt(registry, value, |v| {
                    server.p1(209);
                    server.p1(v as u8);
                }),

                // 210
                "defaultmode" => {
                    let val = NpcMode::from_config_str(value);
                    server.p1(210);
                    server.p1(val as u8);
                }

                // 211
                "members" => parse_bool(value, |v| {
                    if v {
                        server.p1(211);
                    }
                }),

                // 212
                _ if key.starts_with("patrol") => {
                    if let Some((coord_str, delay_str)) = value.split_once(',')
                        && let (Some(coord), Ok(delay)) =
                            (parse_coord(coord_str), delay_str.parse::<u8>())
                    {
                        patrols.push((coord, delay));
                    }
                }

                // 213
                "givechase" => parse_bool(value, |v| {
                    if !v {
                        server.p1(213);
                    }
                }),

                // 214
                "regenrate" => parse_number(value, |v| {
                    server.p1(214);
                    server.p2(v);
                }),

                // 249
                "param" => {} // handled at the end

                // not found
                _ => panic!("Unrecognized npc config key: {key}"),
            }
        }

        // handle 40
        if !recol_s.is_empty() {
            let count = recol_s.len();
            client.p1(40);
            client.p1(count as u8);
            server.p1(40);
            server.p1(count as u8);
            for i in 0..count {
                let s = recol_s[i].1;
                let d = recol_d[i].1;
                if s >= 100 || d >= 100 {
                    client.p2(rgb15_to_hsl16(s));
                    client.p2(rgb15_to_hsl16(d));
                    server.p2(rgb15_to_hsl16(s));
                    server.p2(rgb15_to_hsl16(d));
                } else {
                    client.p2(s);
                    client.p2(d);
                    server.p2(s);
                    server.p2(d);
                }
            }
        }

        // handle 2
        let npc_name = props
            .iter()
            .find(|(k, _)| k == "name")
            .map(|(_, v)| v.as_str())
            .unwrap_or(debugname);
        client.p1(2);
        client.pjstr(npc_name);
        server.p1(2);
        server.pjstr(npc_name);

        // models (opcode 1)
        if !models.is_empty() {
            client.p1(1);
            client.p1(models.len() as u8);
            server.p1(1);
            server.p1(models.len() as u8);
            for (_, v) in &models {
                client.p2(*v);
                server.p2(*v);
            }
        }

        // handle 60
        if !head_models.is_empty() {
            client.p1(60);
            client.p1(head_models.len() as u8);
            server.p1(60);
            server.p1(head_models.len() as u8);
            for (_, v) in &head_models {
                client.p2(*v);
                server.p2(*v);
            }
        }

        // default vislevel=1 if not set (opcode 95)
        if !has_vislevel {
            client.p1(95);
            client.p2(1);
            server.p1(95);
            server.p2(1);
        }

        // handle 212
        if !patrols.is_empty() {
            server.p1(212);
            server.p1(patrols.len() as u8);
            for (coord, delay) in &patrols {
                server.p4(*coord);
                server.p1(*delay);
            }
        }

        // handle 249
        parse_params(registry, param_types, &mut server, props, debugname);

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        client.finish_entry();
    }

    if verify {
        let crc = crc::getcrc(&client.dat, 0, client.dat.len());
        let expected = config_crc::NPC;
        if crc != expected {
            panic!("CRC mismatch ['npc']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
