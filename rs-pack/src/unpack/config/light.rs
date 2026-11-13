use super::UnpackedPacks;
use crate::cache::light::LightType;
use std::collections::HashMap;

pub fn emit_light_props(
    def: &LightType,
    opcode: u8,
    _data: &[u8],
    _packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if opcode == 1 {
        p.push((1, "function".into(), def.function.to_string()));
    }
    if opcode == 2 {
        p.push((2, "frequency".into(), def.speed.to_string()));
    }
    if opcode == 3 {
        p.push((3, "amplitude".into(), def.amplitude.to_string()));
    }
    if opcode == 4 {
        p.push((4, "offset".into(), def.base.to_string()));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
