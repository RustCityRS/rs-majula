use super::UnpackedPacks;
use crate::cache::light::LightType;
use std::collections::HashMap;

pub fn emit_light_props(
    def: &LightType,
    _packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let has = |code| def.opcode_order.contains(&code);
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if has(1) {
        p.push((1, "function".into(), def.function.to_string()));
    }
    if has(2) {
        p.push((2, "frequency".into(), def.speed.to_string()));
    }
    if has(3) {
        p.push((3, "amplitude".into(), def.amplitude.to_string()));
    }
    if has(4) {
        p.push((4, "offset".into(), def.base.to_string()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
