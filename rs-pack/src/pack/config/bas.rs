use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{parse_number, parse_numbers, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

fn anim_code(key: &str) -> Option<u8> {
    Some(match key {
        "crawlanim" => 2,
        "crawlanim_b" => 3,
        "crawlanim_l" => 4,
        "crawlanim_r" => 5,
        "runanim" => 6,
        "runanim_b" => 7,
        "runanim_l" => 8,
        "runanim_r" => 9,
        "readyanim_l" => 38,
        "readyanim_r" => 39,
        "walkanim_b" => 40,
        "walkanim_l" => 41,
        "walkanim_r" => 42,
        "unknown43" => 43,
        "unknown44" => 44,
        "crawlturn_l" => 46,
        "crawlturn_r" => 47,
        "runturn_l" => 48,
        "runturn_r" => 49,
        "walkturn_l" => 50,
        "walkturn_r" => 51,
        _ => return None,
    })
}

pub fn pack_bases(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let seq = &registry.seq;
    client_config_table(
        file_cache,
        "bas",
        &registry.bas,
        constants,
        |client, _, props| {
            let mut wrote_random = false;
            for (key, value) in props {
                if let Some(code) = anim_code(key) {
                    parse_ref(seq, "seq", value, |v| {
                        client.p1(code);
                        client.p2(v);
                    });
                    continue;
                }
                match key.as_str() {
                    // 1
                    "readyanim" => {
                        let (ready, walk) = value.split_once(',').expect("Invalid bas readyanim");
                        client.p1(1);
                        parse_ref(seq, "seq", ready, |v| client.p2(v));
                        parse_ref(seq, "seq", walk, |v| client.p2(v));
                    }

                    // 26
                    "hillrotate" => {
                        let [x, z] = parse_numbers::<u8, 2>(value);
                        client.p1(26);
                        client.p1(x);
                        client.p1(z);
                    }

                    // 27
                    "transform" => {
                        let [slot, a, b, c, d, e, f] = parse_numbers::<i32, 7>(value);
                        client.p1(27);
                        client.p1(slot as u8);
                        for v in [a, b, c, d, e, f] {
                            client.p2(v as u16);
                        }
                    }

                    // 28
                    "unknown28" => {
                        let values: Vec<u8> = value
                            .split(',')
                            .map(|v| v.parse().expect("Invalid bas unknown28"))
                            .collect();
                        client.p1(28);
                        client.p1(values.len() as u8);
                        for v in values {
                            client.p1(v);
                        }
                    }

                    // 29
                    "turnspeed" => parse_number(value, |v: u8| {
                        client.p1(29);
                        client.p1(v);
                    }),

                    // 30
                    "turnmax" => parse_number(value, |v: u16| {
                        client.p1(30);
                        client.p2(v);
                    }),

                    // 31
                    "rollspeed" => parse_number(value, |v: u8| {
                        client.p1(31);
                        client.p1(v);
                    }),

                    // 32
                    "rollmax" => parse_number(value, |v: u16| {
                        client.p1(32);
                        client.p2(v);
                    }),

                    // 33
                    "roll" => parse_number(value, |v: i16| {
                        client.p1(33);
                        client.p2(v as u16);
                    }),

                    // 34
                    "pitchspeed" => parse_number(value, |v: u8| {
                        client.p1(34);
                        client.p1(v);
                    }),

                    // 35
                    "pitchmax" => parse_number(value, |v: u16| {
                        client.p1(35);
                        client.p2(v);
                    }),

                    // 36
                    "pitch" => parse_number(value, |v: i16| {
                        client.p1(36);
                        client.p2(v as u16);
                    }),

                    // 37
                    "walkspeed" => parse_number(value, |v: u8| {
                        client.p1(37);
                        client.p1(v);
                    }),

                    "overlayheight" => parse_number(value, |v: u16| {
                        client.p1(45);
                        client.p2(v);
                    }),

                    // 52
                    "randomreadyanim" => {
                        if !wrote_random {
                            wrote_random = true;
                            let rows: Vec<_> = props
                                .iter()
                                .filter(|(k, _)| k == "randomreadyanim")
                                .map(|(_, v)| v)
                                .collect();
                            client.p1(52);
                            client.p1(rows.len() as u8);
                            for row in rows {
                                let (anim, weight) =
                                    row.split_once(',').expect("Invalid bas readyanim_random");
                                parse_ref(seq, "seq", anim, |v| client.p2(v));
                                parse_number(weight, |v: u8| client.p1(v));
                            }
                        }
                    }

                    // 53
                    "unknown53" => client.p1(53),

                    // 54
                    "hillrotatelimit" => {
                        let [a, b] = parse_numbers::<u8, 2>(value);
                        client.p1(54);
                        client.p1(a);
                        client.p1(b);
                    }

                    // 55
                    "unknown55" => {
                        let [slot, v] = parse_numbers::<u16, 2>(value);
                        client.p1(55);
                        client.p1(slot as u8);
                        client.p2(v);
                    }

                    // 56
                    "slotoffset" => {
                        let [slot, x, y, z] = parse_numbers::<i32, 4>(value);
                        client.p1(56);
                        client.p1(slot as u8);
                        for v in [x, y, z] {
                            client.p2(v as u16);
                        }
                    }

                    // not found
                    _ => panic!("Unrecognized bas config key: {key}"),
                }
            }
        },
    )
}
