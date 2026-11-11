use super::config_entry;
#[cfg(since_647)]
use super::mark_client_params;
use crate::pack::config::param::parse_params;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
#[cfg(before_647)]
use crate::pack::util::colour::{RecolType, rgb15_to_hsl16};
use crate::pack::util::parse_coord;
use crate::pack::util::*;
use crate::types::{BlockWalk, MoveRestrict, NpcMode};
use anyhow::{Context, Result};
use std::collections::HashMap;

pub fn pack_npcs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.npc;

    let configs = parse_config_sections_cached(file_cache, "npc", constants)?;

    #[cfg(since_647)]
    let client_params = registry.param.client_count();

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some((debugname, props)) = config_entry("npc", pack, &configs, id) else {
            server.skip_entry();
            client.skip_entry();
            continue;
        };

        #[cfg(since_647)]
        let props = &mark_client_params(props, &registry.param, client_params);

        // Collected fields written after the loop
        #[cfg(before_647)]
        let mut models: Vec<(usize, u16)> = Vec::new();
        #[cfg(before_647)]
        let mut head_models: Vec<(usize, u16)> = Vec::new();
        #[cfg(before_647)]
        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        #[cfg(before_647)]
        let mut recol_d: Vec<(usize, u16)> = Vec::new();
        let mut patrols: Vec<(i32, u8)> = Vec::new();
        #[cfg(before_647)]
        let mut has_vislevel = false;

        for group in props.chunk_by(|a, b| batch_code(&a.0) == batch_code(&b.0)) {
            #[cfg(since_647)]
            if batch_code(&group[0].0) != 0 {
                let start = client.dat.len();
                pack_batch(&mut client, group, registry, param_types, debugname)?;
                server.dat.extend_from_slice(&client.dat[start..]);
                continue;
            }
            for (key, value) in group {
                let start = client.dat.len();
                match key.as_str() {
                    // 1
                    #[cfg(before_647)]
                    _ if key.starts_with("model") => {
                        parse_model_kind(registry, "model", key, value, |v| models.push((v.0, v.1)))
                    }

                    // 2
                    "name" => {
                        #[cfg(since_647)]
                        {
                            client.p1(2);
                            client.pjstr(value);
                        }
                    }

                    // 3
                    "desc" => {
                        #[cfg(before_647)]
                        {
                            client.p1(3);
                            client.pjstr(value);
                        }
                        #[cfg(since_647)]
                        {
                            server.p1(3);
                            server.pjstr(value);
                        }
                    }

                    // 12
                    "size" => parse_number(value, |v| {
                        client.p1(12);
                        client.p1(v);
                    }),

                    // 13
                    #[cfg(before_647)]
                    "readyanim" => parse_ref(&registry.seq, "seq", value, |v| {
                        client.p1(13);
                        client.p2(v);
                    }),

                    // 14 or 17
                    #[cfg(before_647)]
                    "walkanim" => {
                        if !value.contains(',') {
                            // 14
                            parse_ref(&registry.seq, "seq", value, |v| {
                                client.p1(14);
                                client.p2(v);
                            });
                        } else {
                            // 17
                            let anims: Vec<&str> = value.split(',').collect();
                            if anims.len() == 4 {
                                let ids: Vec<u16> = anims
                                    .iter()
                                    .map(|a| {
                                        let mut result = 0u16;
                                        parse_ref(&registry.seq, "seq", a, |val| result = val);
                                        result
                                    })
                                    .collect();
                                client.p1(17);
                                for &id in &ids {
                                    client.p2(id);
                                }
                            }
                        }
                    }

                    // 16
                    #[cfg(before_254)]
                    "hasalpha" => parse_bool(value, |v| {
                        if v {
                            client.p1(16);
                        }
                    }),

                    // 18
                    "category" => parse_ref(&registry.category, "category", value, |v| {
                        server.p1(18);
                        server.p2(v);
                    }),

                    // 19
                    "timer" => parse_number(value, |v| {
                        server.p1(19);
                        server.p2(v);
                    }),

                    // 20
                    "respawnrate" => parse_number(value, |v| {
                        server.p1(20);
                        server.p2(v);
                    }),

                    // 21
                    "regenrate" => parse_number(value, |v| {
                        server.p1(21);
                        server.p2(v);
                    }),

                    // 22
                    "moverestrict" => {
                        let val = MoveRestrict::from_config_str(value);
                        server.p1(22);
                        server.p1(val as u8);
                    }

                    // 23
                    "blockwalk" => {
                        let val = BlockWalk::from_config_str(value);
                        server.p1(23);
                        server.p1(val as u8);
                    }

                    // 24
                    "defaultmode" => {
                        let val = NpcMode::from_config_str(value);
                        server.p1(24);
                        server.p1(val as u8);
                    }

                    // 25
                    "huntmode" => {
                        let hunt = registry
                            .hunt
                            .get_by_debugname(value)
                            .or_else(|| value.parse().ok())
                            .with_context(|| format!("{debugname}: unknown huntmode '{value}'"))?;
                        let hunt = u8::try_from(hunt).with_context(|| {
                            format!("{debugname}: huntmode '{value}' exceeds 255")
                        })?;
                        server.p1(25);
                        server.p1(hunt);
                    }

                    // 26
                    "wanderrange" => parse_number(value, |v| {
                        server.p1(26);
                        server.p2(v);
                    }),

                    // 27
                    "maxrange" => parse_number(value, |v| {
                        server.p1(27);
                        server.p2(v);
                    }),

                    // 28
                    "huntrange" => parse_number(value, |v| {
                        server.p1(28);
                        server.p1(v);
                    }),

                    // 29
                    "attackrange" => parse_number(value, |v| {
                        server.p1(29);
                        server.p1(v);
                    }),

                    // 30-39
                    "op1" | "op2" | "op3" | "op4" | "op5" => parse_number(&key[2..], |v: u8| {
                        if cfg!(before_647) || value != "hidden" {
                            client.p1(29 + v);
                            client.pjstr(value);
                        } else {
                            server.p1(29 + v);
                            server.pjstr(value);
                        }
                    }),

                    // 35
                    _ if key.starts_with("patrol") => {
                        if let Some((coord_str, delay_str)) = value.split_once(',')
                            && let (Some(coord), Ok(delay)) =
                                (parse_coord(coord_str), delay_str.parse::<u8>())
                        {
                            patrols.push((coord, delay));
                        }
                    }

                    // 36
                    "members" => parse_bool(value, |v| {
                        if v {
                            server.p1(36);
                        }
                    }),

                    // 37
                    "givechase" => parse_bool(value, |v| {
                        if !v {
                            server.p1(37);
                        }
                    }),

                    // 40
                    #[cfg(before_647)]
                    _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                        RecolType::S(idx, v) => recol_s.push((idx, v)),
                        RecolType::D(idx, v) => recol_d.push((idx, v)),
                    }),

                    // 60
                    #[cfg(before_647)]
                    _ if key
                        .strip_prefix("head")
                        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit())) =>
                    {
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
                    #[cfg(before_647)]
                    "resizex" => parse_number(value, |v| {
                        client.p1(90);
                        client.p2(v);
                    }),

                    // 91
                    #[cfg(before_647)]
                    "resizey" => parse_number(value, |v| {
                        client.p1(91);
                        client.p2(v);
                    }),

                    // 92
                    #[cfg(before_647)]
                    "resizez" => parse_number(value, |v| {
                        client.p1(92);
                        client.p2(v);
                    }),

                    // 93
                    "minimap" => parse_bool(value, |v| {
                        if !v {
                            client.p1(93);
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
                        #[cfg(before_647)]
                        {
                            has_vislevel = true;
                        }
                    }

                    // 97
                    "resizeh" => parse_number(value, |v| {
                        client.p1(97);
                        client.p2(v);
                    }),

                    // 98
                    "resizev" => parse_number(value, |v| {
                        client.p1(98);
                        client.p2(v);
                    }),

                    // 99
                    #[cfg(since_244)]
                    "alwaysontop" => parse_bool(value, |v| {
                        if v {
                            client.p1(99);
                        }
                    }),

                    // 100
                    #[cfg(since_244)]
                    "ambient" => parse_number(value, |v: i8| {
                        client.p1(100);
                        client.p1(v as u8);
                    }),

                    // 101
                    #[cfg(since_244)]
                    "contrast" => parse_number(value, |v: i8| {
                        client.p1(101);
                        client.p1(v as u8);
                    }),

                    // 102
                    #[cfg(since_244)]
                    "headicon" => parse_number(value, |v| {
                        client.p1(102);
                        client.p2(v);
                    }),

                    // 103
                    #[cfg(since_289)]
                    "turnspeed" => parse_number(value, |v| {
                        client.p1(103);
                        client.p2(v);
                    }),

                    // 107
                    #[cfg(since_647)]
                    "active" => parse_bool(value, |v| {
                        if !v {
                            client.p1(107);
                        }
                    }),

                    // 109
                    #[cfg(since_647)]
                    "walksmoothing" => parse_bool(value, |v| {
                        if !v {
                            client.p1(109);
                        }
                    }),

                    // 111
                    #[cfg(since_647)]
                    "spotshadow" => parse_bool(value, |v| {
                        if !v {
                            client.p1(111);
                        }
                    }),

                    // 113
                    #[cfg(since_647)]
                    "spotshadowcolour" => {
                        let values = parse_numbers::<i16, 2>(value);
                        client.p1(113);
                        for v in values {
                            client.p2(v as u16);
                        }
                    }

                    // 114
                    #[cfg(since_647)]
                    "spotshadowtrans" => {
                        let values = parse_numbers::<i8, 2>(value);
                        client.p1(114);
                        for v in values {
                            client.p1(v as u8);
                        }
                    }

                    // 119
                    #[cfg(since_647)]
                    "unknown119" => parse_number(value, |v: i8| {
                        client.p1(119);
                        client.p1(v as u8);
                    }),

                    // 122
                    #[cfg(since_647)]
                    "unknown122" => parse_number(value, |v: u16| {
                        client.p1(122);
                        client.p2(v);
                    }),

                    // 123
                    #[cfg(since_647)]
                    "overlayheight" => parse_number(value, |v: u16| {
                        client.p1(123);
                        client.p2(v);
                    }),

                    // 125
                    #[cfg(since_647)]
                    "respawndir" => parse_number(value, |v: i8| {
                        client.p1(125);
                        client.p1(v as u8);
                    }),

                    // 127
                    #[cfg(since_647)]
                    "bas" => parse_ref(&registry.bas, "bas", value, |v| {
                        client.p1(127);
                        client.p2(v);
                    }),

                    // 128
                    #[cfg(since_647)]
                    "defaultmovemode" => {
                        client.p1(128);
                        client.p1(match value.as_str() {
                            "crawl" => 0,
                            "walk" => 1,
                            "run" => 2,
                            other => other.parse().expect("Invalid defaultmovemode"),
                        });
                    }

                    // 134, and 162 when the sounds are vorbis files
                    #[cfg(since_647)]
                    "bgsound" => {
                        let parts: Vec<_> = value.split(',').collect();
                        let [idle, crawl, walk, run, range] = parts[..] else {
                            panic!("{debugname}: invalid bgsound '{value}'");
                        };
                        let mut vorbis = false;
                        client.p1(134);
                        for sound in [idle, crawl, walk, run] {
                            let (id, is_vorbis) = parse_sound(registry, sound.trim());
                            vorbis |= is_vorbis;
                            client.p2(id);
                        }
                        parse_number(range.trim(), |v: u8| client.p1(v));
                        if vorbis {
                            client.p1(162);
                        }
                    }

                    // 135
                    #[cfg(since_647)]
                    "cursor1" => pack_cursor(&mut client, 135, value, registry),

                    // 136
                    #[cfg(since_647)]
                    "cursor2" => pack_cursor(&mut client, 136, value, registry),

                    // 137
                    #[cfg(since_647)]
                    "cursorattack" => parse_ref(&registry.cursor, "cursor", value, |v| {
                        client.p1(137);
                        client.p2(v);
                    }),

                    // 138
                    #[cfg(since_647)]
                    "covermarker" => parse_ref(&registry.sprites, "sprites", value, |v| {
                        client.p1(138);
                        client.p2(v);
                    }),

                    // 139
                    #[cfg(since_647)]
                    "unknown139" => parse_number(value, |v: u16| {
                        client.p1(139);
                        client.p2(v);
                    }),

                    // 140
                    #[cfg(since_647)]
                    "bgsoundvolume" => parse_number(value, |v: u8| {
                        client.p1(140);
                        client.p1(v);
                    }),

                    // 141
                    #[cfg(since_647)]
                    "familiar" => parse_bool(value, |v| {
                        if v {
                            client.p1(141);
                        }
                    }),

                    // 142
                    #[cfg(since_647)]
                    "mapelement" => parse_ref(&registry.mel, "mel", value, |v| {
                        client.p1(142);
                        client.p2(v);
                    }),

                    // 143
                    #[cfg(since_647)]
                    "alwaysonbottom" => parse_bool(value, |v| {
                        if v {
                            client.p1(143);
                        }
                    }),

                    // 150-154
                    #[cfg(since_647)]
                    "membersop1" | "membersop2" | "membersop3" | "membersop4" | "membersop5" => {
                        client.p1(149 + key.as_bytes()["membersop".len()] - b'0');
                        client.pjstr(value);
                    }

                    // 155
                    #[cfg(since_647)]
                    "tint" => {
                        let values = parse_numbers::<i8, 4>(value);
                        client.p1(155);
                        for v in values {
                            client.p1(v as u8);
                        }
                    }

                    // 158 or 159
                    #[cfg(since_647)]
                    "reprioritiseattackop" => {
                        parse_bool(value, |v| client.p1(if v { 158 } else { 159 }))
                    }

                    // 163
                    #[cfg(since_647)]
                    "roughbounding" => parse_number(value, |v: u8| {
                        client.p1(163);
                        client.p1(v);
                    }),

                    // 164
                    #[cfg(since_647)]
                    "bgsoundrate" => {
                        let values = parse_numbers::<u16, 2>(value);
                        client.p1(164);
                        for v in values {
                            client.p2(v);
                        }
                    }

                    // 165
                    #[cfg(since_647)]
                    "unknown165" => parse_number(value, |v: u8| {
                        client.p1(165);
                        client.p1(v);
                    }),

                    // 168
                    #[cfg(since_647)]
                    "bgsounddropoffrange" => parse_number(value, |v: u8| {
                        client.p1(168);
                        client.p1(v);
                    }),

                    // 249
                    "param" => {} // handled at the end

                    // not found
                    _ => panic!("Unrecognized npc config key: {key}"),
                }
                server.dat.extend_from_slice(&client.dat[start..]);
            }
        }

        #[cfg(before_647)]
        {
            let start = client.dat.len();
            // handle 40
            if !recol_s.is_empty() {
                let count = recol_s.len();
                client.p1(40);
                client.p1(count as u8);
                for i in 0..count {
                    let s = recol_s[i].1;
                    let d = recol_d[i].1;
                    if s >= 100 || d >= 100 {
                        client.p2(rgb15_to_hsl16(s));
                        client.p2(rgb15_to_hsl16(d));
                    } else {
                        client.p2(s);
                        client.p2(d);
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

            // models (opcode 1)
            if !models.is_empty() {
                client.p1(1);
                client.p1(models.len() as u8);
                for (_, v) in &models {
                    client.p2(*v);
                }
            }

            // handle 60
            if !head_models.is_empty() {
                client.p1(60);
                client.p1(head_models.len() as u8);
                for (_, v) in &head_models {
                    client.p2(*v);
                }
            }

            // default vislevel=1 if not set (opcode 95)
            if !has_vislevel {
                client.p1(95);
                client.p2(1);
            }

            server.dat.extend_from_slice(&client.dat[start..]);
        }

        #[cfg(since_647)]
        for slot in 1..=5 {
            if let Some((_, value)) = props
                .iter()
                .find(|(key, _)| *key == format!("membersop{slot}"))
            {
                server.p1(29 + slot);
                server.pjstr(value);
            }
        }

        // handle 35
        if !patrols.is_empty() {
            server.p1(35);
            server.p1(patrols.len() as u8);
            for (coord, delay) in &patrols {
                server.p4(*coord);
                server.p1(*delay);
            }
        }

        // handle 249
        parse_params(
            registry,
            param_types,
            &mut server,
            props.iter().filter(|(key, _)| key == "param"),
            debugname,
        );

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        if cfg!(before_647) || !client.entry().is_empty() {
            client.finish_entry();
        } else {
            client.skip_entry();
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}

fn batch_code(key: &str) -> u8 {
    #[cfg(since_647)]
    {
        if config_key_index(key, "model").is_some() {
            return 1;
        }
        if config_key_index(key, "head").is_some() {
            return 60;
        }
        if key.starts_with("recol") {
            return 40;
        }
        if key.starts_with("retex") {
            return 41;
        }
        if config_key_index(key, "colour").is_some() {
            return 42;
        }
        if config_key_index(key, "modeloffset").is_some() {
            return 121;
        }
        if key == "quest" {
            return 160;
        }
        if matches!(key, "multivar" | "multinpc") {
            return 106;
        }
        if key == "clientparam" {
            return 249;
        }
    }
    #[cfg(before_647)]
    let _ = key;
    0
}

#[cfg(since_647)]
fn pack_batch(
    packed: &mut PackedData,
    props: &[(String, String)],
    registry: &PackRegistry,
    param_types: &HashMap<String, String>,
    name: &str,
) -> Result<()> {
    let code = batch_code(&props[0].0);
    match code {
        1 | 60 | 42 => {
            packed.p1(code);
            packed.p1(u8::try_from(props.len()).expect("Too many NPC values"));
            for (_, value) in props {
                match code {
                    42 => parse_number(value, |v| packed.p1(v)),
                    _ => parse_ref(&registry.model, "model", value, |v| packed.p2(v)),
                }
            }
        }
        40 => pack_recol_pairs(packed, props),
        41 => pack_retex_pairs(packed, props, registry),
        121 => {
            packed.p1(121);
            packed.p1(u8::try_from(props.len()).expect("Too many NPC model offsets"));
            for (key, value) in props {
                let index = config_key_index(key, "modeloffset").expect("Invalid model offset");
                packed.p1(u8::try_from(index).expect("Invalid model offset"));
                for v in parse_numbers::<i8, 3>(value) {
                    packed.p1(v as u8);
                }
            }
        }
        160 => {
            let quests: Vec<_> = props
                .iter()
                .flat_map(|(_, value)| value.split(','))
                .collect();
            packed.p1(160);
            packed.p1(u8::try_from(quests.len()).expect("Too many NPC quests"));
            for quest in quests {
                parse_ref(&registry.quest, "quest", quest.trim(), |v| packed.p2(v));
            }
        }
        106 => pack_multi(
            packed,
            (106, 118),
            props,
            ("npc", &registry.npc),
            registry,
            name,
        )?,
        249 => parse_params(registry, param_types, packed, props, name),
        _ => unreachable!(),
    }
    Ok(())
}
