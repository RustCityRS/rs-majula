use super::UnpackedPacks;
use crate::cache::bas::BasType;
use std::collections::HashMap;

fn anim_key(code: u8) -> String {
    match code {
        2 => "crawlanim".into(),
        3 => "crawlanim_b".into(),
        4 => "crawlanim_l".into(),
        5 => "crawlanim_r".into(),
        6 => "runanim".into(),
        7 => "runanim_b".into(),
        8 => "runanim_l".into(),
        9 => "runanim_r".into(),
        38 => "readyanim_l".into(),
        39 => "readyanim_r".into(),
        40 => "walkanim_b".into(),
        41 => "walkanim_l".into(),
        42 => "walkanim_r".into(),
        46 => "crawlturn_l".into(),
        47 => "crawlturn_r".into(),
        48 => "runturn_l".into(),
        49 => "runturn_r".into(),
        50 => "walkturn_l".into(),
        51 => "walkturn_r".into(),
        _ => format!("unknown{code}"),
    }
}

pub fn emit_bas_props(
    def: &BasType,
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let has = |code| def.opcode_order.contains(&code);
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if has(1) {
        let ready = packs.reference("seq", def.readyanim);
        let walk = packs.reference("seq", def.walkanim);
        p.push((1, "readyanim".into(), format!("{ready},{walk}")));
    }
    for (code, anim) in &def.anims {
        p.push((*code, anim_key(*code), packs.reference("seq", *anim)));
    }
    if has(26) {
        let (x, z) = def.rotate;
        p.push((26, "hillrotate".into(), format!("{x},{z}")));
    }
    for (slot, values) in &def.transform {
        let values = values.map(|v| v.to_string()).join(",");
        p.push((27, "transform".into(), format!("{slot},{values}")));
    }
    if has(28) {
        let values: Vec<_> = def.op28.iter().map(u8::to_string).collect();
        p.push((28, "unknown28".into(), values.join(",")));
    }
    for (code, key, value) in [
        (29, "turnspeed", def.turnspeed.to_string()),
        (30, "turnmax", def.turnmax.to_string()),
        (31, "rollspeed", def.rollspeed.to_string()),
        (32, "rollmax", def.rollmax.to_string()),
        (33, "roll", def.roll.to_string()),
        (34, "pitchspeed", def.pitchspeed.to_string()),
        (35, "pitchmax", def.pitchmax.to_string()),
        (36, "pitch", def.pitch.to_string()),
        (37, "walkspeed", def.walkspeed.to_string()),
    ] {
        if has(code) {
            p.push((code, key.into(), value));
        }
    }
    for (anim, weight) in &def.readyanim_random {
        let anim = packs.reference("seq", *anim);
        p.push((52, "randomreadyanim".into(), format!("{anim},{weight}")));
    }
    if def.noshadow {
        p.push((53, "unknown53".into(), "no".into()));
    }
    if has(54) {
        let (a, b) = def.op54;
        p.push((54, "hillrotatelimit".into(), format!("{a},{b}")));
    }
    for (slot, value) in &def.op55 {
        p.push((55, "unknown55".into(), format!("{slot},{value}")));
    }
    for (slot, values) in &def.slotoffset {
        let values = values.map(|v| v.to_string()).join(",");
        p.push((56, "slotoffset".into(), format!("{slot},{values}")));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
