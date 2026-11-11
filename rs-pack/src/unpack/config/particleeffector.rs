use super::UnpackedPacks;
use crate::cache::particleeffector::ParticleEffectorType;
use std::collections::HashMap;

pub fn emit_particleeffector_props(
    def: &ParticleEffectorType,
    opcode: u8,
    _data: &[u8],
    _packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if opcode == 1 {
        p.push((1, "angle".into(), def.angle.to_string()));
    }
    if opcode == 2 {
        p.push((2, "unknown2".into(), def.unused.to_string()));
    }
    if opcode == 3 {
        p.push((
            3,
            "forcedirection".into(),
            format!("{},{},{}", def.force[0], def.force[1], def.force[2]),
        ));
    }
    if opcode == 4 {
        p.push((
            4,
            "falloff".into(),
            format!("{},{}", def.falloff_type, def.falloff),
        ));
    }
    if opcode == 6 {
        p.push((6, "forcetype".into(), def.effect_type.to_string()));
    }
    if opcode == 8 {
        p.push((8, "unknown8".into(), "yes".into()));
    }
    if opcode == 9 {
        p.push((9, "unknown9".into(), "yes".into()));
    }
    if opcode == 10 {
        p.push((10, "unknown10".into(), "yes".into()));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
