use super::UnpackedPacks;
use crate::cache::flu::FluType;
use std::collections::HashMap;

pub fn emit_flu_props(
    def: &FluType,
    _registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        p.push((1, "color".into(), def.color.to_string()));
    }
    if def.opcode_order.contains(&2) {
        p.push((2, "texture".into(), def.texture.to_string()));
    }
    if def.opcode_order.contains(&3) {
        p.push((3, "size".into(), def.size.to_string()));
    }
    if def.op4 {
        p.push((4, "op4_clearbool201".into(), "yes".into()));
    }
    if def.op5 {
        p.push((5, "op5_clearbool202".into(), "yes".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
