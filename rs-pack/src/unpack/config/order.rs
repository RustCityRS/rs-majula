#[cfg(since_647)]
use crate::types::ParamValue;
use rs_ver::STRING_TERMINATOR;
use std::collections::{HashMap, VecDeque};
use std::mem::take;

pub fn reorder_by_opcode(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
) -> Vec<(String, String)> {
    reorder_by_opcode_with_raw(opcode_order, props, &[])
}

pub fn reorder_by_opcode_with_raw(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
    raw_opcodes: &[(u8, Vec<u8>)],
) -> Vec<(String, String)> {
    reorder_by_opcode_with_raw_fn(opcode_order, props, raw_opcodes, decode_raw_to_text)
}

pub fn reorder_by_opcode_with_raw_fn(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
    raw_opcodes: &[(u8, Vec<u8>)],
    decoder: impl Fn(u8, &[u8]) -> Option<(String, String)>,
) -> Vec<(String, String)> {
    if opcode_order.is_empty() {
        return props
            .into_iter()
            .map(|(_, key, value)| (key, value))
            .collect();
    }
    let mut by_op: HashMap<u8, VecDeque<(String, String)>> = HashMap::new();
    for (code, key, value) in props {
        by_op.entry(code).or_default().push_back((key, value));
    }
    let mut remaining = [0usize; 256];
    for &code in opcode_order {
        remaining[code as usize] += 1;
    }
    for (code, &count) in remaining
        .iter()
        .enumerate()
        .filter(|(_, count)| **count != 0)
    {
        let code = code as u8;
        let queue = by_op.entry(code).or_default();
        let extra = count.saturating_sub(queue.len());
        if extra > 0 {
            let mut earlier = raw_opcodes
                .iter()
                .filter(|(op, _)| *op == code)
                .take(extra)
                .filter_map(|(_, bytes)| decoder(code, bytes))
                .collect::<VecDeque<_>>();
            earlier.extend(take(queue));
            *queue = earlier;
        }
    }
    let mut result = Vec::new();
    for &code in opcode_order {
        let Some(queue) = by_op.get_mut(&code).filter(|queue| !queue.is_empty()) else {
            continue;
        };
        let count = &mut remaining[code as usize];
        let take = (queue.len() / *count).max(1);
        result.extend(queue.drain(..take));
        *count -= 1;
    }
    result
}

