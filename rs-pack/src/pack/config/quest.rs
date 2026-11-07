use super::client_config_table;
use crate::pack::config::param::parse_params;
use crate::pack::pack::{ConfigProperties, FileCache};
use crate::pack::pack_registry::{PackFile, PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_coord, parse_number, parse_numbers, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

fn rows<'a>(props: &'a ConfigProperties, key: &str) -> Vec<&'a str> {
    props
        .iter()
        .filter(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .collect()
}

fn write_progress(client: &mut PackedData, pack: &PackFile, kind: &str, code: u8, rows: Vec<&str>) {
    client.p1(code);
    client.p1(rows.len() as u8);
    for row in rows {
        let (var, range) = row.split_once(',').expect("Invalid quest progress");
        parse_ref(pack, kind, var, |v| client.p2(v));
        let [start, end] = parse_numbers::<i32, 2>(range);
        client.p4(start);
        client.p4(end);
    }
}

fn write_var_requirements(
    client: &mut PackedData,
    pack: &PackFile,
    kind: &str,
    code: u8,
    rows: Vec<&str>,
) {
    client.p1(code);
    client.p1(rows.len() as u8);
    for row in rows {
        let mut parts = row.splitn(4, ',');
        let var = parts.next().expect("Missing quest requirement var");
        match var.parse::<i32>() {
            Ok(id) => client.p4(id),
            Err(_) => parse_ref(pack, kind, var, |v| client.p4(v as i32)),
        }
        for _ in 0..2 {
            let bound = parts.next().expect("Missing quest requirement bound");
            parse_number(bound, |v: i32| client.p4(v));
        }
        client.pjstr(parts.next().expect("Missing quest requirement text"));
    }
}

pub fn pack_quests(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "quest",
        &registry.quest,
        constants,
        |client, name, props| {
            let mut written: Vec<&str> = Vec::new();
            for (key, value) in props {
                let first = !written.contains(&key.as_str());
                if first {
                    written.push(key);
                }
                match key.as_str() {
                    // 1
                    "name" => {
                        client.p1(1);
                        client.p1(0);
                        client.pjstr(value);
                    }

                    // 2
                    "sortname" => {
                        client.p1(2);
                        client.p1(0);
                        client.pjstr(value);
                    }

                    // 3
                    "masterquestvarp" if first => {
                        write_progress(client, &registry.varp, "varp", 3, rows(props, key));
                    }

                    // 4
                    "masterquestvarbit" if first => {
                        write_progress(client, &registry.varbit, "varbit", 4, rows(props, key));
                    }

                    // 5
                    "parent" => parse_ref(&registry.quest, "quest", value, |v| {
                        client.p1(5);
                        client.p2(v);
                    }),

                    // 6
                    "type" => {
                        client.p1(6);
                        client.p1(match value.as_str() {
                            "normal" => 0,
                            "seasonal" => 1,
                            other => other.parse().expect("Invalid quest type"),
                        });
                    }

                    // 7
                    "difficulty" => {
                        client.p1(7);
                        client.p1(match value.as_str() {
                            "novice" => 0,
                            "intermediate" => 1,
                            "experienced" => 2,
                            "master" => 3,
                            "grandmaster" => 4,
                            other => other.parse().expect("Invalid quest difficulty"),
                        });
                    }

                    // 8
                    "members" => client.p1(8),

                    // 9
                    "questpoints" => parse_number(value, |v: u8| {
                        client.p1(9);
                        client.p1(v);
                    }),

                    // 10
                    "mainstartcoord" => {
                        let values: Vec<i32> = value
                            .split(',')
                            .map(|v| parse_coord(v).expect("Invalid quest mainstartcoord"))
                            .collect();
                        client.p1(10);
                        client.p1(values.len() as u8);
                        for v in values {
                            client.p4(v);
                        }
                    }

                    // 12
                    "unknown12" => parse_number(value, |v: i32| {
                        client.p1(12);
                        client.p4(v);
                    }),

                    // 13
                    "questreq" if first => {
                        let rows = rows(props, key);
                        client.p1(13);
                        client.p1(rows.len() as u8);
                        for row in rows {
                            parse_ref(&registry.quest, "quest", row, |v| client.p2(v));
                        }
                    }

                    // 14
                    "statreq" if first => {
                        let rows = rows(props, key);
                        client.p1(14);
                        client.p1(rows.len() as u8);
                        for row in rows {
                            let (stat, level) =
                                row.split_once(',').expect("Invalid quest stat requirement");
                            parse_ref(&registry.stat, "stat", stat, |v| client.p1(v as u8));
                            parse_number(level, |v: u8| client.p1(v));
                        }
                    }

                    // 15
                    "pointsreq" => parse_number(value, |v: u16| {
                        client.p1(15);
                        client.p2(v);
                    }),

                    // 17
                    "icon" => parse_ref(&registry.sprites, "sprites", value, |v| {
                        client.p1(17);
                        client.p2(v);
                    }),

                    // 18
                    "varpreq" if first => {
                        let rows = rows(props, key);
                        write_var_requirements(client, &registry.varp, "varp", 18, rows);
                    }

                    // 19
                    "varbitreq" if first => {
                        let rows = rows(props, key);
                        write_var_requirements(client, &registry.varbit, "varbit", 19, rows);
                    }

                    // 249
                    "param" if first => parse_params(registry, param_types, client, props, name),

                    // later rows of a repeated key
                    "masterquestvarp" | "masterquestvarbit" | "questreq" | "statreq"
                    | "varpreq" | "varbitreq" | "param" => {}

                    // not found
                    _ => panic!("Unrecognized quest config key: {key}"),
                }
            }
        },
    )
}
