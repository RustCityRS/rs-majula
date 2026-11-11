use super::UnpackedPacks;
use crate::cache::seq::SeqTypeRaw;
use std::collections::HashMap;

#[cfg(since_647)]
fn anim_frame(frame: u32) -> String {
    format!("animframe_{}_{}", frame >> 16, frame as u16)
}

/// `hide`, or an obj. Below the objs' range a dump still takes 512 off.
#[cfg(since_647)]
fn replace_held(value: u16, packs: &UnpackedPacks) -> String {
    match value {
        u16::MAX => "hide".to_owned(),
        512.. => packs.reference("obj", value - 512),
        below => format!("obj_{}", (below as u32).wrapping_sub(512)),
    }
}

pub fn emit_seq_props(
    def: &SeqTypeRaw,
    opcode: u8,
    _data: &[u8],
    _registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(before_647)]
    {
        if let (Some(frames), Some(delays), Some(iframes)) =
            (&def.frames, &def.delays, &def.iframes)
        {
            for (i, (&frame, &delay)) in frames.iter().zip(delays.iter()).enumerate() {
                p.push((
                    1,
                    format!("frame{i}"),
                    super::anim_ref(frame as u16, _registry),
                ));
                if let Some(iframe) = iframes[i] {
                    p.push((
                        1,
                        format!("iframe{i}"),
                        super::anim_ref(iframe as u16, _registry),
                    ));
                }
                if delay != 0 {
                    p.push((1, format!("delay{i}"), delay.to_string()));
                }
            }
        }
        if let Some(loops) = def.loops {
            p.push((2, "loops".into(), loops.to_string()));
        }
        if let Some(mask) = &def.walkmerge {
            let labels = mask
                .iter()
                .take(mask.len() - 1)
                .map(|v| format!("label_{v}"))
                .collect::<Vec<_>>();
            p.push((3, "walkmerge".into(), labels.join(",")));
        }
        if def.reachforward {
            #[cfg(before_289)]
            p.push((4, "stretches".into(), "yes".into()));
            #[cfg(since_289)]
            p.push((4, "reachforward".into(), "yes".into()));
        }
        for (code, name, value) in [
            (6, "replaceheldleft", def.replaceheldleft),
            (7, "replaceheldright", def.replaceheldright),
        ] {
            if let Some(value) = value {
                p.push((
                    code,
                    name.into(),
                    if value == 0 {
                        "hide".into()
                    } else {
                        super::obj_ref(value - 512, _registry)
                    },
                ));
            }
        }
        #[cfg(since_244)]
        {
            if let Some(value) = def.preanim_move {
                let value = match value {
                    0 => "delaymove",
                    1 => "delayanim",
                    2 => "merge",
                    _ => panic!("Unrecognized seq config preanim_move value: {value}"),
                };
                p.push((9, "preanim_move".into(), value.into()));
            }
            if let Some(value) = def.postanim_move {
                let value = match value {
                    0 => "delaymove",
                    1 => "abortanim",
                    2 => "merge",
                    _ => panic!("Unrecognized seq config postanim_move value: {value}"),
                };
                p.push((10, "postanim_move".into(), value.into()));
            }
            if let Some(value) = def.duplicatebehaviour {
                let value = match value {
                    1 => "reset",
                    2 => "reset_loop",
                    _ => panic!("Unrecognized seq config duplicatebehaviour value: {value}"),
                };
                #[cfg(before_289)]
                let key = "duplicatebehavior";
                #[cfg(since_289)]
                let key = "duplicatebehaviour";
                p.push((11, key.into(), value.into()));
            }
        }
    }
    #[cfg(since_647)]
    if let (Some(frames), Some(delays)) = (&def.frames, &def.delays) {
        for i in 0..frames.len() {
            p.push((1, format!("frame{}", i + 1), anim_frame(frames[i])));
            p.push((1, format!("delay{}", i + 1), delays[i].to_string()));
        }
    }
    #[cfg(since_647)]
    if opcode == 2 {
        p.push((
            2,
            "loops".into(),
            (def.loops.unwrap_or(u16::MAX) as i16).to_string(),
        ));
    }
    #[cfg(since_647)]
    if let Some(mask) = &def.walkmerge {
        let labels = mask
            .iter()
            .map(|v| format!("label_{v}"))
            .collect::<Vec<_>>()
            .join(",");
        p.push((3, "walkmerge".into(), labels));
    }
    if opcode == 5 {
        p.push((5, "priority".into(), def.priority.to_string()));
    }
    #[cfg(since_647)]
    if opcode == 6 {
        p.push((
            6,
            "replaceheldleft".into(),
            replace_held(def.replaceheldleft.unwrap_or(u16::MAX), _registry),
        ));
    }
    #[cfg(since_647)]
    if opcode == 7 {
        p.push((
            7,
            "replaceheldright".into(),
            replace_held(def.replaceheldright.unwrap_or(u16::MAX), _registry),
        ));
    }
    if opcode == 8 {
        p.push((8, "maxloops".into(), def.maxloops.to_string()));
    }
    #[cfg(since_647)]
    if opcode == 9 {
        let value = match def.preanim_move.unwrap_or(u8::MAX) {
            0 => "delaymove".to_owned(),
            1 => "delayanim".to_owned(),
            2 => "merge".to_owned(),
            3 => "abortanim".to_owned(),
            other => other.to_string(),
        };
        p.push((9, "preanim_move".into(), value));
    }
    #[cfg(since_647)]
    if opcode == 10 {
        let value = match def.postanim_move.unwrap_or(u8::MAX) {
            0 => "delaymove".to_owned(),
            1 => "abortanim".to_owned(),
            2 => "merge".to_owned(),
            other => other.to_string(),
        };
        p.push((10, "postanim_move".into(), value));
    }
    #[cfg(since_647)]
    if opcode == 11 {
        let value = match def.duplicatebehaviour.unwrap_or(2) {
            0 => "ignore".to_owned(),
            1 => "reset".to_owned(),
            2 => "reset_loop".to_owned(),
            other => other.to_string(),
        };
        p.push((11, "duplicatebehaviour".into(), value));
    }
    #[cfg(since_647)]
    if let Some(iframes) = &def.iframes {
        for (i, frame) in iframes.iter().enumerate() {
            p.push((
                12,
                format!("iframe{}", i + 1),
                anim_frame(frame.unwrap_or(u32::MAX)),
            ));
        }
    }
    #[cfg(since_647)]
    if let Some(synths) = &def.synths {
        let kind = if def.sound_vorbis { "vorbis" } else { "synth" };
        for (i, slot) in synths.iter().enumerate() {
            match slot {
                Some(sound) => {
                    let mut values = vec![
                        _registry.reference(kind, (sound.head >> 8) as u16),
                        ((sound.head >> 4) & 0xf).to_string(),
                        (sound.head & 0xf).to_string(),
                    ];
                    values.extend(sound.tail.iter().map(u16::to_string));
                    p.push((13, format!("sound{i}"), values.join(",")));
                }
                None if i + 1 == synths.len() => {
                    p.push((13, format!("sound{i}"), "null".into()));
                }
                None => {}
            }
        }
    }
    #[cfg(since_647)]
    if def.a_bool9 {
        p.push((14, "unknown14".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if def.tween {
        p.push((15, "unknown15".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if def.a_bool8 {
        p.push((16, "unknown16".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if let Some((idx, val)) = def.op19.last() {
        p.push((19, "soundvolume".into(), format!("{idx},{val}")));
    }
    #[cfg(since_647)]
    if let Some((idx, a, b)) = def.op20.last() {
        p.push((20, "soundrandomrate".into(), format!("{idx},{a},{b}")));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