pub fn decode_raw_to_text(opcode: u8, raw: &[u8]) -> Option<(String, String)> {
    // Helper to read big-endian u16
    let r_u16 = |data: &[u8], pos: usize| -> u16 {
        if pos + 1 < data.len() {
            u16::from_be_bytes([data[pos], data[pos + 1]])
        } else {
            0
        }
    };
    let r_i8 =
        |data: &[u8], pos: usize| -> i8 { if pos < data.len() { data[pos] as i8 } else { 0 } };
    let r_u8 = |data: &[u8], pos: usize| -> u8 { if pos < data.len() { data[pos] } else { 0 } };
    let _r_i32 = |data: &[u8], pos: usize| -> i32 {
        if pos + 3 < data.len() {
            i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
        } else {
            0
        }
    };
    let r_i16 = |data: &[u8], pos: usize| -> i16 {
        if pos + 1 < data.len() {
            i16::from_be_bytes([data[pos], data[pos + 1]])
        } else {
            0
        }
    };
    // Read NUL-terminated string
    let r_str = |data: &[u8], pos: usize| -> String {
        let mut end = pos;
        while end < data.len() && data[end] != STRING_TERMINATOR {
            end += 1;
        }
        String::from_utf8_lossy(&data[pos..end]).to_string()
    };

    match opcode {
        // op2: name (string)
        2 => Some(("name".into(), r_str(raw, 0))),
        // op12: size (u8)
        12 => Some(("size".into(), r_u8(raw, 0).to_string())),
        // op30..34: op1..op5 (string) — slot encoded in opcode
        30..=34 => Some((format!("op{}", opcode - 29), r_str(raw, 0))),
        // op93: minimap=no (flag, no payload)
        93 => Some(("minimap".into(), "no".into())),
        // op95: vislevel (u16)
        95 => {
            let v = r_u16(raw, 0);
            if v == 0 {
                Some(("vislevel".into(), "hide".into()))
            } else {
                Some(("vislevel".into(), v.to_string()))
            }
        }
        // op97: resizeh (u16)
        97 => Some(("resizeh".into(), r_u16(raw, 0).to_string())),
        // op98: resizev (u16)
        98 => Some(("resizev".into(), r_u16(raw, 0).to_string())),
        // op99: alwaysontop (flag)
        99 => Some(("alwaysontop".into(), "yes".into())),
        // op100: ambient (i8)
        100 => Some(("ambient".into(), r_i8(raw, 0).to_string())),
        // op101: contrast (i8, raw byte not * 5)
        101 => Some(("contrast".into(), r_i8(raw, 0).to_string())),
        // op102: headicon (u16)
        102 => Some(("headicon".into(), r_u16(raw, 0).to_string())),
        // op103: turnspeed (u16)
        103 => Some(("turnspeed".into(), r_u16(raw, 0).to_string())),

        // op14: width (u8)
        14 => Some(("width".into(), r_u8(raw, 0).to_string())),
        // op15: length (u8)
        15 => Some(("length".into(), r_u8(raw, 0).to_string())),
        // op17: blockwalk=no (flag)
        17 => Some(("blockwalk".into(), "no".into())),
        // op18: blockrange=no (flag)
        18 => Some(("blockrange".into(), "no".into())),
        // op19: active (u8) — Note: for LOC this is a single u8, for SEQ it's multi-instance
        19 => Some(("active".into(), r_u8(raw, 0).to_string())),
        // op21: hillskew=yes (flag)
        21 => Some(("hillskew".into(), "yes".into())),
        // op24: anim (u16, 0xFFFF = null)
        24 => {
            let v = r_u16(raw, 0);
            if v == 0xFFFF {
                Some(("anim".into(), "null".into()))
            } else {
                Some(("anim".into(), v.to_string()))
            }
        }
        // op29: ambient (i8) — LOC version
        29 => Some(("ambient".into(), r_i8(raw, 0).to_string())),
        // op39: contrast (i8) — LOC version
        39 => Some(("contrast".into(), r_i8(raw, 0).to_string())),
        // op62: mirror (flag)
        62 => Some(("mirror".into(), "yes".into())),
        // op64: shadow=no (flag)
        64 => Some(("shadow".into(), "no".into())),
        // op65: resizex (u16)
        65 => Some(("resizex".into(), r_u16(raw, 0).to_string())),
        // op66: resizey (u16)
        66 => Some(("resizey".into(), r_u16(raw, 0).to_string())),
        // op67: resizez (u16)
        67 => Some(("resizez".into(), r_u16(raw, 0).to_string())),
        // op69: forceapproach (u8)
        69 => Some(("forceapproach".into(), r_u8(raw, 0).to_string())),
        // op70: offsetx (u16)
        70 => Some(("offsetx".into(), r_u16(raw, 0).to_string())),
        // op71: offsety (u16)
        71 => Some(("offsety".into(), r_u16(raw, 0).to_string())),
        // op72: offsetz (u16)
        72 => Some(("offsetz".into(), r_u16(raw, 0).to_string())),
        // op73: forcedecor (flag)
        73 => Some(("forcedecor".into(), "yes".into())),
        // op74: breakroutefinding (flag)
        74 => Some(("breakroutefinding".into(), "yes".into())),
        // op75: raiseobject (u8)
        75 => Some(("raiseobject".into(), r_u8(raw, 0).to_string())),

        // op1: model (u16) — for OBJ
        1 => Some(("model".into(), r_u16(raw, 0).to_string())),
        // op4: 2dzoom (u16)
        4 => Some(("2dzoom".into(), r_u16(raw, 0).to_string())),
        // op5: 2dxan (u16)
        5 => Some(("2dxan".into(), r_u16(raw, 0).to_string())),
        // op6: 2dyan (u16)
        6 => Some(("2dyan".into(), r_u16(raw, 0).to_string())),
        // op7: 2dxof (i16)
        7 => Some(("2dxof".into(), r_i16(raw, 0).to_string())),
        // op8: 2dyof (i16)
        8 => Some(("2dyof".into(), r_i16(raw, 0).to_string())),
        // op11: stackable (flag)
        11 => Some(("stackable".into(), "yes".into())),
        // op16: members (flag)
        16 => Some(("members".into(), "yes".into())),
        // op23: manwear (u16)
        23 => Some(("manwear".into(), r_u16(raw, 0).to_string())),
        // op25: womanwear (u16)
        25 => Some(("womanwear".into(), r_u16(raw, 0).to_string())),

        // op12 for OBJ: cost (i32) — conflicts with NPC op12 (size u8)
        // We can't distinguish here; the caller knows the type.
        // For safety, these ambiguous opcodes should be handled by
        // the type-specific emitter, not here.

        // op99 (LOC): cursor1 (u8, u16) — conflicts with NPC op99 (alwaysontop flag)
        // op100 (LOC): cursor2 (u8, u16) — conflicts with NPC op100 (ambient i8)

        // op101 (LOC): mapfunction (u8) — shadowed by NPC op101 (contrast)
        // above; the type-specific emitter handles LOC mapfunction.
        // op122 (NPC): hitbar (u16)
        122 => Some(("hitbar".into(), r_u16(raw, 0).to_string())),
        // op123 (NPC): height (u16)
        123 => Some(("height".into(), r_u16(raw, 0).to_string())),
        // op125 (NPC): respawndir (i8)
        125 => Some(("respawndir".into(), r_i8(raw, 0).to_string())),
        // op127 (NPC): readyanim (u16)
        127 => Some(("readyanim".into(), r_u16(raw, 0).to_string())),
        // op128 (NPC): unknown128 (u8)
        128 => Some(("unknown128".into(), r_u8(raw, 0).to_string())),

        // op19 (SEQ): idx:val pair — already pushed to Vec, handled by emitter
        // op20 (SEQ): idx:a:b triple — already pushed to Vec, handled by emitter

        // op82: hidden (flag)
        82 => Some(("hidden".into(), "yes".into())),
        // op88: nominimap_visible (flag)
        88 => Some(("nominimap_visible".into(), "yes".into())),
        // op89: noblocks_npcs (flag)
        89 => Some(("noblocks_npcs".into(), "yes".into())),
        // op90: op90 (flag)
        90 => Some(("op90".into(), "yes".into())),
        // op91: members_only (flag)
        91 => Some(("members_only".into(), "yes".into())),
        // op104: interaction_sides (u8)
        104 => Some(("interaction_sides".into(), r_u8(raw, 0).to_string())),
        // op105: op105 (flag)
        105 => Some(("op105".into(), "yes".into())),
        // op107: mapelement (u16)
        107 => Some(("mapelement".into(), r_u16(raw, 0).to_string())),

        // op164..167: i16/u16 values
        164 => Some(("op164".into(), r_i16(raw, 0).to_string())),
        165 => Some(("op165".into(), r_i16(raw, 0).to_string())),
        166 => Some(("op166".into(), r_i16(raw, 0).to_string())),
        167 => Some(("op167".into(), r_u16(raw, 0).to_string())),
        // op168..169: flags
        168 => Some(("op168".into(), "yes".into())),
        169 => Some(("op169".into(), "yes".into())),
        // op177: flag
        177 => Some(("op177".into(), "yes".into())),
        // op178: u8
        178 => Some(("op178".into(), r_u8(raw, 0).to_string())),

        _ => None,
    }
}

#[cfg(since_647)]
pub(super) fn decode_raw_params(raw_opcodes: &[(u8, Vec<u8>)]) -> Vec<ParamValue> {
    let Some((_, data)) = raw_opcodes.iter().find(|(op, _)| *op == 249) else {
        return Vec::new();
    };
    if data.is_empty() {
        return Vec::new();
    }
    let count = data[0] as usize;
    let mut pos = 1;
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        if pos >= data.len() {
            break;
        }
        let is_string = data[pos] == 1;
        pos += 1;
        pos += 3; // skip key
        if is_string {
            let start = pos;
            while pos < data.len() && data[pos] != STRING_TERMINATOR {
                pos += 1;
            }
            let s = String::from_utf8_lossy(&data[start..pos]).to_string();
            pos += 1;
            result.push(ParamValue::String(s.into_boxed_str()));
        } else {
            if pos + 3 < data.len() {
                let v =
                    i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                result.push(ParamValue::Int(v));
            }
            pos += 4;
        }
    }
    result
}
