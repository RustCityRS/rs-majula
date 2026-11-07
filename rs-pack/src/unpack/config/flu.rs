use super::UnpackedPacks;
use crate::cache::flu::FluType;
use std::collections::HashMap;

pub fn emit_flu_props(
    def: &FluType,
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        p.push((1, "colour".into(), format!("0x{:x}", def.color)));
    }
    if def.opcode_order.contains(&2) {
        p.push((
            2,
            "material".into(),
            packs.reference("texture", def.texture),
        ));
    }
    if def.opcode_order.contains(&3) {
        p.push((3, "texturescale".into(), def.size.to_string()));
    }
    if def.op4 {
        p.push((4, "hardshadow".into(), "no".into()));
    }
    if def.op5 {
        p.push((5, "occlude".into(), "no".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
