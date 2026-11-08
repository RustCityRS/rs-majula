use std::collections::HashMap;

#[cfg(since_647)]
use super::mark_client_params;
use crate::pack::config::param::parse_params;
#[cfg(since_647)]
use crate::pack::pack::ConfigSections;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
#[cfg(since_647)]
use crate::pack::pack_registry::PackFile;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
#[cfg(before_647)]
use crate::pack::util::colour::{RecolType, rgb15_to_hsl16};
use crate::pack::util::*;
use crate::types::{DummyItem, WearPos};
use anyhow::Result;

pub fn pack_objs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.obj;

    let configs = parse_config_sections_cached(file_cache, "obj", constants)?;
    #[cfg(since_647)]
    let varobjs = parse_config_sections_cached(file_cache, "varobj", constants)?;

    #[cfg(since_647)]
    let client_params = file_cache
        .client_count("param", &registry.param, constants)?
        .unwrap_or(0);

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            #[cfg(before_647)]
            panic!("Unknown obj id: {id}");
            #[cfg(since_647)]
            {
                server.finish_entry();
                client.skip_entry();
                continue;
            }
        };

        #[cfg(since_647)]
        let Some(props) = configs.get(debugname) else {
            server.p1(250);
            server.pjstr(debugname);
            server.finish_entry();
            client.skip_entry();
            continue;
        };
        #[cfg(since_647)]
        let props = &mark_client_params(props, &registry.param, client_params);
        #[cfg(since_647)]
        let has_objvar = props.iter().any(|(key, _)| key == "objvar");
        #[cfg(before_647)]
        let mut cert_props_owned;
        #[cfg(before_647)]
        let cert_props;
        #[cfg(before_647)]
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

        #[cfg(before_647)]
        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        #[cfg(before_647)]
        let mut recol_d: Vec<(usize, u16)> = Vec::new();
        #[cfg(before_647)]
        let mut name = None;

        for group in props.chunk_by(|a, b| batch_code(&a.0) == batch_code(&b.0)) {
            #[cfg(since_647)]
            if batch_code(&group[0].0) != 0 {
                let start = client.dat.len();
                pack_batch(&mut client, group, registry, param_types, debugname);
                server.dat.extend_from_slice(&client.dat[start..]);
                continue;
            }
            for (key, value) in group {
                let start = client.dat.len();
                match key.as_str() {
                    // 1
                    "model" => parse_ref(&registry.model, "model", value, |v| {
                        client.p1(1);
                        client.p2(v);
                    }),

                    // 2
                    "name" => {
                        #[cfg(before_647)]
                        {
                            name = Some(value);
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
                            client.p1(3);
                            client.pjstr(value);
                        }
                        #[cfg(since_647)]
                        {
                            server.p1(3);
                            server.pjstr(value);
                        }
                    }

                    // 4
                    "2dzoom" => parse_number(value, |v| {
                        client.p1(4);
                        client.p2(v);
                    }),

                    // 5
                    "2dxan" => parse_number(value, |v| {
                        client.p1(5);
                        client.p2(v);
                    }),

                    // 6
                    "2dyan" => parse_number(value, |v| {
                        client.p1(6);
                        client.p2(v);
                    }),

                    // 7
                    "2dxof" => parse_number(value, |v: i32| {
                        client.p1(7);
                        client.p2(v as u16);
                    }),

                    // 8
                    "2dyof" => parse_number(value, |v: i32| {
                        client.p1(8);
                        client.p2(v as u16);
                    }),

                    // 9
                    #[cfg(before_647)]
                    "code9" => parse_bool(value, |v| {
                        if v {
                            client.p1(9);
                        }
                    }),

                    // 10
                    #[cfg(before_647)]
                    "code10" => parse_ref(&registry.seq, "seq", value, |v| {
                        client.p1(10);
                        client.p2(v);
                    }),

                    // 11
                    "stackable" => parse_bool(value, |v| {
                        #[cfg(since_647)]
                        let v = v && !has_objvar;
                        if v {
                            client.p1(11);
                        }
                    }),

                    // 12
                    "cost" => parse_number(value, |v| {
                        client.p1(12);
                        client.p4(v);
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
                        }
                    }),

                    // 17
                    #[cfg(since_647)]
                    "objvar" => {
                        pack_objvars(&mut server, value, &registry.varobj, &varobjs, debugname)
                    }

                    #[cfg(since_647)]
                    "multistack" => parse_ref(&registry.obj, "obj", value, |v| {
                        client.p1(18);
                        client.p2(v);
                    }),

                    // 23
                    "manwear" => {
                        let mut parts = value.split(',');
                        parse_ref(&registry.model, "model", parts.next().unwrap(), |v| {
                            client.p1(23);
                            client.p2(v);
                        });
                        #[cfg(before_647)]
                        parse_number(parts.next().expect("Missing wear offset"), |v| client.p1(v));
                        assert!(parts.next().is_none(), "Invalid manwear: {value}");
                    }

                    // 24
                    "manwear2" => parse_ref(&registry.model, "model", value, |v| {
                        client.p1(24);
                        client.p2(v);
                    }),

                    // 25
                    "womanwear" => {
                        let mut parts = value.split(',');
                        parse_ref(&registry.model, "model", parts.next().unwrap(), |v| {
                            client.p1(25);
                            client.p2(v);
                        });
                        #[cfg(before_647)]
                        parse_number(parts.next().expect("Missing wear offset"), |v| client.p1(v));
                        assert!(parts.next().is_none(), "Invalid womanwear: {value}");
                    }

                    // 26
                    "womanwear2" => parse_ref(&registry.model, "model", value, |v| {
                        client.p1(26);
                        client.p2(v);
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
                    }),

                    // 35-39
                    "iop1" | "iop2" | "iop3" | "iop4" | "iop5" => {
                        parse_number(&key[3..], |v: u8| {
                            client.p1(34 + v);
                            client.pjstr(value);
                        })
                    }

                    // 40
                    #[cfg(before_647)]
                    _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                        RecolType::S(idx, v) => recol_s.push((idx, v)),
                        RecolType::D(idx, v) => recol_d.push((idx, v)),
                    }),

                    #[cfg(since_647)]
                    "stockmarket" => parse_bool(value, |v| {
                        if v {
                            client.p1(65);
                        }
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
                            #[cfg(before_647)]
                            panic!("Invalid weight value: {value}");
                            #[cfg(since_647)]
                            value.parse::<f64>().expect("Invalid weight")
                        };
                        if grams.is_nan() || !(-32768.0..=32767.0).contains(&grams) {
                            panic!("Weight out of range: {value}");
                        }
                        server.p1(75);
                        // Signed (boots of lightness weigh -10lb): `as u16` would
                        // saturate a negative weight to 0.
                        server.p2(grams as i16 as u16);
                    }

                    // 78
                    "manwear3" => parse_ref(&registry.model, "model", value, |v| {
                        client.p1(78);
                        client.p2(v);
                    }),

                    // 79
                    "womanwear3" => parse_ref(&registry.model, "model", value, |v| {
                        client.p1(79);
                        client.p2(v);
                    }),

                    // 90
                    "manhead" => parse_ref(&registry.model, "model", value, |v| {
                        client.p1(90);
                        client.p2(v);
                    }),

                    // 91
                    "womanhead" => parse_ref(&registry.model, "model", value, |v| {
                        client.p1(91);
                        client.p2(v);
                    }),

                    // 92
                    "manhead2" => parse_ref(&registry.model, "model", value, |v| {
                        client.p1(92);
                        client.p2(v);
                    }),

                    // 93
                    "womanhead2" => parse_ref(&registry.model, "model", value, |v| {
                        client.p1(93);
                        client.p2(v);
                    }),

                    // 94
                    "category" => parse_ref(&registry.category, "category", value, |v| {
                        server.p1(94);
                        server.p2(v);
                    }),

                    // 95
                    "2dzan" => parse_number(value, |v| {
                        client.p1(95);
                        client.p2(v);
                    }),

                    // 96
                    #[cfg(since_647)]
                    "dummyitem" if value.parse::<u8>().is_ok() => parse_number(value, |v: u8| {
                        client.p1(96);
                        client.p1(v);
                    }),

                    "dummyitem" => {
                        let dummy = DummyItem::from_config_str(value) as u8;
                        server.p1(96);
                        server.p1(dummy);
                    }

                    // 97
                    "certlink" => parse_ref(&registry.obj, "obj", value, |v| {
                        client.p1(97);
                        client.p2(v);
                    }),

                    // 98
                    "certtemplate" => parse_ref(&registry.obj, "obj", value, |v| {
                        client.p1(98);
                        client.p2(v);
                    }),

                    // 100-109
                    _ if key.starts_with("count") => {
                        let parts: Vec<&str> = value.split(',').collect();
                        if parts.len() != 2 {
                            panic!("Invalid count value: {key}={value}");
                        }
                        parse_ref(&registry.obj, "obj", parts[0], |v| {
                            parse_number(parts[1], |x| {
                                if cfg!(before_647) && x == 0 {
                                    panic!("Invalid count value: {key}={value}");
                                }
                                let Some(idx) = key.strip_prefix("count") else {
                                    panic!("Invalid count value: {key}={value}");
                                };
                                parse_number(idx, |z: u8| {
                                    client.p1(99 + z);
                                    client.p2(v);
                                    client.p2(x);
                                });
                            });
                        });
                    }

                    // 110
                    #[cfg(since_244)]
                    "resizex" => parse_number(value, |v| {
                        client.p1(110);
                        client.p2(v);
                    }),

                    // 111
                    #[cfg(since_244)]
                    "resizey" => parse_number(value, |v| {
                        client.p1(111);
                        client.p2(v);
                    }),

                    // 112
                    #[cfg(since_244)]
                    "resizez" => parse_number(value, |v| {
                        client.p1(112);
                        client.p2(v);
                    }),

                    // 113
                    #[cfg(since_244)]
                    "ambient" => parse_number(value, |v: i8| {
                        client.p1(113);
                        client.p1(v as u8);
                    }),

                    // 114
                    #[cfg(since_244)]
                    "contrast" => parse_number(value, |v: i8| {
                        client.p1(114);
                        client.p1(v as u8);
                    }),

                    // 115
                    #[cfg(since_289)]
                    "team" => parse_number(value, |v| {
                        client.p1(115);
                        client.p1(v);
                    }),

                    #[cfg(since_647)]
                    "lentlink" => parse_ref(&registry.obj, "obj", value, |v| {
                        client.p1(121);
                        client.p2(v);
                    }),

                    #[cfg(since_647)]
                    "lenttemplate" => parse_ref(&registry.obj, "obj", value, |v| {
                        client.p1(122);
                        client.p2(v);
                    }),

                    #[cfg(since_647)]
                    "manwearoff" | "womanwearoff" => {
                        client.p1(if key == "manwearoff" { 125 } else { 126 });
                        for v in parse_numbers::<i8, 3>(value) {
                            client.p1(v as u8);
                        }
                    }

                    #[cfg(since_647)]
                    "cursor1" | "cursor2" | "icursor1" | "icursor2" => {
                        let (op, cursor) = value.split_once(',').expect("Invalid obj cursor");
                        let op = op.trim().parse::<u8>().expect("Invalid obj cursor") - 1;
                        let op = op.to_string();
                        let op = op.as_str();
                        client.p1(match key.as_str() {
                            "cursor1" => 127,
                            "cursor2" => 128,
                            "icursor1" => 129,
                            _ => 130,
                        });
                        parse_number(op.trim(), |v| client.p1(v));
                        parse_ref(&registry.cursor, "cursor", cursor.trim(), |v| client.p2(v));
                    }

                    #[cfg(since_647)]
                    "picksizeshift" => parse_number(value, |v| {
                        client.p1(134);
                        client.p1(v);
                    }),

                    #[cfg(since_647)]
                    "boughtlink" => parse_ref(&registry.obj, "obj", value, |v| {
                        client.p1(139);
                        client.p2(v);
                    }),

                    #[cfg(since_647)]
                    "boughttemplate" => parse_ref(&registry.obj, "obj", value, |v| {
                        client.p1(140);
                        client.p2(v);
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
                server.dat.extend_from_slice(&client.dat[start..]);
            }
        }

        #[cfg(before_647)]
        {
            // reverse-lookup the certificate (so the server can find it quicker)
            if let Some(cert) = registry.obj.get_by_debugname(&format!("cert_{debugname}")) {
                server.p1(97);
                server.p2(cert);
            }

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
            if let Some(name) = &name {
                client.p1(2);
                client.pjstr(name);
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

fn batch_code(key: &str) -> u8 {
    #[cfg(since_647)]
    {
        if config_key_index(key, "recolpalette").is_some() {
            return 42;
        }
        if key.starts_with("recol") {
            return 40;
        }
        if key.starts_with("retex") {
            return 41;
        }
        if key == "quest" {
            return 132;
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
) {
    match batch_code(&props[0].0) {
        40 => pack_recol_pairs(packed, props),
        41 => pack_retex_pairs(packed, props, registry),
        42 => pack_indexed_values(packed, 42, props, "recolpalette", |packed, value| {
            parse_number(value, |v: i8| packed.p1(v as u8))
        }),
        132 => {
            packed.p1(132);
            packed.p1(u8::try_from(props.len()).expect("Too many obj quests"));
            for (_, value) in props {
                parse_number(value, |v| packed.p2(v));
            }
        }
        249 => parse_params(registry, param_types, packed, props, name),
        _ => unreachable!(),
    }
}

/// The varobjs an obj carries. Each must have its bits, apart from the others'.
#[cfg(since_647)]
fn pack_objvars(
    server: &mut PackedData,
    value: &str,
    pack: &PackFile,
    varobjs: &ConfigSections,
    debugname: &str,
) {
    let names: Vec<&str> = value.split(',').map(str::trim).collect();
    let mut ranges: Vec<(u8, u8)> = Vec::with_capacity(names.len());
    server.p1(17);
    server.p1(u8::try_from(names.len()).expect("Too many obj varobjs"));
    for name in names {
        let bit = |key: &str| {
            let props = varobjs.get(name)?;
            props.iter().find(|(k, _)| k == key)?.1.parse::<u8>().ok()
        };
        let (Some(start), Some(end)) = (bit("startbit"), bit("endbit")) else {
            panic!("[{debugname}]: varobj {name} has no startbit and endbit");
        };
        if let Some((s, e)) = ranges.iter().find(|&&(s, e)| start <= e && s <= end) {
            panic!("[{debugname}]: varobj {name}'s bits {start}..={end} overlap {s}..={e}");
        }
        ranges.push((start, end));
        parse_ref(pack, "varobj", name, |id| server.p2(id));
    }
}
