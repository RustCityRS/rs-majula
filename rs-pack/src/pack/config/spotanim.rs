use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::RecolType;
#[cfg(before_647)]
use crate::pack::util::colour::rgb15_to_hsl16;
#[cfg(before_647)]
use crate::pack::util::parse_bool;
#[cfg(since_647)]
use crate::pack::util::{pack_pairs, parse_retex};
use crate::pack::util::{parse_number, parse_recol, parse_ref};
use anyhow::Result;
#[cfg(since_647)]
use anyhow::bail;
use std::collections::HashMap;

pub fn pack_spotanims(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.spotanim;

    let configs = parse_config_sections_cached(file_cache, "spotanim", constants)?;

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some((debugname, props)) = config_entry("spotanim", pack, &configs, id) else {
            server.skip_entry();
            client.skip_entry();
            continue;
        };

        #[cfg(since_647)]
        let (mut recol_s, mut recol_d, mut retex_s, mut retex_d) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());

        for (key, value) in props {
            match key.as_str() {
                // 1
                "model" => parse_ref(&registry.model, "model", value, |v| {
                    client.p1(1);
                    client.p2(v);
                    server.p1(1);
                    server.p2(v);
                }),

                // 2
                "anim" => parse_ref(&registry.seq, "seq", value, |v| {
                    client.p1(2);
                    client.p2(v);
                    server.p1(2);
                    server.p2(v);
                }),

                // 3
                #[cfg(before_647)]
                "hasalpha" => parse_bool(value, |v| {
                    if v {
                        client.p1(3);
                        server.p1(3);
                    }
                }),

                // 4
                "resizeh" => parse_number(value, |v| {
                    client.p1(4);
                    client.p2(v);
                    server.p1(4);
                    server.p2(v);
                }),

                // 5
                "resizev" => parse_number(value, |v| {
                    client.p1(5);
                    client.p2(v);
                    server.p1(5);
                    server.p2(v);
                }),

                // 6
                #[cfg(before_647)]
                "angle" => parse_number(value, |v| {
                    client.p1(6);
                    client.p2(v);
                    server.p1(6);
                    server.p2(v);
                }),

                #[cfg(since_647)]
                "rotation" => parse_number(value, |v| {
                    client.p1(6);
                    client.p2(v);
                    server.p1(6);
                    server.p2(v);
                }),

                // 7
                "ambient" => parse_number(value, |v| {
                    client.p1(7);
                    client.p1(v);
                    server.p1(7);
                    server.p1(v);
                }),

                // 8
                "contrast" => parse_number(value, |v| {
                    client.p1(8);
                    client.p1(v);
                    server.p1(8);
                    server.p1(v);
                }),

                #[cfg(since_647)]
                "hillchange" => {
                    let parts: Vec<_> = value.split(',').map(str::trim).collect();
                    let (code, param) = match parts.as_slice() {
                        ["rotate"] => (9, 0),
                        ["skew"] => (11, 0),
                        ["ceiling_skew"] => (12, 0),
                        ["skew_to_fit"] => (13, 0),
                        ["tree_skew", param] => (14, param.parse::<u8>()? as i32),
                        ["rotate", param] => {
                            let param = param.parse::<i32>()?;
                            (if u16::try_from(param).is_ok() { 15 } else { 16 }, param)
                        }
                        ["rotate", param, "32"] => (16, param.parse::<i32>()?),
                        _ => bail!("{debugname}: invalid hillchange '{value}'"),
                    };
                    for stream in [&mut server, &mut client] {
                        stream.p1(code);
                        match code {
                            14 => stream.p1(param as u8),
                            15 => stream.p2(param as u16),
                            16 => stream.p4(param),
                            _ => {}
                        }
                    }
                }

                #[cfg(since_647)]
                "allowloop" => {
                    server.p1(10);
                    client.p1(10);
                }

                // 40-60
                #[cfg(before_647)]
                _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                    RecolType::S(idx, v) => {
                        client.p1((39 + idx) as u8);
                        client.p2(rgb15_to_hsl16(v));
                        server.p1((39 + idx) as u8);
                        server.p2(rgb15_to_hsl16(v));
                    }
                    RecolType::D(idx, v) => {
                        client.p1((49 + idx) as u8);
                        client.p2(rgb15_to_hsl16(v));
                        server.p1((49 + idx) as u8);
                        server.p2(rgb15_to_hsl16(v));
                    }
                }),

                #[cfg(since_647)]
                _ if key.starts_with("recol") => parse_recol(key, value, |value| match value {
                    RecolType::S(index, value) => recol_s.push((index, value)),
                    RecolType::D(index, value) => recol_d.push((index, value)),
                }),

                #[cfg(since_647)]
                _ if key.starts_with("retex") => {
                    parse_retex(registry, key, value, |value| match value {
                        RecolType::S(index, value) => retex_s.push((index, value)),
                        RecolType::D(index, value) => retex_d.push((index, value)),
                    })
                }

                // not found
                _ => panic!("Unrecognized spotanim config key: {key}"),
            }
        }

        #[cfg(since_647)]
        {
            recol_s.sort_by_key(|(index, _)| *index);
            recol_d.sort_by_key(|(index, _)| *index);
            retex_s.sort_by_key(|(index, _)| *index);
            retex_d.sort_by_key(|(index, _)| *index);
            for stream in [&mut server, &mut client] {
                pack_pairs(stream, 40, &recol_s, &recol_d);
                pack_pairs(stream, 41, &retex_s, &retex_d);
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
