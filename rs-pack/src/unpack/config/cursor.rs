use super::UnpackedPacks;
use crate::cache::cursor::CursorType;
use std::collections::HashMap;

pub fn emit_cursor_props(
    def: &CursorType,
    opcode: u8,
    _data: &[u8],
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if opcode == 1 {
        p.push((1, "graphic".into(), packs.reference("sprites", def.sprite)));
    }
    if opcode == 2 {
        let (x, y) = def.hotspot;
        p.push((2, "hotspot".into(), format!("{x},{y}")));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
