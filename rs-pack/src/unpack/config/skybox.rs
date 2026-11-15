use super::UnpackedPacks;
use crate::cache::skybox::SkyboxType;
use std::collections::HashMap;

pub fn emit_skybox_props(
    def: &SkyboxType,
    opcode: u8,
    _data: &[u8],
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let xyz = |[x, y, z]: [i16; 3]| format!("{x},{y},{z}");
    let prop = match opcode {
        1 => def
            .material
            .map(|id| ("material", packs.reference("texture", id))),
        2 => def.op2.then(|| ("op2", "yes".to_owned())),
        3 => def.staticmodels.map(|v| ("staticmodels", xyz(v))),
        4 => def.fillmode.map(|v| ("fillmode", v.to_string())),
        5 => def
            .staticmodel
            .map(|id| ("staticmodel", super::bigid_name(packs, "model", id))),
        6 => def.op6.map(|v| ("op6", super::format_colour(v))),
        7 => def.op7.map(|v| ("op7", xyz(v))),
        _ => None,
    };
    prop.map(|(key, value)| (key.to_owned(), value))
        .into_iter()
        .collect()
}
