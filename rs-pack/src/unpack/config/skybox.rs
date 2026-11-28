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
    let prop = match opcode {
        1 => def
            .material
            .map(|id| ("material", packs.reference("texture", id))),
        2 => def.op2.as_ref().map(|ids| {
            let ids = ids.iter().map(u16::to_string).collect::<Vec<_>>();
            ("op2", ids.join(","))
        }),
        3 => def.op3.map(|v| ("op3", v.to_string())),
        4 => def.fillmode.map(|v| ("fillmode", v.to_string())),
        5 => def
            .staticmodel
            .map(|id| ("staticmodel", super::bigid_name(packs, "model", id))),
        _ => None,
    };
    prop.map(|(key, value)| (key.to_owned(), value))
        .into_iter()
        .collect()
}
