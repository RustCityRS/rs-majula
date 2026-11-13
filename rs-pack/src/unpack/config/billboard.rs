use super::UnpackedPacks;
use crate::cache::billboard::BillboardType;
use std::collections::HashMap;

pub fn emit_billboard_props(
    def: &BillboardType,
    opcode: u8,
    _data: &[u8],
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if opcode == 1 {
        p.push((
            1,
            "material".into(),
            packs.reference("texture", def.material.unwrap_or(u16::MAX)),
        ));
    }
    if opcode == 2 {
        p.push((
            2,
            "size".into(),
            format!("{},{}", def.width_minus_one, def.height_minus_one),
        ));
    }
    if opcode == 3 {
        p.push((3, "unknown3".into(), def.unused.to_string()));
    }
    if opcode == 4 {
        p.push((4, "sourceblendmode".into(), def.blend_src.to_string()));
    }
    if opcode == 5 {
        p.push((5, "destinationblendmode".into(), def.blend_dst.to_string()));
    }
    if opcode == 6 {
        p.push((6, "unknown6".into(), "yes".into()));
    }
    if opcode == 7 {
        p.push((7, "unknown7".into(), "yes".into()));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
