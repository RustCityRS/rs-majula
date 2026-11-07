use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
#[cfg(since_647)]
use crate::pack::util::{config_key_index, parse_numbers, parse_sound};
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
        let mut sounds = Vec::new();
        #[cfg(since_647)]
        let props = client_configs
            .get(debugname)
            .map(Vec::as_slice)
            .unwrap_or_default();

        for (key, value) in props {
            #[cfg(since_647)]
            {
                if config_key_index(key, "frame").is_none()
                    && config_key_index(key, "delay").is_none()
                {
                    flush_frames(&mut client, &mut frames);
                }
                if config_key_index(key, "iframe").is_none() {
                    flush_iframes(&mut client, &mut iframes);
                }
                if config_key_index(key, "sound").is_none() {
                    flush_sounds(&mut client, &mut sounds, registry);
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
                        let (file, group) = parse_frame(value);
                        frames.push((frame.parse().unwrap(), 0, file, group));
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
                        let (file, group) = parse_frame(value);
                        iframes.push((iframe.parse().unwrap(), file, group));
                    }
                }

                _ if key.starts_with("delay") => {
                    let Some(delay) = key.strip_prefix("delay") else {
                        panic!("Unknown delay index: {key}");
                    };
                    #[cfg(before_647)]
                    parse_number(delay, |v| {
                        parse_number(value, |x| delays.push((v, x)));
                    });
                    #[cfg(since_647)]
                    {
                        let index: usize = delay.parse().unwrap();
                        let frame = frames
                            .iter_mut()
                            .rev()
                            .find(|frame| frame.0 == index)
                            .unwrap_or_else(|| panic!("{debugname}: {key} has no frame"));
                        parse_number(value, |v| frame.1 = v);
                    }
                }

                // 2
                #[cfg(before_647)]
                "loops" => parse_number(value, |v| {
                    client.p1(2);
                    client.p2(v);
                }),

                #[cfg(since_647)]
                "loops" => parse_number(value, |v: i16| {
                    client.p1(2);
                    client.p2(v as u16);
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

                    for label in labels {
                        client.p1(label);
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
                #[cfg(before_647)]
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
                "replaceheldleft" => {
                    client.p1(6);
                    client.p2(parse_replace_held(value, registry));
                }

                // 7
                #[cfg(before_647)]
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
                "replaceheldright" => {
                    client.p1(7);
                    client.p2(parse_replace_held(value, registry));
                }

                // 8
                "maxloops" => parse_number(value, |v| {
                    client.p1(8);
                    client.p1(v);
                }),

                // 9
                #[cfg(since_244)]
                "preanim_move" => {
                    let v: u8 = match value.as_str() {
                        "delaymove" => 0,
                        "delayanim" => 1,
                        "merge" => 2,
                        #[cfg(since_647)]
                        "abortanim" => 3,
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
                "duplicatebehaviour" => {
                    let v: u8 = match value.as_str() {
                        #[cfg(since_647)]
                        "ignore" => 0,
                        "reset" => 1,
                        "reset_loop" => 2,
                        other => other
                            .parse()
                            .unwrap_or_else(|_| panic!("Invalid duplicatebehaviour: {other}")),
                    };
                    client.p1(11);
                    client.p1(v);
                }

                // 13, and 18 when the sounds are vorbis files
                #[cfg(since_647)]
                _ if config_key_index(key, "sound").is_some() => {
                    sounds.push((config_key_index(key, "sound").unwrap(), value.as_str()))
                }

                // 14
                #[cfg(since_647)]
                "unknown14" => client.p1(14),

                // 15
                #[cfg(since_647)]
                "unknown15" => client.p1(15),

                // 16
                #[cfg(since_647)]
                "unknown16" => client.p1(16),

                // 19
                #[cfg(since_647)]
                "soundvolume" => {
                    let [index, volume] = parse_numbers::<u8, 2>(value);
                    client.p1(19);
                    client.p1(index);
                    client.p1(volume);
                }

                // 20
                #[cfg(since_647)]
                "soundrandomrate" => {
                    let [index, min, max] = parse_numbers::<u16, 3>(value);
                    client.p1(20);
                    client.p1(u8::try_from(index).expect("Invalid sound index"));
                    client.p2(min);
                    client.p2(max);
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
            flush_sounds(&mut client, &mut sounds, registry);
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

/// `animframe_{group}_{file}`, as (file, group).
#[cfg(since_647)]
fn parse_frame(value: &str) -> (u16, u16) {
    let (group, file) = value
        .strip_prefix("animframe_")
        .and_then(|value| value.split_once('_'))
        .unwrap_or_else(|| panic!("Invalid sequence frame: {value}"));
    (
        file.parse().expect("Invalid sequence frame"),
        group.parse().expect("Invalid sequence frame"),
    )
}

/// The inverse of how a dump prints these: below the objs' range the number
/// is the raw value less 512, wrapped.
#[cfg(since_647)]
fn parse_replace_held(value: &str, registry: &PackRegistry) -> u16 {
    if value == "hide" {
        return u16::MAX;
    }
    if let Some(wrapped) = value
        .strip_prefix("obj_")
        .and_then(|id| id.parse::<u32>().ok())
        .filter(|&id| id > u16::MAX as u32)
    {
        return wrapped.wrapping_add(512) as u16;
    }
    let mut id = 0;
    parse_ref(&registry.obj, "obj", value, |v| id = v + 512);
    id
}

#[cfg(since_647)]
fn flush_sounds(out: &mut PackedData, buf: &mut Vec<(usize, &str)>, registry: &PackRegistry) {
    let Some(last) = buf.iter().map(|(index, _)| *index).max() else {
        return;
    };
    let mut vorbis = false;
    out.p1(13);
    out.p2(u16::try_from(last + 1).expect("Too many sequence entries"));
    for index in 0..=last {
        let sound = buf
            .iter()
            .find(|(i, value)| *i == index && *value != "null")
            .map(|(_, value)| *value);
        let Some(sound) = sound else {
            out.p1(0);
            continue;
        };
        let parts: Vec<&str> = sound.split(',').collect();
        let [id, a, b, tail @ ..] = &parts[..] else {
            panic!("Invalid sequence sound: {sound}");
        };
        let (id, is_vorbis) = parse_sound(registry, id.trim());
        vorbis |= is_vorbis;
        let a: i32 = a.trim().parse().expect("Invalid sequence sound");
        let b: i32 = b.trim().parse().expect("Invalid sequence sound");
        out.p1(u8::try_from(1 + tail.len()).expect("Too many sequence sounds"));
        out.p3(((id as i32) << 8) | (a << 4) | b);
        for value in tail {
            out.p2(value.trim().parse().expect("Invalid sequence sound"));
        }
    }
    if vorbis {
        out.p1(18);
    }
    buf.clear();
}
