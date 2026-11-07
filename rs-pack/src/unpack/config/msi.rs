use super::{UnpackedPacks, format_colour};
use crate::cache::msi::MsiType;
use std::collections::HashMap;

pub fn emit_msi_props(
    def: &MsiType,
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        p.push((1, "graphic".into(), packs.reference("sprites", def.sprite)));
    }
    if def.opcode_order.contains(&2) {
        p.push((2, "colour".into(), format_colour(def.colour)));
    }
    if def.locsize {
        p.push((3, "resize".into(), "yes".into()));
    }
    if def.nosprite {
        p.push((4, "graphic".into(), "null".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
