use super::config_entry;
#[cfg(since_647)]
use super::{cache_params, mark_client_params};
use crate::pack::config::param::parse_params;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
#[cfg(before_647)]
use crate::pack::util::colour::{RecolType, rgb15_to_hsl16};
use crate::pack::util::*;
use crate::types::ForceApproach;
#[cfg(before_647)]
use crate::types::LocShape;
#[cfg(since_647)]
use crate::types::loc_shape_from_key;
use anyhow::Result;
#[cfg(since_647)]
use anyhow::{Context, bail};
use std::collections::HashMap;

#[cfg(before_647)]
struct LocModelShape {
    model: u16,
    shape: u8,
}

/// A yes/no key that also takes the byte itself.
fn flag(value: &str) -> u8 {
    match value {
        "yes" | "true" => 1,
        "no" | "false" => 0,
        _ => value
            .parse()
            .unwrap_or_else(|e| panic!("Invalid flag value '{value}': {e}")),
    }
}

pub fn pack_locs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.loc;

    let configs = parse_config_sections_cached(file_cache, "loc", constants)?;

    #[cfg(since_647)]
    let cache_params = cache_params(file_cache, &registry.param);

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some((debugname, props)) = config_entry("loc", pack, &configs, id) else {
            server.finish_entry();
            client.skip_entry();
            continue;
        };
        // An explicit declaration can name an absent cache slot without
        // turning it into an empty (opcode-0) entry on the client.
        if props
            .iter()
            .any(|(key, value)| key == "absent" && value == "yes")
        {
            anyhow::ensure!(
                props.len() == 1,
                "Absent loc {debugname} cannot define properties"
            );
            server.finish_entry();
            client.skip_entry();
            continue;
        }

        #[cfg(since_647)]
        let props = &mark_client_params(props, &registry.param, &cache_params);

        #[cfg(before_647)]
        let mut src_models: Vec<&str> = Vec::new();
        #[cfg(before_647)]
        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        #[cfg(before_647)]
        let mut recol_d: Vec<(usize, u16)> = Vec::new();
        #[cfg(all(since_289, before_647))]
        let mut multivar: Option<&str> = None;
        #[cfg(all(since_289, before_647))]
        let mut multiloc: Vec<(usize, &str)> = Vec::new();
        #[cfg(before_647)]
        let mut active = -1;
        #[cfg(before_647)]
        let mut name = None;
        #[cfg(before_647)]
        let mut desc = None;

        for group in props.chunk_by(|a, b| batch_code(a) == batch_code(b)) {
            #[cfg(since_647)]
            if batch_code(&group[0]) != 0 {
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
                    _ if key.starts_with("model") => src_models.push(value),

                    // 2
                    "name" => {
                        #[cfg(before_647)]
                        {
                            name = Some(value.to_string());
                        }
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
                            desc = Some(value);
                        }
                        #[cfg(since_647)]
                        {
                            server.p1(3);
                            server.pjstr(value);
                        }
                    }

                    // 14
                    "width" => parse_number(value, |v| {
                        client.p1(14);
                        client.p1(v);
                    }),

                    // 15
                    "length" => parse_number(value, |v| {
                        client.p1(15);
                        client.p1(v);
                    }),

                    // 17 or 27
                    "blockwalk" => parse_bool(value, |v| {
                        if !v {
                            client.p1(17);
                        } else if cfg!(since_647) {
                            client.p1(27);
                        }
                    }),

                    // 18
                    "blockrange" => parse_bool(value, |v| {
                        if !v {
                            client.p1(18);
                        }
                    }),

                    // 19
                    "active" => {
                        let v = flag(value);
                        client.p1(19);
                        client.p1(v);
                        #[cfg(before_647)]
                        {
                            active = v as i8;
                        }
                    }

                    // 21
                    "hillskew" => parse_bool(value, |v| {
                        if v {
                            client.p1(21);
                        }
                    }),

                    // 22
                    "sharelight" => parse_bool(value, |v| {
                        if v {
                            client.p1(22);
                        }
                    }),

                    // 23 or 103
                    "occlude" => parse_bool(value, |v| {
                        if v {
                            client.p1(23);
                        } else if cfg!(since_647) {
                            client.p1(103);
                        }
                    }),

                    // 24
                    "anim" => parse_ref(&registry.seq, "seq", value, |v| {
                        client.p1(24);
                        client.p2(v);
                    }),

                    // 25
                    "hasalpha" => parse_bool(value, |v| {
                        if v {
                            client.p1(25);
                        }
                    }),

                    // 28
                    "wallwidth" => parse_number(value, |v| {
                        client.p1(28);
                        client.p1(v);
                    }),

                    // 29
                    "ambient" => parse_number(value, |v: i8| {
                        client.p1(29);
                        client.p1(v as u8);
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

                    // 39
                    "contrast" => parse_number(value, |v: i8| {
                        client.p1(39);
                        client.p1(v as u8);
                    }),

                    // 40
                    #[cfg(before_647)]
                    _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                        RecolType::S(idx, v) => recol_s.push((idx, rgb15_to_hsl16(v))),
                        RecolType::D(idx, v) => recol_d.push((idx, rgb15_to_hsl16(v))),
                    }),

                    // retextures stored in recol until rev 465
                    #[cfg(before_647)]
                    _ if key.starts_with("retex") => {
                        parse_retex(registry, key, value, |v| match v {
                            RecolType::S(idx, v) => recol_s.push((idx, v)),
                            RecolType::D(idx, v) => recol_d.push((idx, v)),
                        })
                    }

                    // 60
                    #[cfg(before_647)]
                    "mapfunction" => parse_number(value, |v| {
                        client.p1(60);
                        client.p2(v);
                    }),

                    // 61
                    "category" => parse_ref(&registry.category, "category", value, |v| {
                        server.p1(61);
                        server.p2(v);
                    }),

                    // 62
                    "mirror" => parse_bool(value, |v| {
                        if v {
                            client.p1(62);
                        }
                    }),

                    // 64
                    "shadow" => parse_bool(value, |v| {
                        if !v {
                            client.p1(64);
                        }
                    }),

                    // 65
                    "resizex" => parse_number(value, |v| {
                        client.p1(65);
                        client.p2(v);
                    }),

                    // 66
                    "resizey" => parse_number(value, |v| {
                        client.p1(66);
                        client.p2(v);
                    }),

                    // 67
                    "resizez" => parse_number(value, |v| {
                        client.p1(67);
                        client.p2(v);
                    }),

                    // 68
                    #[cfg(before_647)]
                    "mapscene" => parse_number(value, |v| {
                        client.p1(68);
                        client.p2(v);
                    }),

                    // 69
                    "forceapproach" => {
                        let val = value.parse::<u8>().map_or_else(
                            |_| ForceApproach::from_config_str(value),
                            |v| ForceApproach::try_from(v).expect("Invalid forceapproach"),
                        );
                        client.p1(69);
                        client.p1(val as u8);
                    }

                    // 70
                    "offsetx" => parse_number(value, |v: i16| {
                        client.p1(70);
                        client.p2(v as u16);
                    }),

                    // 71
                    "offsety" => parse_number(value, |v: i16| {
                        client.p1(71);
                        client.p2(v as u16);
                    }),

                    // 72
                    "offsetz" => parse_number(value, |v: i16| {
                        client.p1(72);
                        client.p2(v as u16);
                    }),

                    // 73
                    "forcedecor" => parse_bool(value, |v| {
                        if v {
                            client.p1(73);
                        }
                    }),

                    // 74
                    #[cfg(since_245_2)]
                    "breakroutefinding" => parse_bool(value, |v| {
                        if v {
                            client.p1(74);
                        }
                    }),

                    // 75
                    #[cfg(since_254)]
                    "raiseobject" => {
                        client.p1(75);
                        client.p1(flag(value));
                    }

                    // 77 (collected, emitted after the loop)
                    #[cfg(all(since_289, before_647))]
                    "multivar" => multivar = Some(value.as_str()),

                    #[cfg(all(since_289, before_647))]
                    "multiloc" => {
                        let (idx, loc_name) = value
                            .split_once(',')
                            .unwrap_or_else(|| panic!("{debugname}: invalid multiloc '{value}'"));
                        let idx: usize = idx.trim().parse().unwrap_or_else(|e| {
                            panic!("{debugname}: invalid multiloc index '{idx}': {e}")
                        });
                        multiloc.push((idx, loc_name.trim()));
                    }

                    // 78, and 168 when the sound is a vorbis file
                    #[cfg(since_647)]
                    "bgsound" => {
                        let (sound, range) = value
                            .split_once(',')
                            .with_context(|| format!("{debugname}: invalid bgsound '{value}'"))?;
                        let (id, vorbis) = parse_sound(registry, sound.trim());
                        client.p1(78);
                        client.p2(id);
                        parse_number(range.trim(), |v: u8| client.p1(v));
                        if vorbis {
                            client.p1(168);
                        }
                    }

                    // 79, and 169 when the sounds are vorbis files
                    #[cfg(since_647)]
                    "randomsound" => {
                        let parts: Vec<_> = value.split(',').collect();
                        let [min, max, range, sounds @ ..] = &parts[..] else {
                            bail!("{debugname}: invalid randomsound '{value}'");
                        };
                        client.p1(79);
                        parse_number(min.trim(), |v: u16| client.p2(v));
                        parse_number(max.trim(), |v: u16| client.p2(v));
                        parse_number(range.trim(), |v: u8| client.p1(v));
                        client.p1(u8::try_from(sounds.len())?);
                        let mut vorbis = false;
                        for sound in sounds {
                            let (id, is_vorbis) = parse_sound(registry, sound.trim());
                            vorbis |= is_vorbis;
                            client.p2(id);
                        }
                        if vorbis {
                            client.p1(169);
                        }
                    }

                    // 81, 93, 94, 95 or 162
                    #[cfg(since_647)]
                    "hillchange" => {
                        let (mode, param) = value
                            .split_once(',')
                            .map_or((value.as_str(), ""), |(mode, param)| (mode, param.trim()));
                        match mode {
                            "tree_skew" => parse_number(param, |v: u8| {
                                client.p1(81);
                                client.p1(v);
                            }),
                            "rotate" => parse_number(param, |v: i32| {
                                if let Ok(v) = u16::try_from(v) {
                                    client.p1(93);
                                    client.p2(v);
                                } else {
                                    client.p1(162);
                                    client.p4(v);
                                }
                            }),
                            "ceiling_skew" => client.p1(94),
                            "skew_to_fit" => parse_number(param, |v: u16| {
                                client.p1(95);
                                client.p2(v);
                            }),
                            _ => bail!("{debugname}: invalid hillchange '{value}'"),
                        }
                    }

                    // 82
                    #[cfg(since_647)]
                    "istexture" => parse_bool(value, |v| {
                        if v {
                            client.p1(82);
                        }
                    }),

                    // 88
                    #[cfg(since_647)]
                    "hardshadow" => parse_bool(value, |v| {
                        if !v {
                            client.p1(88);
                        }
                    }),

                    // 89
                    #[cfg(since_647)]
                    "randomanimframe" => parse_bool(value, |v| {
                        if !v {
                            client.p1(89);
                        }
                    }),

                    // 90
                    #[cfg(since_647)]
                    "unknown90" => parse_bool(value, |v| {
                        if v {
                            client.p1(90);
                        }
                    }),

                    // 91
                    #[cfg(since_647)]
                    "members" => parse_bool(value, |v| {
                        if v {
                            client.p1(91);
                        }
                    }),

                    // 97
                    #[cfg(since_647)]
                    "msirotate" => parse_bool(value, |v| {
                        if v {
                            client.p1(97);
                        }
                    }),

                    // 98
                    #[cfg(since_647)]
                    "hasparticles" => parse_bool(value, |v| {
                        if v {
                            client.p1(98);
                        }
                    }),

                    // 99
                    #[cfg(since_647)]
                    "cursor1" => pack_cursor(&mut client, 99, value, registry),

                    // 100
                    #[cfg(since_647)]
                    "cursor2" => pack_cursor(&mut client, 100, value, registry),

                    // 101
                    #[cfg(since_647)]
                    "mapfunction" => parse_number(value, |v: u8| {
                        client.p1(101);
                        client.p1(v);
                    }),

                    // 102
                    #[cfg(since_647)]
                    "msi" => parse_ref(&registry.msi, "msi", value, |v| {
                        client.p1(102);
                        client.p2(v);
                    }),

                    // 104
                    #[cfg(since_647)]
                    "bgsoundvolume" => parse_number(value, |v: u8| {
                        client.p1(104);
                        client.p1(v);
                    }),

                    // 105
                    #[cfg(since_647)]
                    "msimirror" => parse_bool(value, |v| {
                        if v {
                            client.p1(105);
                        }
                    }),

                    // 107
                    #[cfg(since_647)]
                    "mapelement" => parse_ref(&registry.mel, "mel", value, |v| {
                        client.p1(107);
                        client.p2(v);
                    }),

                    // 150-154
                    #[cfg(since_647)]
                    "membersop1" | "membersop2" | "membersop3" | "membersop4" | "membersop5" => {
                        client.p1(149 + key.as_bytes()["membersop".len()] - b'0');
                        client.pjstr(value);
                    }

                    // 163
                    #[cfg(since_647)]
                    "tint" => {
                        client.p1(163);
                        for v in parse_numbers::<i8, 4>(value) {
                            client.p1(v as u8);
                        }
                    }

                    // 164
                    #[cfg(since_647)]
                    "postoffsetx" => parse_number(value, |v: i16| {
                        client.p1(164);
                        client.p2(v as u16);
                    }),

                    // 165
                    #[cfg(since_647)]
                    "postoffsety" => parse_number(value, |v: i16| {
                        client.p1(165);
                        client.p2(v as u16);
                    }),

                    // 166
                    #[cfg(since_647)]
                    "postoffsetz" => parse_number(value, |v: i16| {
                        client.p1(166);
                        client.p2(v as u16);
                    }),

                    // 167
                    #[cfg(since_647)]
                    "raiseobjectgrounddecor" => parse_number(value, |v: u16| {
                        client.p1(167);
                        client.p2(v);
                    }),

                    // 170
                    #[cfg(since_647)]
                    "unknown170" => parse_number(value, |v: i32| {
                        client.p1(170);
                        emit_smart_packed(&mut client, v);
                    }),

                    // 171
                    #[cfg(since_647)]
                    "unknown171" => parse_number(value, |v: i32| {
                        client.p1(171);
                        emit_smart_packed(&mut client, v);
                    }),

                    // 173
                    #[cfg(since_647)]
                    "bgsoundrate" => {
                        client.p1(173);
                        for v in parse_numbers::<u16, 2>(value) {
                            client.p2(v);
                        }
                    }

                    // 177
                    #[cfg(since_647)]
                    "unknown177" => parse_bool(value, |v| {
                        if v {
                            client.p1(177);
                        }
                    }),

                    // 178
                    #[cfg(since_647)]
                    "bgsounddropoffrange" => parse_number(value, |v: u8| {
                        client.p1(178);
                        client.p1(v);
                    }),

                    // 249
                    "param" => {} // handled at the end

                    // not found
                    _ => panic!("Unrecognized loc config key: {key}"),
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
                    client.p2(s);
                    client.p2(d);
                }
            }

            // handle 77 (multivar / multiloc morph)
            #[cfg(since_289)]
            if let Some(varbit_name) = multivar {
                let varbit_id = registry
                    .varbit
                    .get_by_debugname(varbit_name)
                    .unwrap_or_else(|| {
                        panic!("{debugname}: unknown multivar varpbit '{varbit_name}'")
                    });
                let count = multiloc.iter().map(|(i, _)| *i).max().unwrap_or(0);
                client.p1(77);
                client.p2(varbit_id);
                client.p1(count as u8);
                for i in 0..=count {
                    let id = match multiloc.iter().find(|(idx, _)| *idx == i).map(|&(_, n)| n) {
                        Some(name) => registry
                            .loc
                            .get_by_debugname(name)
                            .unwrap_or_else(|| panic!("{debugname}: unknown multiloc '{name}'")),
                        None => u16::MAX,
                    };
                    client.p2(id);
                }
            }

            // handle 1
            let try_model = |name: &str| registry.model.get_by_debugname(name);

            let mut models: Vec<LocModelShape> = Vec::new();
            for src in &src_models {
                let direct_reference = try_model(src).is_some()
                    && (LocShape::WallStraight as u8..=LocShape::GroundDecor as u8)
                        .filter(|&s| s != LocShape::CentrepieceStraight as u8)
                        .all(|s| {
                            try_model(&format!("{src}{}", LocShape::try_from(s).unwrap().suffix()))
                                .is_none()
                        });

                if direct_reference && let Some(id) = try_model(src) {
                    models.push(LocModelShape {
                        model: id,
                        shape: LocShape::CentrepieceStraight as u8,
                    });
                    continue;
                }

                if let Some(id) =
                    try_model(&format!("{src}{}", LocShape::CentrepieceStraight.suffix()))
                        .or_else(|| try_model(src))
                {
                    models.push(LocModelShape {
                        model: id,
                        shape: LocShape::CentrepieceStraight as u8,
                    });
                }

                // now check the rest of the shapes
                for shape in LocShape::WallStraight as u8..=LocShape::GroundDecor as u8 {
                    if shape == LocShape::CentrepieceStraight as u8 {
                        continue;
                    }
                    let loc_shape = LocShape::try_from(shape)?;
                    if let Some(id) = try_model(&format!("{src}{}", loc_shape.suffix())) {
                        models.push(LocModelShape { model: id, shape });
                    }
                }
            }

            if !src_models.is_empty() && models.is_empty() {
                panic!("{debugname}: Failed to find suitable loc models");
            }

            models.sort_by_key(|m| (m.shape != LocShape::CentrepieceStraight as u8, m.shape));

            #[cfg(before_254)]
            if !models.is_empty() {
                client.p1(1);
                client.p1(models.len() as u8);
                for m in &models {
                    client.p2(m.model);
                    client.p1(m.shape);
                }
            }

            #[cfg(since_254)]
            if !models.is_empty() {
                let mut centrepiece_only = true;
                for model in &models {
                    let loc_shape = LocShape::try_from(model.shape)?;
                    if loc_shape != LocShape::CentrepieceStraight {
                        centrepiece_only = false;
                        break;
                    }
                }

                if centrepiece_only {
                    client.p1(5);
                    client.p1(models.len() as u8);
                    for m in &models {
                        client.p2(m.model);
                    }
                } else {
                    client.p1(1);
                    client.p1(models.len() as u8);
                    for m in &models {
                        client.p2(m.model);
                        client.p1(m.shape);
                    }
                }
            }

            // edge case: no name= but has centrepiece_straight shape or active=yes
            if name.is_none() && active != 0 {
                let mut should_transmit = active == 1;
                if active == -1 {
                    for m in &models {
                        if m.shape == LocShape::CentrepieceStraight as u8 {
                            should_transmit = true;
                            break;
                        }
                    }
                }
                if should_transmit {
                    name = Some(debugname.to_string());
                }
            }

            // handle 2
            if let Some(name) = &name {
                client.p1(2);
                client.pjstr(name);
            }

            // handle 3
            if let Some(desc) = desc {
                client.p1(3);
                client.pjstr(desc);
            }

            server.dat.extend_from_slice(&client.dat[start..]);
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
        client.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}

