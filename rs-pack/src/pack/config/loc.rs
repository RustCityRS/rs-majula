#[cfg(since_647)]
use anyhow::{Context, bail};
#[cfg(since_647)]
use std::collections::BTreeMap;
use std::collections::HashMap;

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
use anyhow::Result;
use tracing::debug;

#[cfg(before_647)]
struct LocModelShape {
    model: u16,
    shape: u8,
}

pub fn pack_locs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.loc;

    let files = file_cache.collect("loc");
    debug!("  Found {} .loc files", files.len());

    let configs = parse_config_sections_cached(file_cache, "loc", constants);
    debug!("  Parsed {} loc configs", configs.len());

    #[cfg(since_647)]
    let mut loc_ids = HashMap::new();
    #[cfg(since_647)]
    for id in 0..pack.max {
        if let Some(name) = pack.get_by_id(id) {
            loc_ids.entry(name).or_insert(id);
        }
    }

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            #[cfg(before_647)]
            panic!("Unknown loc id: {id}");
            #[cfg(since_647)]
            {
                server.finish_entry();
                client.skip_entry();
                continue;
            }
        };

        let Some(props) = configs.get(debugname) else {
            #[cfg(before_647)]
            panic!("Unknown loc config: {debugname}");
            #[cfg(since_647)]
            {
                server.finish_entry();
                client.skip_entry();
                continue;
            }
        };

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

                    // 17
                    "blockwalk" => parse_bool(value, |v| {
                        if !v {
                            client.p1(17);
                        }
                    }),

                    // 18
                    "blockrange" => parse_bool(value, |v| {
                        if !v {
                            client.p1(18);
                        }
                    }),

                    // 19
                    #[cfg(before_647)]
                    "active" => parse_bool(value, |v| {
                        client.p1(19);
                        client.p1(v as u8);
                        #[cfg(before_647)]
                        {
                            active = v as i8;
                        }
                    }),

                    #[cfg(since_647)]
                    "active" => {
                        client.p1(19);
                        if matches!(value.as_str(), "yes" | "no" | "true" | "false") {
                            parse_bool(value, |v| client.p1(v as u8));
                        } else {
                            parse_number(value, |v| client.p1(v));
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

                    // 23
                    "occlude" => parse_bool(value, |v| {
                        if v {
                            client.p1(23);
                        }
                    }),

                    // 24
                    "anim" => parse_seq(registry, value, |v| {
                        client.p1(24);
                        client.p2(v);
                    }),

                    // 25
                    "hasalpha" => parse_bool(value, |v| {
                        if v {
                            client.p1(25);
                        }
                    }),

                    #[cfg(since_647)]
                    "interact_type1" => parse_bool(value, |v| {
                        if v {
                            client.p1(27);
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
                    "category" => parse_category(registry, value, |v| {
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
                        #[cfg(before_647)]
                        let val = ForceApproach::from_config_str(value);
                        #[cfg(since_647)]
                        let val = value.parse::<u8>().map_or_else(
                            |_| ForceApproach::from_config_str(value),
                            |v| ForceApproach::try_from(v).expect("Invalid forceapproach"),
                        );
                        client.p1(69);
                        client.p1(val as u8);
                    }

                    // 70
                    "offsetx" => parse_number(value, |v| {
                        client.p1(70);
                        client.p2(v);
                    }),

                    // 71
                    "offsety" => parse_number(value, |v| {
                        client.p1(71);
                        client.p2(v);
                    }),

                    // 72
                    "offsetz" => parse_number(value, |v| {
                        client.p1(72);
                        client.p2(v);
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
                    #[cfg(before_647)]
                    "raiseobject" => parse_bool(value, |v| {
                        client.p1(75);
                        client.p1(v as u8);
                    }),

                    #[cfg(since_647)]
                    "raiseobject" => {
                        client.p1(75);
                        if matches!(value.as_str(), "yes" | "no" | "true" | "false") {
                            parse_bool(value, |v| client.p1(v as u8));
                        } else {
                            parse_number(value, |v| client.p1(v));
                        }
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

                    #[cfg(since_647)]
                    "ambient_sfx" => {
                        if let Some((id_s, range_s)) = value.split_once(',') {
                            client.p1(78);
                            client.p2(id_s.trim().parse::<u16>().expect("Invalid loc value"));
                            client.p1(range_s.trim().parse::<u8>().expect("Invalid loc value"));
                        }
                    }

                    #[cfg(since_647)]
                    "ambient_sfx_random" => {
                        let parts: Vec<&str> = value.splitn(4, ',').collect();
                        if parts.len() == 4 {
                            client.p1(79);
                            client.p2(parts[0].trim().parse::<u16>().expect("Invalid loc value"));
                            client.p2(parts[1].trim().parse::<u16>().expect("Invalid loc value"));
                            client.p1(parts[2].trim().parse::<u8>().expect("Invalid loc value"));
                            let ids: Vec<u16> = if parts[3].is_empty() {
                                Vec::new()
                            } else {
                                parts[3]
                                    .split(';')
                                    .map(|s| s.trim().parse::<u16>().expect("Invalid loc value"))
                                    .collect()
                            };
                            client.p1(ids.len() as u8);
                            for id in &ids {
                                client.p2(*id);
                            }
                        }
                    }

                    #[cfg(since_647)]
                    "hillskew_t2" => {
                        client.p1(81);
                        client.p1(value.parse::<u8>().expect("Invalid loc value"));
                    }

                    #[cfg(since_647)]
                    "hidden" => parse_bool(value, |v| {
                        if v {
                            client.p1(82);
                        }
                    }),

                    #[cfg(since_647)]
                    "nominimap_visible" => parse_bool(value, |v| {
                        if v {
                            client.p1(88);
                        }
                    }),

                    #[cfg(since_647)]
                    "noblocks_npcs" => parse_bool(value, |v| {
                        if v {
                            client.p1(89);
                        }
                    }),

                    #[cfg(since_647)]
                    "op90" => parse_bool(value, |v| {
                        if v {
                            client.p1(90);
                        }
                    }),

                    #[cfg(since_647)]
                    "members_only" => parse_bool(value, |v| {
                        if v {
                            client.p1(91);
                        }
                    }),

                    #[cfg(since_647)]
                    "hillskew_t3_u16" => {
                        client.p1(93);
                        client.p2(value.parse::<u16>().expect("Invalid loc value"));
                    }

                    #[cfg(since_647)]
                    "hillskew_t4" => parse_bool(value, |v| {
                        if v {
                            client.p1(94);
                        }
                    }),

                    #[cfg(since_647)]
                    "hillskew_t5" => {
                        client.p1(95);
                        client.p2(value.parse::<u16>().expect("Invalid loc value"));
                    }

                    #[cfg(since_647)]
                    "op97" => parse_bool(value, |v| {
                        if v {
                            client.p1(97);
                        }
                    }),

                    #[cfg(since_647)]
                    "op98" => parse_bool(value, |v| {
                        if v {
                            client.p1(98);
                        }
                    }),

                    #[cfg(since_647)]
                    "cursor1" => {
                        if let Some((a_s, b_s)) = value.split_once(',') {
                            client.p1(99);
                            client.p1(a_s.trim().parse::<u8>().expect("Invalid loc value"));
                            client.p2(b_s.trim().parse::<u16>().expect("Invalid loc value"));
                        }
                    }

                    #[cfg(since_647)]
                    "cursor2" => {
                        if let Some((a_s, b_s)) = value.split_once(',') {
                            client.p1(100);
                            client.p1(a_s.trim().parse::<u8>().expect("Invalid loc value"));
                            client.p2(b_s.trim().parse::<u16>().expect("Invalid loc value"));
                        }
                    }

                    #[cfg(since_647)]
                    "mapfunction" => {
                        client.p1(101);
                        client.p1(value.parse::<u8>().expect("Invalid loc value"));
                    }

                    #[cfg(since_647)]
                    "msi_id" => {
                        client.p1(102);
                        client.p2(value.parse::<u16>().expect("Invalid loc value"));
                    }

                    #[cfg(since_647)]
                    "occlude_off" => parse_bool(value, |v| {
                        if v {
                            client.p1(103);
                        }
                    }),

                    #[cfg(since_647)]
                    "interaction_sides" => {
                        client.p1(104);
                        client.p1(value.parse::<u8>().expect("Invalid loc value"));
                    }

                    #[cfg(since_647)]
                    "op105" => parse_bool(value, |v| {
                        if v {
                            client.p1(105);
                        }
                    }),

                    #[cfg(since_647)]
                    "mapelement" => {
                        client.p1(107);
                        client.p2(value.parse::<u16>().expect("Invalid loc value"));
                    }

                    #[cfg(since_647)]
                    "memberop1" | "memberop2" | "memberop3" | "memberop4" | "memberop5" => {
                        let slot = key.as_bytes()["memberop".len()] - b'1';
                        client.p1(150 + slot);
                        client.pjstr(value);
                    }

                    #[cfg(since_647)]
                    "hillskew_t3_i32" => {
                        client.p1(162);
                        client.p4(value.parse::<i32>().expect("Invalid loc value"));
                    }

                    #[cfg(since_647)]
                    "op163" => {
                        let parts: Vec<&str> = value.split(',').collect();
                        if parts.len() == 4 {
                            client.p1(163);
                            for p in &parts {
                                client
                                    .p1(p.trim().parse::<i32>().expect("Invalid loc value") as i8
                                        as u8);
                            }
                        }
                    }

                    #[cfg(since_647)]
                    "op164" => {
                        client.p1(164);
                        client.p2(value.parse::<i32>().expect("Invalid loc value") as i16 as u16);
                    }

                    #[cfg(since_647)]
                    "op165" => {
                        client.p1(165);
                        client.p2(value.parse::<i32>().expect("Invalid loc value") as i16 as u16);
                    }

                    #[cfg(since_647)]
                    "op166" => {
                        client.p1(166);
                        client.p2(value.parse::<i32>().expect("Invalid loc value") as i16 as u16);
                    }

                    #[cfg(since_647)]
                    "op167" => {
                        client.p1(167);
                        client.p2(value.parse::<u16>().expect("Invalid loc value"));
                    }

                    #[cfg(since_647)]
                    "op168" => parse_bool(value, |v| {
                        if v {
                            client.p1(168);
                        }
                    }),

                    #[cfg(since_647)]
                    "op169" => parse_bool(value, |v| {
                        if v {
                            client.p1(169);
                        }
                    }),

                    #[cfg(since_647)]
                    "op170" => {
                        client.p1(170);
                        let v = value.parse::<i32>().expect("Invalid loc value");
                        emit_smart_packed(&mut client, v);
                    }

                    #[cfg(since_647)]
                    "op171" => {
                        client.p1(171);
                        let v = value.parse::<i32>().expect("Invalid loc value");
                        emit_smart_packed(&mut client, v);
                    }

                    #[cfg(since_647)]
                    "op173" => {
                        if let Some((a_s, b_s)) = value.split_once(',') {
                            client.p1(173);
                            client.p2(a_s.trim().parse::<u16>().expect("Invalid loc value"));
                            client.p2(b_s.trim().parse::<u16>().expect("Invalid loc value"));
                        }
                    }

                    #[cfg(since_647)]
                    "op177" => parse_bool(value, |v| {
                        if v {
                            client.p1(177);
                        }
                    }),

                    #[cfg(since_647)]
                    "op178" => {
                        client.p1(178);
                        client.p1(value.parse::<u8>().expect("Invalid loc value"));
                    }

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
            #[cfg(all(since_289, before_647))]
            if let Some(varbit_name) = multivar {
                let varbit_id = registry
                    .varbit
                    .get_by_debugname(varbit_name)
                    .unwrap_or_else(|| {
                        panic!("{debugname}: unknown multivar varbit '{varbit_name}'")
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
        let params: Vec<_> = props
            .iter()
            .filter(|(key, _)| key == "param")
            .cloned()
            .collect();
        #[cfg(since_647)]
        let params: Vec<_> = params
            .into_iter()
            .map(|(key, value)| {
                let (param, val) = value.split_once(',').expect("Invalid loc param");
                let val = val.trim();
                let ty = param_types.get(param.trim()).map_or("int", String::as_str);
                let id = if matches!(ty, "int" | "loc") && val.parse::<i32>().is_err() {
                    loc_ids.get(val).copied().or_else(|| {
                        if ty == "int" {
                            registry
                                .obj
                                .get_by_debugname(val)
                                .or_else(|| registry.seq.get_by_debugname(val))
                                .or_else(|| registry.npc.get_by_debugname(val))
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };
                (key, id.map_or(value.clone(), |id| format!("{param},{id}")))
            })
            .collect();
        parse_params(registry, param_types, &mut server, &params, debugname);

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

fn batch_code(key: &str) -> u8 {
    #[cfg(since_647)]
    {
        if config_key_index(key, "model").is_some() {
            return 1;
        }
        if config_key_index(key, "model_sd").is_some()
            || config_key_index(key, "model_hd").is_some()
        {
            return 5;
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
        if config_key_index(key, "random_anim").is_some() {
            return 106;
        }
        if config_key_index(key, "quest").is_some() {
            return 160;
        }
        if matches!(key, "multivar" | "multiloc") {
            return 77;
        }
        if matches!(key, "paramint" | "paramstr") {
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
        1 | 5 => {
            packed.p1(code);
            if code == 1 {
                pack_models(packed, props, "model", registry);
            } else {
                pack_models(packed, props, "model_sd", registry);
                pack_models(packed, props, "model_hd", registry);
            }
        }
        40 | 41 => pack_indexed_pairs(packed, code, props, 's'),
        42 | 106 | 160 => {
            let prefix = match code {
                42 => "recolpalette",
                106 => "random_anim",
                _ => "quest",
            };
            let values: BTreeMap<_, _> = props
                .iter()
                .map(|(key, value)| {
                    (
                        config_key_index(key, prefix).expect("Invalid loc value index"),
                        value,
                    )
                })
                .collect();
            packed.p1(code);
            packed.p1(u8::try_from(values.len()).expect("Too many loc values"));
            for value in values.into_values() {
                match code {
                    42 => parse_number(value, |v: i8| packed.p1(v as u8)),
                    160 => parse_number(value, |v| packed.p2(v)),
                    106 => {
                        let (seq, weight) = value.split_once(',').expect("Invalid random anim");
                        parse_seq(registry, seq.trim(), |v| packed.p2(v));
                        parse_number(weight.trim(), |v| packed.p1(v));
                    }
                    _ => unreachable!(),
                }
            }
        }
        77 => {
            let mut varbit = u16::MAX;
            let mut varp = u16::MAX;
            let mut default = None;
            let mut locs = BTreeMap::new();
            for (key, value) in props {
                if key == "multivar" {
                    let bit = registry
                        .varbit
                        .get_by_debugname(value)
                        .or_else(|| value.strip_prefix("varbit_")?.parse().ok());
                    let var = registry
                        .varp
                        .get_by_debugname(value)
                        .or_else(|| value.strip_prefix("varp_")?.parse().ok());
                    match (bit, var) {
                        (Some(id), None) => varbit = u16::try_from(id)?,
                        (None, Some(id)) => varp = u16::try_from(id)?,
                        (Some(_), Some(_)) => bail!("{name}: ambiguous multivar '{value}'"),
                        (None, None) => bail!("{name}: unknown multivar '{value}'"),
                    }
                } else {
                    let (index, loc) = value
                        .split_once(',')
                        .with_context(|| format!("{name}: invalid multiloc '{value}'"))?;
                    let loc = loc.trim();
                    let id = if loc == "null" {
                        u16::MAX
                    } else {
                        u16::try_from(
                            registry
                                .loc
                                .get_by_debugname(loc)
                                .or_else(|| loc.strip_prefix("loc_")?.parse().ok())
                                .with_context(|| format!("{name}: unknown multiloc '{loc}'"))?,
                        )?
                    };
                    if index.trim() == "default" {
                        default = Some(id);
                    } else {
                        locs.insert(index.trim().parse::<u8>()?, id);
                    }
                }
            }
            packed.p1(if default.is_some() { 92 } else { 77 });
            packed.p2(varbit);
            packed.p2(varp);
            if let Some(default) = default {
                packed.p2(default);
            }
            let count = locs.last_key_value().map_or(0, |(&index, _)| index);
            packed.p1(count);
            for index in 0..=count {
                packed.p2(locs.get(&index).copied().unwrap_or(u16::MAX));
            }
        }
        249 => parse_params(registry, param_types, packed, props, name),
        _ => unreachable!(),
    }
    Ok(())
}

#[cfg(since_647)]
fn pack_models(
    packed: &mut PackedData,
    props: &[(String, String)],
    prefix: &str,
    registry: &PackRegistry,
) {
    let models: BTreeMap<_, _> = props
        .iter()
        .filter_map(|(key, value)| Some((config_key_index(key, prefix)?, value)))
        .collect();
    packed.p1(u8::try_from(models.len()).expect("Too many model shapes"));
    for value in models.into_values() {
        let (shape, models) = value.split_once(',').expect("Invalid model group");
        parse_number(shape.trim(), |v: i8| packed.p1(v as u8));
        let models: Vec<_> = models
            .split(';')
            .filter(|model| !model.is_empty())
            .collect();
        packed.p1(u8::try_from(models.len()).expect("Too many models"));
        for model in models {
            parse_model(registry, model.trim(), |v| packed.p2(v));
        }
    }
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
    use crate::unpack::config::{UnpackedPacks, loc::emit_loc_props};
    use rs_io::Packet;
    use std::collections::HashMap;
    use std::{env, fs, process};

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
            for (key, value) in emit_loc_props(&def, &mut UnpackedPacks::new(), &HashMap::new()) {
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
