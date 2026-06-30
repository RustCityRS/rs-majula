use std::collections::HashMap;

use crate::config_crc;
use crate::pack::config::param::parse_params;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::{RecolType, rgb15_to_hsl16};
use crate::pack::util::*;
use crate::types::{DummyItem, WearPos};
use anyhow::Result;
use rs_io::crc;
use tracing::debug;

pub fn pack_objs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.obj;

    let files = file_cache.collect("obj");
    debug!("  Found {} .obj files", files.len());

    let configs = parse_config_sections_cached(file_cache, "obj", constants);
    debug!("  Parsed {} obj configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown obj id: {id}");
        };

        let cert_props;
        let mut cert_props_owned;
        let props: &Vec<(String, String)> = if debugname.starts_with("cert_") {
            cert_props = vec![
                (
                    "certlink".to_string(),
                    debugname.strip_prefix("cert_").unwrap().to_string(),
                ),
                ("certtemplate".to_string(), "template_for_cert".to_string()),
            ];
            &cert_props
        } else {
            let Some(p) = configs.get(debugname) else {
                panic!("Unknown obj config: {debugname}");
            };

            let has_name = p.iter().any(|(k, _)| k == "name");
            let has_model = p.iter().any(|(k, _)| k == "model");

            if !has_name && has_model {
                let name = {
                    let mut chars = debugname.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().to_string() + &chars.as_str().replace('_', " "),
                    }
                };
                cert_props_owned = p.clone();
                cert_props_owned.push(("name".to_string(), name));
                &cert_props_owned
            } else {
                p
            }
        };

        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        let mut recol_d: Vec<(usize, u16)> = Vec::new();
        let mut name = None;

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
                "name" => name = Some(value),

                // 3
                "desc" => {
                    client.p1(3);
                    client.pjstr(value);
                    server.p1(3);
                    server.pjstr(value);
                }

                // 4
                "2dzoom" => parse_number(value, |v| {
                    client.p1(4);
                    client.p2(v);
                    server.p1(4);
                    server.p2(v);
                }),

                // 5
                "2dxan" => parse_number(value, |v| {
                    client.p1(5);
                    client.p2(v);
                    server.p1(5);
                    server.p2(v);
                }),

                // 6
                "2dyan" => parse_number(value, |v| {
                    client.p1(6);
                    client.p2(v);
                    server.p1(6);
                    server.p2(v);
                }),

                // 7
                "2dxof" => parse_number(value, |v: i32| {
                    client.p1(7);
                    client.p2(v as u16);
                    server.p1(7);
                    server.p2(v as u16);
                }),

                // 8
                "2dyof" => parse_number(value, |v: i32| {
                    client.p1(8);
                    client.p2(v as u16);
                    server.p1(8);
                    server.p2(v as u16);
                }),

                // 9
                "code9" => parse_bool(value, |v| {
                    if v {
                        client.p1(9);
                        server.p1(9);
                    }
                }),

                // 10
                "code10" => parse_seq(registry, value, |v| {
                    client.p1(10);
                    client.p2(v);
                    server.p1(10);
                    server.p2(v);
                }),

                // 11
                "stackable" => parse_bool(value, |v| {
                    if v {
                        client.p1(11);
                        server.p1(11);
                    }
                }),

                // 12
                "cost" => parse_number(value, |v| {
                    client.p1(12);
                    client.p4(v);
                    server.p1(12);
                    server.p4(v);
                }),

                // 13
                "wearpos" => {
                    let pos = WearPos::from_config_str(value) as u8;
                    server.p1(13);
                    server.p1(pos);
                }

                // 14
                "wearpos2" => {
                    let pos = WearPos::from_config_str(value) as u8;
                    server.p1(14);
                    server.p1(pos);
                }

                // 15
                "tradeable" => parse_bool(value, |v| {
                    if !v {
                        server.p1(15);
                    }
                }),

                // 16
                "members" => parse_bool(value, |v| {
                    if v {
                        client.p1(16);
                        server.p1(16);
                    }
                }),

                // 23
                "manwear" => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() != 2 {
                        panic!("Invalid manwear value: {key}={value}");
                    }
                    parse_model(registry, parts[0], |v| {
                        parse_number(parts[1], |x| {
                            client.p1(23);
                            client.p2(v);
                            client.p1(x);
                            server.p1(23);
                            server.p2(v);
                            server.p1(x);
                        });
                    });
                }

                // 24
                "manwear2" => parse_model(registry, value, |v| {
                    client.p1(24);
                    client.p2(v);
                    server.p1(24);
                    server.p2(v);
                }),

                // 25
                "womanwear" => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() != 2 {
                        panic!("Invalid womanwear value: {key}={value}");
                    }
                    parse_model(registry, parts[0], |v| {
                        parse_number(parts[1], |x| {
                            client.p1(25);
                            client.p2(v);
                            client.p1(x);
                            server.p1(25);
                            server.p2(v);
                            server.p1(x);
                        });
                    });
                }

                // 26
                "womanwear2" => parse_model(registry, value, |v| {
                    client.p1(26);
                    client.p2(v);
                    server.p1(26);
                    server.p2(v);
                }),

                // 27
                "wearpos3" => {
                    let pos = WearPos::from_config_str(value) as u8;
                    server.p1(27);
                    server.p1(pos);
                }

                // 30-34
                "op1" | "op2" | "op3" | "op4" | "op5" => parse_number(&key[2..], |v: u8| {
                    client.p1(29 + v);
                    client.pjstr(value);
                    server.p1(29 + v);
                    server.pjstr(value);
                }),

                // 35-39
                "iop1" | "iop2" | "iop3" | "iop4" | "iop5" => parse_number(&key[3..], |v: u8| {
                    client.p1(34 + v);
                    client.pjstr(value);
                    server.p1(34 + v);
                    server.pjstr(value);
                }),

                // 40
                _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                    RecolType::S(idx, v) => recol_s.push((idx, v)),
                    RecolType::D(idx, v) => recol_d.push((idx, v)),
                }),

                // 75
                "weight" => {
                    let grams: f64 = if let Some(v) = value.strip_suffix("kg") {
                        v.parse::<f64>().unwrap_or(f64::NAN) * 1000.0
                    } else if let Some(v) = value.strip_suffix("oz") {
                        v.parse::<f64>().unwrap_or(f64::NAN) * 28.3495
                    } else if let Some(v) = value.strip_suffix("lb") {
                        v.parse::<f64>().unwrap_or(f64::NAN) * 453.592
                    } else if let Some(v) = value.strip_suffix('g') {
                        v.parse::<f64>().unwrap_or(f64::NAN)
                    } else {
                        panic!("Invalid weight value: {value}");
                    };
                    if grams.is_nan() || !(-32768.0..=32767.0).contains(&grams) {
                        panic!("Weight out of range: {value}");
                    }
                    server.p1(75);
                    server.p2(grams as u16);
                }

                // 78
                "manwear3" => parse_model(registry, value, |v| {
                    client.p1(78);
                    client.p2(v);
                    server.p1(78);
                    server.p2(v);
                }),

                // 79
                "womanwear3" => parse_model(registry, value, |v| {
                    client.p1(79);
                    client.p2(v);
                    server.p1(79);
                    server.p2(v);
                }),

                // 90
                "manhead" => parse_model(registry, value, |v| {
                    client.p1(90);
                    client.p2(v);
                    server.p1(90);
                    server.p2(v);
                }),

                // 91
                "womanhead" => parse_model(registry, value, |v| {
                    client.p1(91);
                    client.p2(v);
                    server.p1(91);
                    server.p2(v);
                }),

                // 92
                "manhead2" => parse_model(registry, value, |v| {
                    client.p1(92);
                    client.p2(v);
                    server.p1(92);
                    server.p2(v);
                }),

                // 93
                "womanhead2" => parse_model(registry, value, |v| {
                    client.p1(93);
                    client.p2(v);
                    server.p1(93);
                    server.p2(v);
                }),

                // 94
                "category" => parse_category(registry, value, |v| {
                    server.p1(94);
                    server.p2(v);
                }),

                // 95
                "2dzan" => parse_number(value, |v| {
                    client.p1(95);
                    client.p2(v);
                    server.p1(95);
                    server.p2(v);
                }),

                // 96
                "dummyitem" => {
                    let dummy = DummyItem::from_config_str(value) as u8;
                    server.p1(96);
                    server.p1(dummy);
                }

                // 97
                "certlink" => parse_obj(registry, value, |v| {
                    client.p1(97);
                    client.p2(v);
                    server.p1(97);
                    server.p2(v);
                }),

                // 98
                "certtemplate" => parse_obj(registry, value, |v| {
                    client.p1(98);
                    client.p2(v);
                    server.p1(98);
                    server.p2(v);
                }),

                // 100-109
                _ if key.starts_with("count") => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() != 2 {
                        panic!("Invalid count value: {key}={value}");
                    }
                    parse_obj(registry, parts[0], |v| {
                        parse_number(parts[1], |x| {
                            if x == 0 {
                                panic!("Invalid count value: {key}={value}");
                            }
                            let Some(idx) = key.strip_prefix("count") else {
                                panic!("Invalid count value: {key}={value}");
                            };
                            parse_number(idx, |z: u8| {
                                client.p1(99 + z);
                                client.p2(v);
                                client.p2(x);
                                server.p1(99 + z);
                                server.p2(v);
                                server.p2(x);
                            });
                        });
                    });
                }

                // 110
                #[cfg(since_244)]
                "resizex" => parse_number(value, |v| {
                    client.p1(110);
                    client.p2(v);
                    server.p1(110);
                    server.p2(v);
                }),

                // 111
                #[cfg(since_244)]
                "resizey" => parse_number(value, |v| {
                    client.p1(111);
                    client.p2(v);
                    server.p1(111);
                    server.p2(v);
                }),

                // 112
                #[cfg(since_244)]
                "resizez" => parse_number(value, |v| {
                    client.p1(112);
                    client.p2(v);
                    server.p1(112);
                    server.p2(v);
                }),

                // 113
                #[cfg(since_244)]
                "ambient" => parse_number(value, |v: i8| {
                    client.p1(113);
                    client.p1(v as u8);
                    server.p1(113);
                    server.p1(v as u8);
                }),

                // 114
                #[cfg(since_244)]
                "contrast" => parse_number(value, |v: i8| {
                    client.p1(114);
                    client.p1(v as u8);
                    server.p1(114);
                    server.p1(v as u8);
                }),

                // 115
                #[cfg(since_289)]
                "team" => parse_number(value, |v| {
                    client.p1(115);
                    client.p1(v);
                    server.p1(115);
                    server.p1(v);
                }),

                // 201
                "respawnrate" => parse_number(value, |v| {
                    server.p1(201);
                    server.p2(v);
                }),

                // 249
                "param" => {} // handled at the end

                // not found
                _ => panic!("Unrecognized obj config key: {key}"),
            }
        }

        // reverse-lookup the certificate (so the server can find it quicker)
        if let Some(cert) = registry.obj.get_by_debugname(&format!("cert_{debugname}")) {
            server.p1(97);
            server.p2(cert);
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
        if let Some(name) = &name {
            client.p1(2);
            client.pjstr(name);
            server.p1(2);
            server.pjstr(name);
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
        let expected = config_crc::OBJ;
        if crc != expected {
            panic!("CRC mismatch ['obj']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
