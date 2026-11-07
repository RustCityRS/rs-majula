use super::UnpackedPacks;
use crate::cache::particleeffector::ParticleEffectorType;
use std::collections::HashMap;

pub fn emit_particleeffector_props(
    def: &ParticleEffectorType,
    _packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let has = |code| def.opcode_order.contains(&code);
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if has(1) {
        p.push((1, "angle".into(), def.angle.to_string()));
    }
    if has(2) {
        p.push((2, "unknown2".into(), def.unused.to_string()));
    }
    if has(3) {
        p.push((
            3,
            "forcedirection".into(),
            format!("{},{},{}", def.force[0], def.force[1], def.force[2]),
        ));
    }
    if has(4) {
        p.push((
            4,
            "falloff".into(),
            format!("{},{}", def.falloff_type, def.falloff),
        ));
    }
    if has(6) {
        p.push((6, "forcetype".into(), def.effect_type.to_string()));
    }
    if has(8) {
        p.push((8, "unknown8".into(), "yes".into()));
    }
    if has(9) {
        p.push((9, "unknown9".into(), "yes".into()));
    }
    if has(10) {
        p.push((10, "unknown10".into(), "yes".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
