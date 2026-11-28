use super::UnpackedPacks;
use crate::cache::bas::BasType;
use rs_ver::TypeId;
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
        #[cfg(since_647)]
        46 => "crawlturn_l".into(),
        #[cfg(since_647)]
        47 => "crawlturn_r".into(),
        #[cfg(since_647)]
        48 => "runturn_l".into(),
        #[cfg(since_647)]
        49 => "runturn_r".into(),
        #[cfg(since_647)]
        50 => "walkturn_l".into(),
        #[cfg(since_647)]
        51 => "walkturn_r".into(),
        _ => format!("unknown{code}"),
    }
}

pub fn emit_bas_props(
    def: &BasType,
    opcode: u8,
    _data: &[u8],
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if opcode == 1 {
        let ready = packs.reference("seq", def.readyanim.unwrap_or(TypeId::NULL));
        let walk = packs.reference("seq", def.walkanim.unwrap_or(TypeId::NULL));
        p.push((1, "readyanim".into(), format!("{ready},{walk}")));
    }
    if let Some((code, anim)) = def.anims.last() {
        p.push((*code, anim_key(*code), packs.reference("seq", *anim)));
    }
    if opcode == 26 {
        let (x, z) = def.hill_rotate;
        p.push((26, "hillrotate".into(), format!("{x},{z}")));
    }
    if let Some((slot, values)) = def.transform.last() {
        let values = values.map(|v| v.to_string()).join(",");
        p.push((27, "transform".into(), format!("{slot},{values}")));
    }
    #[cfg(since_647)]
    if opcode == 28 {
        let values: Vec<_> = def
            .op28
            .iter()
            .map(|value| value.unwrap_or(u8::MAX).to_string())
            .collect();
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
    ] {
        if opcode == code {
            p.push((code, key.into(), value));
        }
    }
    if let Some(speed) = def.walkspeed {
        p.push((37, "walkspeed".into(), speed.to_string()));
    }
    if let Some(height) = def.overlayheight {
        p.push((45, "overlayheight".into(), height.to_string()));
    }
    #[cfg(since_647)]
    for (anim, weight) in &def.random_ready_anims {
        let anim = packs.reference("seq", *anim);
        p.push((52, "randomreadyanim".into(), format!("{anim},{weight}")));
    }
    #[cfg(since_647)]
    if def.noshadow {
        p.push((53, "unknown53".into(), "no".into()));
    }
    #[cfg(since_647)]
    if opcode == 54 {
        let (a, b) = def.hill_rotate_limit;
        p.push((54, "hillrotatelimit".into(), format!("{a},{b}")));
    }
    #[cfg(since_647)]
    if let Some((slot, value)) = def.op55.last() {
        p.push((55, "unknown55".into(), format!("{slot},{value}")));
    }
    #[cfg(since_647)]
    if let Some((slot, values)) = def.slotoffset.last() {
        let values = values.map(|v| v.to_string()).join(",");
        p.push((56, "slotoffset".into(), format!("{slot},{values}")));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
