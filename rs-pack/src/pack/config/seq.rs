use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
#[cfg(since_647)]
use crate::pack::util::config_key_index;
use crate::pack::util::{parse_bool, parse_number, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_seqs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.seq;

    let configs = parse_config_sections_cached(file_cache, "seq", constants)?;

    #[cfg(since_647)]
    let client_configs = file_cache.client_configs("seq", constants)?;
    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some((debugname, _props)) = config_entry("seq", pack, &configs, id) else {
            server.skip_entry();
            client.skip_entry();
            continue;
        };

        #[cfg(before_647)]
        let props = _props;
        let mut frames: Vec<(usize, u16, u16, u16)> = Vec::new();
        let mut iframes: Vec<(usize, u16, u16)> = Vec::new();
        #[cfg(before_647)]
        let mut delays: Vec<(usize, u16)> = Vec::new();

        #[cfg(since_647)]
        let mut synths = Vec::new();
        #[cfg(since_647)]
        let props = client_configs
            .get(debugname)
            .map(Vec::as_slice)
            .unwrap_or_default();

        for (key, value) in props {
            #[cfg(since_647)]
            {
                if config_key_index(key, "frame").is_none() {
                    flush_frames(&mut client, &mut frames);
                }
                if config_key_index(key, "iframe").is_none() {
                    flush_iframes(&mut client, &mut iframes);
                }
                if config_key_index(key, "synth").is_none() {
                    flush_synths(&mut client, &mut synths);
                }
            }
            match key.as_str() {
                // 1
                _ if key.starts_with("frame") => {
                    let Some(frame) = key.strip_prefix("frame") else {
                        panic!("Unknown frame index: {key}");
                    };
                    #[cfg(before_647)]
                    parse_number(frame, |v| {
                        parse_ref(&registry.anim, "anim", value, |x| frames.push((v, 0, x, 0)));
                    });
                    #[cfg(since_647)]
                    {
                        let parts: Vec<u16> = value
                            .split(':')
                            .map(|part| part.parse().expect("Invalid sequence frame"))
                            .collect();
                        assert_eq!(parts.len(), 3, "Expected delay:frame:group");
                        frames.push((frame.parse().unwrap(), parts[0], parts[1], parts[2]));
                    }
                }

                _ if key.starts_with("iframe") => {
                    let Some(iframe) = key.strip_prefix("iframe") else {
                        panic!("Unknown iframe index: {key}");
                    };
                    #[cfg(before_647)]
                    parse_number(iframe, |v| {
                        parse_ref(&registry.anim, "anim", value, |x| iframes.push((v, x, 0)));
                    });
                    #[cfg(since_647)]
                    {
                        let (frame, group) = value.split_once(':').expect("Expected frame:group");
                        iframes.push((
                            iframe.parse().unwrap(),
                            frame.parse().unwrap(),
                            group.parse().unwrap(),
                        ));
                    }
                }

                #[cfg(before_647)]
                _ if key.starts_with("delay") => {
                    let Some(delay) = key.strip_prefix("delay") else {
                        panic!("Unknown delay index: {key}");
                    };
                    parse_number(delay, |v| {
                        parse_number(value, |x| delays.push((v, x)));
                    });
                }

                // 2
                "loops" | "loopoffset" => parse_number(value, |v| {
                    client.p1(2);
                    client.p2(v);
                }),

                // 3
                #[cfg(before_647)]
                "walkmerge" => {
                    let labels: Vec<u8> = value
                        .split(',')
                        .map(|x| x.strip_prefix("label_").unwrap())
                        .map(|x| x.parse::<u8>().unwrap())
                        .collect();
                    client.p1(3);
                    client.p1(labels.len() as u8);

                    for label in labels {
                        client.p1(label);
                    }
                }

                #[cfg(since_647)]
                "sound_mask" => {
                    let vals: Vec<u8> = value
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    client.p1(3);
                    client.p1(vals.len() as u8);
                    for v in &vals {
                        client.p1(*v);
                    }
                }

                // 4
                #[cfg(before_289)]
                "stretches" => parse_bool(value, |v| {
                    if v {
                        client.p1(4);
                    }
                }),

                #[cfg(since_289)]
                "reachforward" => parse_bool(value, |v| {
                    if v {
                        client.p1(4);
                    }
                }),

                // 5
                "priority" => parse_number(value, |v| {
                    client.p1(5);
                    client.p1(v);
                }),

                // 6
                "replaceheldleft" => {
                    client.p1(6);

                    if value == "hide" {
                        client.p2(0);
                    } else {
                        parse_ref(&registry.obj, "obj", value, |v| {
                            client.p2(v + 512);
                        });
                    }
                }

                #[cfg(since_647)]
                "op6" => {
                    let v: u16 = value.parse().expect("Invalid sequence value");
                    client.p1(6);
                    client.p2(v);
                }

                // 7
                "replaceheldright" => {
                    client.p1(7);

                    if value == "hide" {
                        client.p2(0);
                    } else {
                        parse_ref(&registry.obj, "obj", value, |v| {
                            client.p2(v + 512);
                        });
                    }
                }

                #[cfg(since_647)]
                "op7" => {
                    let v: u16 = value.parse().expect("Invalid sequence value");
                    client.p1(7);
                    client.p2(v);
                }

                // 8
                "maxloops" | "loopcount" => parse_number(value, |v| {
                    client.p1(8);
                    client.p1(v);
                }),

                // 9
                #[cfg(since_244)]
                "preanim_move" | "interrupt" => {
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
                }

                #[cfg(since_647)]
                "walkmerge" => parse_number(value, |v| {
                    client.p1(10);
                    client.p1(v);
                }),

                // 11
                #[cfg(all(since_244, before_289))]
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
                }

                #[cfg(since_289)]
                "duplicatebehaviour" | "looptype" => {
                    let v: u8 = match value.as_str() {
                        "reset" => 1,
                        "reset_loop" => 2,
                        other => other
                            .parse()
                            .unwrap_or_else(|_| panic!("Invalid duplicatebehaviour: {other}")),
                    };
                    client.p1(11);
                    client.p1(v);
                }

                #[cfg(since_647)]
                _ if config_key_index(key, "synth").is_some() => {
                    synths.push((key[5..].parse().unwrap(), value.as_str()))
                }

                #[cfg(since_647)]
                "op14_abool9" => {
                    client.p1(14);
                }

                #[cfg(since_647)]
                "tween" => {
                    client.p1(15);
                }

                #[cfg(since_647)]
                "op16_abool8" => {
                    client.p1(16);
                }

                #[cfg(since_647)]
                "op18_abool10" => {
                    client.p1(18);
                }

                #[cfg(since_647)]
                "op19" => {
                    let parts: Vec<&str> = value.split(':').collect();
                    if parts.len() == 2 {
                        let idx: u8 = parts[0].parse().expect("Invalid sequence value");
                        let val: u8 = parts[1].parse().expect("Invalid sequence value");
                        client.p1(19);
                        client.p1(idx);
                        client.p1(val);
                    }
                }

                #[cfg(since_647)]
                "op20" => {
                    let parts: Vec<&str> = value.split(':').collect();
                    if parts.len() == 3 {
                        let idx: u8 = parts[0].parse().expect("Invalid sequence value");
                        let a: u16 = parts[1].parse().expect("Invalid sequence value");
                        let b: u16 = parts[2].parse().expect("Invalid sequence value");
                        client.p1(20);
                        client.p1(idx);
                        client.p2(a);
                        client.p2(b);
                    }
                }

                // not found
                _ => panic!("Unrecognized seq config key: {key}"),
            }
        }

        #[cfg(before_647)]
        {
            // handle 1
            if !frames.is_empty() {
                client.p1(1);
                client.p1(frames.len() as u8);

                for (index, _, frame, _) in frames {
                    client.p2(frame);

                    if let Some((_, iframe, _)) = iframes.iter().find(|x| x.0 == index) {
                        client.p2(*iframe);
                    } else {
                        client.p2(0xFFFF); // -1
                    }

                    if let Some((_, delay)) = delays.iter().find(|x| x.0 == index) {
                        client.p2(*delay);
                    } else {
                        client.p2(0);
                    }
                }
            }
        }
        #[cfg(since_647)]
        {
            flush_frames(&mut client, &mut frames);
            flush_iframes(&mut client, &mut iframes);
            flush_synths(&mut client, &mut synths);
        }

        server.dat.extend_from_slice(client.entry());

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

