use super::UnpackedPacks;
use crate::cache::cursor::CursorType;
use std::collections::HashMap;

pub fn emit_cursor_props(
    def: &CursorType,
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        p.push((1, "graphic".into(), packs.reference("sprites", def.sprite)));
    }
    if def.opcode_order.contains(&2) {
        let (x, y) = def.hotspot;
        p.push((2, "hotspot".into(), format!("{x},{y}")));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
