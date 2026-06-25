use crate::config_crc;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_anim, parse_bool, parse_number, parse_obj};
use anyhow::Result;
use rs_io::crc;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_seqs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.seq;

    let files = file_cache.collect("seq");
    debug!("  Found {} .seq files", files.len());

    let configs = parse_config_sections_cached(file_cache, "seq", constants);
    debug!("  Parsed {} seq configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown seq id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown seq config: {debugname}");
        };

        let mut frames: Vec<(usize, u16)> = Vec::new();
        let mut iframes: Vec<(usize, u16)> = Vec::new();
        let mut delays: Vec<(usize, u16)> = Vec::new();

        for (key, value) in props {
            match key.as_str() {
                // 1
                _ if key.starts_with("frame") => {
                    let Some(frame) = key.strip_prefix("frame") else {
                        panic!("Unknown frame index: {key}");
                    };
                    parse_number(frame, |v| {
                        parse_anim(registry, value, |x| frames.push((v, x)));
                    });
                }
                _ if key.starts_with("iframe") => {
                    let Some(iframe) = key.strip_prefix("iframe") else {
                        panic!("Unknown iframe index: {key}");
                    };
                    parse_number(iframe, |v| {
                        parse_anim(registry, value, |x| iframes.push((v, x)));
                    });
                }
                _ if key.starts_with("delay") => {
                    let Some(delay) = key.strip_prefix("delay") else {
                        panic!("Unknown delay index: {key}");
                    };
                    parse_number(delay, |v| {
                        parse_number(value, |x| delays.push((v, x)));
                    });
                }

                // 2
                "loops" => parse_number(value, |v| {
                    client.p1(2);
                    client.p2(v);
                    server.p1(2);
                    server.p2(v);
                }),

                // 3
                "walkmerge" => {
                    let labels: Vec<u8> = value
                        .split(',')
                        .map(|x| x.strip_prefix("label_").unwrap())
                        .map(|x| x.parse::<u8>().unwrap())
                        .collect();
                    client.p1(3);
                    client.p1(labels.len() as u8);
                    server.p1(3);
                    server.p1(labels.len() as u8);
                    for label in labels {
                        client.p1(label);
                        server.p1(label);
                    }
                }

                // 4
                "stretches" => parse_bool(value, |v| {
                    if v {
                        client.p1(4);
                        server.p1(4);
                    }
                }),

                // 5
                "priority" => parse_number(value, |v| {
                    client.p1(5);
                    client.p1(v);
                    server.p1(5);
                    server.p1(v);
                }),

                // 6
                "replaceheldleft" => {
                    client.p1(6);
                    server.p1(6);
                    if value == "hide" {
                        client.p2(0);
                        server.p2(0);
                    } else {
                        parse_obj(registry, value, |v| {
                            client.p2(v + 512);
                            server.p2(v + 512);
                        });
                    }
                }

                // 7
                "replaceheldright" => {
                    client.p1(7);
                    server.p1(7);
                    if value == "hide" {
                        client.p2(0);
                        server.p2(0);
                    } else {
                        parse_obj(registry, value, |v| {
                            client.p2(v + 512);
                            server.p2(v + 512);
                        });
                    }
                }

                // 8
                "maxloops" => parse_number(value, |v| {
                    client.p1(8);
                    client.p1(v);
                    server.p1(8);
                    server.p1(v);
                }),

                // 9
                #[cfg(since_244)]
                "preanim_move" => {
                    let v: u8 = match value.as_str() {
                        "delaymove" => 0,
                        "delayanim" => 1,
                        "merge" => 2,
                        other => other
                            .parse()
                            .unwrap_or_else(|_| panic!("Invalid preanim_move: {other}")),
                    };
                    client.p1(9);
                    client.p1(v);
                    server.p1(9);
                    server.p1(v);
                }

                // 10
                #[cfg(since_244)]
                "postanim_move" => {
                    let v: u8 = match value.as_str() {
                        "delaymove" => 0,
                        "abortanim" => 1,
                        "merge" => 2,
                        other => other
                            .parse()
                            .unwrap_or_else(|_| panic!("Invalid postanim_move: {other}")),
                    };
                    client.p1(10);
                    client.p1(v);
                    server.p1(10);
                    server.p1(v);
                }

                // 11
                #[cfg(since_244)]
                "duplicatebehavior" => {
                    // TODO: this has to be in british spelling
                    let v: u8 = match value.as_str() {
                        "reset" => 1,
                        "reset_loop" => 2,
                        other => other
                            .parse()
                            .unwrap_or_else(|_| panic!("Invalid duplicatebehavior: {other}")),
                    };
                    client.p1(11);
                    client.p1(v);
                    server.p1(11);
                    server.p1(v);
                }

                // not found
                _ => panic!("Unrecognized seq config key: {key}"),
            }
        }

        // handle 1
        if !frames.is_empty() {
            client.p1(1);
            client.p1(frames.len() as u8);
            server.p1(1);
            server.p1(frames.len() as u8);

            for (index, frame) in frames {
                client.p2(frame);
                server.p2(frame);

                if let Some((_, iframe)) = iframes.iter().find(|x| x.0 == index) {
                    client.p2(*iframe);
                    server.p2(*iframe);
                } else {
                    client.p2(0xFFFF); // -1
                    server.p2(0xFFFF); // -1
                }

                if let Some((_, delay)) = delays.iter().find(|x| x.0 == index) {
                    client.p2(*delay);
                    server.p2(*delay);
                } else {
                    client.p2(0);
                    server.p2(0);
                }
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        client.finish_entry();
    }

    if verify {
        let crc = crc::getcrc(&client.dat, 0, client.dat.len());
        let expected = config_crc::SEQ;
        if crc != expected {
            panic!("CRC mismatch ['seq']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