#[cfg(since_647)]
fn flush_frames(out: &mut PackedData, buf: &mut Vec<(usize, u16, u16, u16)>) {
    if buf.is_empty() {
        return;
    }
    buf.sort_by_key(|(idx, _, _, _)| *idx);
    out.p1(1);
    out.p2(u16::try_from(buf.len()).expect("Too many sequence entries"));
    for &(_, dur, _, _) in buf.iter() {
        out.p2(dur);
    }
    for &(_, _, lo, _) in buf.iter() {
        out.p2(lo);
    }
    for &(_, _, _, hi) in buf.iter() {
        out.p2(hi);
    }
    buf.clear();
}

#[cfg(since_647)]
fn flush_iframes(out: &mut PackedData, buf: &mut Vec<(usize, u16, u16)>) {
    if buf.is_empty() {
        return;
    }
    buf.sort_by_key(|(idx, _, _)| *idx);
    out.p1(12);
    out.p1(u8::try_from(buf.len()).expect("Too many interleaved frames"));
    for &(_, lo, _) in buf.iter() {
        out.p2(lo);
    }
    for &(_, _, hi) in buf.iter() {
        out.p2(hi);
    }
    buf.clear();
}

#[cfg(since_647)]
fn flush_synths(out: &mut PackedData, buf: &mut Vec<(usize, &str)>) {
    if buf.is_empty() {
        return;
    }
    buf.sort_by_key(|(idx, _)| *idx);
    out.p1(13);
    out.p2(u16::try_from(buf.len()).expect("Too many sequence entries"));
    for (_, value) in buf.iter() {
        if *value == "null" {
            out.p1(0);
        } else {
            let parts: Vec<&str> = value.split(':').collect();
            let head: u32 = parts[0].parse().expect("Invalid sequence value");
            let tail_count = parts.len() - 1;
            out.p1(u8::try_from(1 + tail_count).expect("Too many sequence sounds"));
            out.p3(head as i32);
            for &t in &parts[1..] {
                let v: u16 = t.parse().expect("Invalid sequence value");
                out.p2(v);
            }
        }
    }
    buf.clear();
}