fn batch_code(prop: &(String, String)) -> u8 {
    #[cfg(since_647)]
    {
        let (key, value) = prop;
        match key.as_str() {
            "model" => return 1,
            "modelhd" | "modelsd" => return 5,
            "anim" if value.contains(',') => return 106,
            "quest" => return 160,
            "multivar" | "multiloc" => return 77,
            "clientparam" => return 249,
            _ => {}
        }
        if config_key_index(key, "recolpalette").is_some() {
            return 42;
        }
        if key.starts_with("recol") {
            return 40;
        }
        if key.starts_with("retex") {
            return 41;
        }
    }
    #[cfg(before_647)]
    let _ = prop;
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
    let code = batch_code(&props[0]);
    match code {
        1 | 5 => {
            packed.p1(code);
            if code == 1 {
                pack_models(packed, props, "model", registry)?;
            } else {
                pack_models(packed, props, "modelhd", registry)?;
                pack_models(packed, props, "modelsd", registry)?;
            }
        }
        40 => pack_recol_pairs(packed, props),
        41 => pack_retex_pairs(packed, props, registry),
        42 => {
            packed.p1(42);
            packed.p1(u8::try_from(props.len()).expect("Too many loc values"));
            for (_, value) in props {
                parse_number(value, |v: i8| packed.p1(v as u8));
            }
        }
        106 => {
            packed.p1(106);
            packed.p1(u8::try_from(props.len()).expect("Too many loc values"));
            for (_, value) in props {
                let (seq, weight) = value
                    .split_once(',')
                    .with_context(|| format!("{name}: invalid anim '{value}'"))?;
                parse_ref(&registry.seq, "seq", seq.trim(), |v| packed.p2(v));
                parse_number(weight.trim(), |v: u8| packed.p1(v));
            }
        }
        160 => {
            let quests: Vec<_> = props
                .iter()
                .flat_map(|(_, value)| value.split(','))
                .collect();
            packed.p1(160);
            packed.p1(u8::try_from(quests.len()).expect("Too many loc quests"));
            for quest in quests {
                parse_ref(&registry.quest, "quest", quest.trim(), |v| packed.p2(v));
            }
        }
        77 => pack_multi(
            packed,
            (77, 92),
            props,
            ("loc", &registry.loc),
            registry,
            name,
        )?,
        249 => parse_params(registry, param_types, packed, props, name),
        _ => unreachable!(),
    }
    Ok(())
}

