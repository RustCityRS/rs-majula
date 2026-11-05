use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::RecolType;
#[cfg(before_647)]
use crate::pack::util::colour::rgb15_to_hsl16;
#[cfg(since_647)]
use crate::pack::util::{pack_pairs, parse_retex};
use crate::pack::util::{parse_bool, parse_model_kind, parse_recol};
use crate::types::BodyType;
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_idks(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.idk;

    let configs = parse_config_sections_cached(file_cache, "idk", constants);

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some((debugname, props)) = config_entry("idk", pack, &configs, id) else {
            server.finish_entry();
            client.finish_entry();
            continue;
        };

        let mut models: Vec<(usize, u16)> = Vec::new();
        #[cfg(before_647)]
        let mut head_models: Vec<(usize, u16)> = Vec::new();
        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        let mut recol_d: Vec<(usize, u16)> = Vec::new();

        #[cfg(since_647)]
        let mut retex_s = Vec::new();
        #[cfg(since_647)]
        let mut retex_d = Vec::new();
        #[cfg(since_647)]
        let mut order = Vec::new();

        for (key, value) in props {
            #[cfg(since_647)]
            if let Some(code) = if key.starts_with("model") {
                Some(2)
            } else if key.starts_with("recol") {
                Some(40)
            } else if key.starts_with("retex") {
                Some(41)
            } else {
                None
            } && !order.contains(&code)
            {
                order.push(code);
            }
            match key.as_str() {
                // 1
                "type" => {
                    let v = BodyType::from_config_str(value);
                    client.p1(1);
                    client.p1(v as u8);
                    server.p1(1);
                    server.p1(v as u8);
                }

                // 2
                _ if key.starts_with("model") => {
                    parse_model_kind(registry, "model", key, value, |v| models.push((v.0, v.1)))
                }

                // 3
                "disable" => parse_bool(value, |v| {
                    if v {
                        client.p1(3);
                        server.p1(3);
                    }
                }),

                // 40-59
                _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                    RecolType::S(idx, v) => recol_s.push((idx, v)),
                    RecolType::D(idx, v) => recol_d.push((idx, v)),
                }),

                #[cfg(since_647)]
                _ if key.starts_with("retex") => {
                    parse_retex(registry, key, value, |value| match value {
                        RecolType::S(index, value) => retex_s.push((index, value)),
                        RecolType::D(index, value) => retex_d.push((index, value)),
                    })
                }

                // 60-69
                _ if key
                    .strip_prefix("head")
                    .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit())) =>
                {
                    parse_model_kind(registry, "head", key, value, |v| {
                        #[cfg(before_647)]
                        head_models.push((v.0, v.1));
                        #[cfg(since_647)]
                        {
                            client.p1((59 + v.0) as u8);
                            client.p2(v.1);
                            server.p1((59 + v.0) as u8);
                            server.p2(v.1);
                        }
                    })
                }

                // not found
                _ => panic!("Unrecognized idk config key: {key}"),
            }
        }

        #[cfg(before_647)]
        {
            // handle 40-49
            if !recol_s.is_empty() {
                for (idx, v) in recol_s {
                    client.p1((39 + idx) as u8);
                    client.p2(rgb15_to_hsl16(v));
                    server.p1((39 + idx) as u8);
                    server.p2(rgb15_to_hsl16(v));
                }
            }

            // handle 50-59
            if !recol_d.is_empty() {
                for (idx, v) in recol_d {
                    client.p1((49 + idx) as u8);
                    client.p2(rgb15_to_hsl16(v));
                    server.p1((49 + idx) as u8);
                    server.p2(rgb15_to_hsl16(v));
                }
            }

            // handle 60-69
            if !head_models.is_empty() {
                for (idx, v) in head_models {
                    client.p1((59 + idx) as u8);
                    client.p2(v);
                    server.p1((59 + idx) as u8);
                    server.p2(v);
                }
            }

            // handle 2
            if !models.is_empty() {
                client.p1(2);
                client.p1(models.len() as u8);
                server.p1(2);
                server.p1(models.len() as u8);
                for (_, v) in models {
                    client.p2(v);
                    server.p2(v);
                }
            }
        }
        #[cfg(since_647)]
        for code in order {
            match code {
                2 => {
                    models.sort_by_key(|(index, _)| *index);
                    for stream in [&mut server, &mut client] {
                        stream.p1(2);
                        stream.p1(u8::try_from(models.len()).expect("Too many models"));
                        for &(_, model) in &models {
                            stream.p2(model);
                        }
                    }
                }
                40 => {
                    recol_s.sort_by_key(|(index, _)| *index);
                    recol_d.sort_by_key(|(index, _)| *index);
                    pack_pairs(&mut server, 40, &recol_s, &recol_d);
                    pack_pairs(&mut client, 40, &recol_s, &recol_d);
                }
                41 => {
                    retex_s.sort_by_key(|(index, _)| *index);
                    retex_d.sort_by_key(|(index, _)| *index);
                    pack_pairs(&mut server, 41, &retex_s, &retex_d);
                    pack_pairs(&mut client, 41, &retex_s, &retex_d);
                }
                _ => unreachable!(),
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
