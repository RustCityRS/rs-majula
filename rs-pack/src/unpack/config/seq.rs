use super::UnpackedPacks;
use crate::cache::seq::SeqTypeRaw;
use std::collections::HashMap;

pub fn emit_seq_props(
    def: &SeqTypeRaw,
    _registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    if def.raw_opcodes.len() > 1
        && (cfg!(since_647) || super::has_repeated_opcodes(&def.opcode_order))
    {
        return super::emit_repeated::<SeqTypeRaw>(
            def.id,
            &def.raw_opcodes,
            _registry,
            _reverse_hsl,
            emit_seq_props,
            false,
            |value| {
                value.opcode_order.clear();
                value.raw_opcodes.clear();
                #[cfg(since_647)]
                {
                    value.op19.clear();
                    value.op20.clear();
                }
            },
        );
    }
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
            p.push((
                1,
                format!("frame{}", i + 1),
                format!("{}:{}:{}", delays[i], frames[i] as u16, frames[i] >> 16),
            ));
        }
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&2) || def.loops.is_some() {
        p.push((
            2,
            "loopoffset".into(),
            (def.loops.unwrap_or(u16::MAX) as i16).to_string(),
        ));
    }
    #[cfg(since_647)]
    if let Some(mask) = &def.walkmerge {
        let csv = mask
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",");
        p.push((3, "sound_mask".into(), csv));
    }
    if def.opcode_order.contains(&5) || def.priority != 5 {
        p.push((5, "priority".into(), def.priority.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&6) || def.replaceheldleft.is_some() {
        p.push((
            6,
            "op6".into(),
            def.replaceheldleft.unwrap_or(u16::MAX).to_string(),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&7) || def.replaceheldright.is_some() {
        p.push((
            7,
            "op7".into(),
            def.replaceheldright.unwrap_or(u16::MAX).to_string(),
        ));
    }
    if def.opcode_order.contains(&8) || def.maxloops != 99 {
        #[cfg(before_647)]
        let key = "maxloops";
        #[cfg(since_647)]
        let key = "loopcount";
        p.push((8, key.into(), def.maxloops.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&9) {
        p.push((
            9,
            "interrupt".into(),
            (def.preanim_move.unwrap_or(u8::MAX) as i8).to_string(),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&10) {
        p.push((
            10,
            "walkmerge".into(),
            (def.postanim_move.unwrap_or(u8::MAX) as i8).to_string(),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&11) || def.duplicatebehaviour.unwrap_or(2) != 2 {
        p.push((
            11,
            "looptype".into(),
            def.duplicatebehaviour.unwrap_or(2).to_string(),
        ));
    }
    #[cfg(since_647)]
    if let Some(iframes) = &def.iframes {
        for i in 0..iframes.len() {
            p.push((
                12,
                format!("iframe{}", i + 1),
                format!(
                    "{}:{}",
                    iframes[i].unwrap_or(u32::MAX) as u16,
                    iframes[i].unwrap_or(u32::MAX) >> 16
                ),
            ));
        }
    }
    #[cfg(since_647)]
    if let Some(synths) = &def.synths {
        for (i, slot) in synths.iter().enumerate() {
            let value = match slot {
                None => "null".to_string(),
                Some(s) => {
                    if s.tail.is_empty() {
                        format!("{}", s.head)
                    } else {
                        let tail = s
                            .tail
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(":");
                        format!("{}:{}", s.head, tail)
                    }
                }
            };
            p.push((13, format!("synth{}", i + 1), value));
        }
    }
    #[cfg(since_647)]
    if def.a_bool9 {
        p.push((14, "op14_abool9".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if def.tween {
        p.push((15, "tween".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if def.a_bool8 {
        p.push((16, "op16_abool8".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if def.a_bool10 {
        p.push((18, "op18_abool10".into(), "yes".into()));
    }
    #[cfg(since_647)]
    for (idx, val) in &def.op19 {
        p.push((19, "op19".into(), format!("{idx}:{val}")));
    }
    #[cfg(since_647)]
    for (idx, a, b) in &def.op20 {
        p.push((20, "op20".into(), format!("{idx}:{a}:{b}")));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
