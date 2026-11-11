use super::UnpackedPacks;
use crate::cache::billboard::BillboardType;
use std::collections::HashMap;

pub fn emit_billboard_props(
    def: &BillboardType,
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let has = |code| def.opcode_order.contains(&code);
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if has(1) {
        p.push((
            1,
            "material".into(),
            packs.reference("texture", def.material.unwrap_or(u16::MAX)),
        ));
    }
    if has(2) {
        p.push((
            2,
            "size".into(),
            format!("{},{}", def.width_minus_one, def.height_minus_one),
        ));
    }
    if has(3) {
        p.push((3, "unknown3".into(), def.unused.to_string()));
    }
    if has(4) {
        p.push((4, "sourceblendmode".into(), def.blend_src.to_string()));
    }
    if has(5) {
        p.push((5, "destinationblendmode".into(), def.blend_dst.to_string()));
    }
    if has(6) {
        p.push((6, "unknown6".into(), "yes".into()));
    }
    if has(7) {
        p.push((7, "unknown7".into(), "yes".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