/// The lines of `key`: consecutive ones of a shape are one group.
#[cfg(since_647)]
fn pack_models(
    packed: &mut PackedData,
    props: &[(String, String)],
    key: &str,
    registry: &PackRegistry,
) -> Result<()> {
    let models = props
        .iter()
        .filter(|(line, _)| line == key)
        .map(|(_, value)| {
            value
                .split_once(',')
                .with_context(|| format!("Invalid {key} '{value}'"))
        })
        .collect::<Result<Vec<_>>>()?;
    let groups: Vec<_> = models.chunk_by(|a, b| a.0 == b.0).collect();
    packed.p1(u8::try_from(groups.len()).expect("Too many model shapes"));
    for group in groups {
        let shape = loc_shape_from_key(group[0].0.trim())
            .with_context(|| format!("Invalid {key} shape '{}'", group[0].0))?;
        packed.p1(shape);
        packed.p1(u8::try_from(group.len()).expect("Too many models"));
        for (_, model) in group {
            parse_ref(&registry.model, "model", model.trim(), |v| packed.p2(v));
        }
    }
    Ok(())
}

#[cfg(since_647)]
fn emit_smart_packed(out: &mut PackedData, value: i32) {
    if (0..128).contains(&value) {
        out.p1(value as u8);
    } else {
        out.p2((value + 32768) as u16);
    }
}

