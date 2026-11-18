use super::UnpackedPacks;
use crate::cache::headbar::HeadbarType;
use rs_ver::TypeId;
use std::collections::HashMap;

pub fn emit_headbar_props(
    def: &HeadbarType,
    opcode: u8,
    _data: &[u8],
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let graphic = |id: Option<TypeId>| id.map(|id| packs.reference("sprites", id));
    let prop = match opcode {
        1 => def.unknown1.map(|v| ("unknown1", v.to_string())),
        2 => def.display_order.map(|v| ("showpriority", v.to_string())),
        3 => def.priority.map(|v| ("hidepriority", v.to_string())),
        4 => Some(("fadeout", "no".to_owned())),
        5 => def.sticktime.map(|v| ("sticktime", v.to_string())),
        6 => def.unknown6.map(|v| ("unknown6", v.to_string())),
        7 => graphic(def.full).map(|v| ("full", v)),
        8 => graphic(def.empty).map(|v| ("empty", v)),
        9 => graphic(def.fullplayergroup).map(|v| ("fullplayergroup", v)),
        10 => graphic(def.emptyplayergroup).map(|v| ("emptyplayergroup", v)),
        11 => def.fadeout.map(|v| ("fadeout", v.to_string())),
        _ => None,
    };
    prop.map(|(key, value)| (key.to_owned(), value))
        .into_iter()
        .collect()
}
