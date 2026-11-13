use super::UnpackedPacks;
use crate::cache::hitmark::HitmarkType;
use std::collections::HashMap;

pub fn emit_hitmark_props(
    def: &HitmarkType,
    opcode: u8,
    _data: &[u8],
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if opcode == 1 {
        p.push((
            1,
            "damagefont".into(),
            packs.reference("fontmetrics", def.damagefont.unwrap()),
        ));
    }
    if opcode == 2 {
        p.push((2, "damagecolour".into(), def.damagecolour.to_string()));
    }
    for (code, key, sprite) in [
        (3, "classgraphic", def.classgraphic),
        (4, "leftgraphic", def.leftgraphic),
        (5, "middlegraphic", def.middlegraphic),
        (6, "rightgraphic", def.rightgraphic),
    ] {
        if opcode == code {
            p.push((
                code,
                key.into(),
                packs.reference("sprites", sprite.unwrap()),
            ));
        }
    }
    if opcode == 7 {
        p.push((7, "scrolltooffsetx".into(), def.scrolltooffsetx.to_string()));
    }
    if opcode == 8 {
        p.push((8, "damageformat".into(), def.damageformat.clone()));
    }
    if opcode == 9 {
        p.push((9, "sticktime".into(), def.sticktime.to_string()));
    }
    if opcode == 10 {
        p.push((
            10,
            "scrolltooffsety".into(),
            def.scrolltooffsety.to_string(),
        ));
    }
    if opcode == 11 {
        p.push((11, "fadeout".into(), "no".into()));
    }
    if opcode == 12 {
        p.push((
            12,
            "replacemode".into(),
            def.replacemode.unwrap().to_string(),
        ));
    }
    if opcode == 13 {
        p.push((13, "damageyof".into(), def.damageyof.to_string()));
    }
    if opcode == 14 {
        p.push((14, "fadeout".into(), def.fadeout.unwrap().to_string()));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