#[cfg(all(test, since_647))]
mod tests {
    use super::pack_locs;
    use crate::cache::loc::{LocType, LocTypeRaw};
    use crate::cache::provider::CacheType;
    use crate::pack::pack::FileCache;
    use crate::pack::pack_registry::PackRegistry;
    use crate::types::MultiVar;
    use crate::unpack::config::{UnpackedPacks, decode_record, loc::emit_loc_props};
    use rs_io::Packet;
    use std::collections::HashMap;
    use std::{env, fs, process};

    #[test]
    fn an_absent_loc_declaration_preserves_the_empty_cache_slot() {
        let dir = env::temp_dir().join(format!("rs-majula-absent-loc-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut registry = PackRegistry::load(&dir).unwrap();
        registry.loc.max = 1;
        registry.loc.id_to_debugname.insert(0, "missing".into());
        registry.loc.debugname_to_id.insert("missing".into(), 0);
        let pack = || {
            pack_locs(
                &FileCache::new(&dir),
                &registry,
                &HashMap::new(),
                &HashMap::new(),
            )
            .unwrap()
        };
        let original = pack();
        fs::write(dir.join("missing.loc"), "[missing]\nabsent=yes\n").unwrap();
        let declared = pack();
        assert_eq!(original.server.dat, declared.server.dat);
        assert_eq!(original.server.idx, declared.server.idx);
        let original = original.client.unwrap();
        let declared = declared.client.unwrap();
        assert_eq!(original.dat, declared.dat);
        assert_eq!(original.idx, declared.idx);
        fs::remove_file(dir.join("missing.loc")).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn multiloc_config_preserves_both_selectors_and_defaults() {
        let dir = env::temp_dir().join(format!("rs-majula-multiloc-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut registry = PackRegistry::load(&dir).unwrap();
        registry.loc.max = 1;
        for (pack, id, name) in [
            (&mut registry.loc, 0, "subject"),
            (&mut registry.varbit, 3, "phase_bits"),
            (&mut registry.varp, 7, "phase"),
        ] {
            pack.id_to_debugname.insert(id, name.into());
            pack.debugname_to_id.insert(name.into(), id);
        }
        registry.loc.id_to_debugname.insert(9, "shared".into());
        registry.loc.id_to_debugname.insert(11, "shared".into());
        registry.loc.debugname_to_id.insert("shared".into(), 11);

        for (bytes, var) in [
            (
                vec![77, 0, 3, 255, 255, 2, 0, 9, 255, 255, 255, 255, 0],
                Some(MultiVar::Varbit(3)),
            ),
            (
                vec![77, 255, 255, 0, 7, 0, 0, 9, 0],
                Some(MultiVar::Varp(7)),
            ),
            (
                vec![92, 0, 3, 0, 7, 0, 11, 2, 0, 9, 255, 255, 255, 255, 0],
                Some(MultiVar::Varbit(3)),
            ),
            (vec![92, 255, 255, 255, 255, 255, 255, 0, 255, 255, 0], None),
        ] {
            let mut def = LocTypeRaw::new(0);
            def.decode(&mut Packet::from(bytes.clone()));
            let mut text = "[subject]\n".to_string();
            for (key, value) in decode_record(
                &mut Packet::from(bytes.clone()),
                0,
                &HashMap::new(),
                &mut UnpackedPacks::new(),
                emit_loc_props,
            ) {
                assert!(key == "multivar" || key == "multiloc");
                if value != "1,null" {
                    text.push_str(&format!("{key}={value}\n"));
                }
            }
            fs::write(dir.join("all.loc"), text).unwrap();
            let packed = pack_locs(
                &FileCache::new(&dir),
                &registry,
                &HashMap::new(),
                &HashMap::new(),
            )
            .unwrap()
            .client
            .unwrap();
            assert_eq!(packed.dat[2..], bytes);
            let loc = LocType::from(def);
            assert_eq!(loc.multivar, var);
            let default = if bytes[0] == 92 && var.is_some() {
                Some(11)
            } else {
                None
            };
            assert_eq!(loc.multi_loc(-1), default);
            assert_eq!(loc.multi_loc(1), default);
            assert_eq!(loc.multi_loc(256), default);
        }
        fs::remove_file(dir.join("all.loc")).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
