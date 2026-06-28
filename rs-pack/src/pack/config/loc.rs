use std::collections::HashMap;

use crate::config_crc;
use crate::pack::config::param::parse_params;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::{RecolType, rgb15_to_hsl16};
use crate::pack::util::{
    parse_bool, parse_category, parse_number, parse_recol, parse_retex, parse_seq,
};
use crate::types::{ForceApproach, LocShape};
use anyhow::Result;
use rs_io::crc;
use tracing::debug;

struct LocModelShape {
    model: u16,
    shape: u8,
}

pub fn pack_locs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.loc;

    let files = file_cache.collect("loc");
    debug!("  Found {} .loc files", files.len());

    let configs = parse_config_sections_cached(file_cache, "loc", constants);
    debug!("  Parsed {} loc configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown loc id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown loc config: {debugname}");
        };

        let mut src_models: Vec<&str> = Vec::new();
        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        let mut recol_d: Vec<(usize, u16)> = Vec::new();
        let mut active = -1;
        let mut name = None;
        let mut desc = None;

        for (key, value) in props {
            match key.as_str() {
                // 1
                _ if key.starts_with("model") => src_models.push(value),

                // 2
                "name" => name = Some(value.to_string()),

                // 3
                "desc" => desc = Some(value),

                // 14
                "width" => parse_number(value, |v| {
                    client.p1(14);
                    client.p1(v);
                    server.p1(14);
                    server.p1(v);
                }),

                // 15
                "length" => parse_number(value, |v| {
                    client.p1(15);
                    client.p1(v);
                    server.p1(15);
                    server.p1(v);
                }),

                // 17
                "blockwalk" => parse_bool(value, |v| {
                    if !v {
                        client.p1(17);
                        server.p1(17);
                    }
                }),

                // 18
                "blockrange" => parse_bool(value, |v| {
                    if !v {
                        client.p1(18);
                        server.p1(18);
                    }
                }),

                // 19
                "active" => parse_bool(value, |v| {
                    client.p1(19);
                    client.p1(v as u8);
                    server.p1(19);
                    server.p1(v as u8);
                    active = v as i8;
                }),

                // 21
                "hillskew" => parse_bool(value, |v| {
                    if v {
                        client.p1(21);
                        server.p1(21);
                    }
                }),

                // 22
                "sharelight" => parse_bool(value, |v| {
                    if v {
                        client.p1(22);
                        server.p1(22);
                    }
                }),

                // 23
                "occlude" => parse_bool(value, |v| {
                    if v {
                        client.p1(23);
                        server.p1(23);
                    }
                }),

                // 24
                "anim" => parse_seq(registry, value, |v| {
                    client.p1(24);
                    client.p2(v);
                    server.p1(24);
                    server.p2(v);
                }),

                // 25
                "hasalpha" => parse_bool(value, |v| {
                    if v {
                        client.p1(25);
                        server.p1(25);
                    }
                }),

                // 28
                "wallwidth" => parse_number(value, |v| {
                    client.p1(28);
                    client.p1(v);
                    server.p1(28);
                    server.p1(v);
                }),

                // 29
                "ambient" => parse_number(value, |v: i8| {
                    client.p1(29);
                    client.p1(v as u8);
                    server.p1(29);
                    server.p1(v as u8);
                }),

                // 39
                "contrast" => parse_number(value, |v: i8| {
                    client.p1(39);
                    client.p1(v as u8);
                    server.p1(39);
                    server.p1(v as u8);
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
                    RecolType::S(idx, v) => recol_s.push((idx, rgb15_to_hsl16(v))),
                    RecolType::D(idx, v) => recol_d.push((idx, rgb15_to_hsl16(v))),
                }),
                // retextures stored in recol until rev 465
                _ if key.starts_with("retex") => parse_retex(registry, key, value, |v| match v {
                    RecolType::S(idx, v) => recol_s.push((idx, v)),
                    RecolType::D(idx, v) => recol_d.push((idx, v)),
                }),

                // 60
                "mapfunction" => parse_number(value, |v| {
                    client.p1(60);
                    client.p2(v);
                    server.p1(60);
                    server.p2(v);
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
                        server.p1(62);
                    }
                }),

                // 64
                "shadow" => parse_bool(value, |v| {
                    if !v {
                        client.p1(64);
                        server.p1(64);
                    }
                }),

                // 65
                "resizex" => parse_number(value, |v| {
                    client.p1(65);
                    client.p2(v);
                    server.p1(65);
                    server.p2(v);
                }),

                // 66
                "resizey" => parse_number(value, |v| {
                    client.p1(66);
                    client.p2(v);
                    server.p1(66);
                    server.p2(v);
                }),

                // 67
                "resizez" => parse_number(value, |v| {
                    client.p1(67);
                    client.p2(v);
                    server.p1(67);
                    server.p2(v);
                }),

                // 68
                "mapscene" => parse_number(value, |v| {
                    client.p1(68);
                    client.p2(v);
                    server.p1(68);
                    server.p2(v);
                }),

                // 69
                "forceapproach" => {
                    let val = ForceApproach::from_config_str(value);
                    client.p1(69);
                    client.p1(val as u8);
                    server.p1(69);
                    server.p1(val as u8);
                }

                // 70
                "offsetx" => parse_number(value, |v| {
                    client.p1(70);
                    client.p2(v);
                    server.p1(70);
                    server.p2(v);
                }),

                // 71
                "offsety" => parse_number(value, |v| {
                    client.p1(71);
                    client.p2(v);
                    server.p1(71);
                    server.p2(v);
                }),

                // 72
                "offsetz" => parse_number(value, |v| {
                    client.p1(72);
                    client.p2(v);
                    server.p1(72);
                    server.p2(v);
                }),

                // 73
                "forcedecor" => parse_bool(value, |v| {
                    if v {
                        client.p1(73);
                        server.p1(73);
                    }
                }),

                // 74
                #[cfg(since_245_2)]
                "breakroutefinding" => parse_bool(value, |v| {
                    if v {
                        client.p1(74);
                        server.p1(74);
                    }
                }),

                // 75
                #[cfg(since_254)]
                "raiseobject" => parse_bool(value, |v| {
                    client.p1(75);
                    client.p1(v as u8);
                    server.p1(75);
                    server.p1(v as u8);
                }),

                // 249
                "param" => {} // handled at the end

                // not found
                _ => panic!("Unrecognized loc config key: {key}"),
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
                client.p2(s);
                client.p2(d);
                server.p2(s);
                server.p2(d);
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

            // centrepiece_straight (_8) comes first
            if let Some(id) = try_model(&format!("{src}{}", LocShape::CentrepieceStraight.suffix()))
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

        #[cfg(before_254)]
        if !models.is_empty() {
            client.p1(1);
            client.p1(models.len() as u8);
            server.p1(1);
            server.p1(models.len() as u8);
            for m in &models {
                client.p2(m.model);
                client.p1(m.shape);
                server.p2(m.model);
                server.p1(m.shape);
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
                server.p1(5);
                server.p1(models.len() as u8);
                for m in &models {
                    client.p2(m.model);
                    server.p2(m.model);
                }
            } else {
                client.p1(1);
                client.p1(models.len() as u8);
                server.p1(1);
                server.p1(models.len() as u8);
                for m in &models {
                    client.p2(m.model);
                    client.p1(m.shape);
                    server.p2(m.model);
                    server.p1(m.shape);
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
            server.p1(2);
            server.pjstr(name);
        }

        // handle 3
        if let Some(desc) = desc {
            client.p1(3);
            client.pjstr(desc);
            server.p1(3);
            server.pjstr(desc);
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
        let expected = config_crc::LOC;
        if crc != expected {
            panic!("CRC mismatch ['loc']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
