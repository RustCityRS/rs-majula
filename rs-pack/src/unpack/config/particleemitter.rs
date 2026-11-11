use super::UnpackedPacks;
use crate::cache::particleemitter::ParticleEmitterType;
use std::collections::HashMap;

fn joined<T: ToString>(values: &[T]) -> String {
    values
        .iter()
        .map(T::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

pub fn emit_particleemitter_props(
    def: &ParticleEmitterType,
    _packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let has = |code| def.opcode_order.contains(&code);
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if has(1) {
        p.push((1, "coneangle".into(), joined(&def.angles)));
    }
    if has(2) {
        p.push((2, "unknown2".into(), def.unused2.to_string()));
    }
    if has(3) {
        p.push((3, "speed".into(), joined(&def.speed)));
    }
    if has(4) {
        p.push((
            4,
            "speedcontrol".into(),
            format!("{},{}", def.speed_change_type, def.speed_change),
        ));
    }
    if has(5) {
        p.push((5, "size".into(), def.size.to_string()));
    }
    if has(6) {
        p.push((6, "rgba".into(), joined(&def.colour.map(|v| v as i32))));
    }
    if has(7) {
        p.push((7, "lifetime".into(), joined(&def.lifetime)));
    }
    if has(8) {
        p.push((8, "emissionrate".into(), joined(&def.rate)));
    }
    if has(9) {
        p.push((9, "worldeffectors".into(), joined(&def.effectors)));
    }
    if has(10) {
        p.push((10, "localeffectors".into(), joined(&def.effectors10)));
    }
    if has(12) {
        p.push((12, "killbelowlevel".into(), def.priority12.to_string()));
    }
    if has(13) {
        p.push((13, "killabovelevel".into(), def.priority13.to_string()));
    }
    if has(14) {
        p.push((14, "unknown14".into(), def.max_particles.to_string()));
    }
    if has(15) {
        p.push((15, "material".into(), def.material.unwrap().to_string()));
    }
    if has(16) {
        p.push((
            16,
            "unknown16".into(),
            format!(
                "{},{},{},{}",
                def.cycle_start,
                def.cycle_offset.unwrap(),
                def.cycle_duration.unwrap(),
                def.cycle_repeat
            ),
        ));
    }
    if has(17) {
        p.push((17, "unknown17".into(), def.material17.unwrap().to_string()));
    }
    if has(18) {
        p.push((
            18,
            "rgbatarget".into(),
            (def.target_colour as i32).to_string(),
        ));
    }
    if has(19) {
        p.push((19, "unknown19".into(), def.colour_percent.to_string()));
    }
    if has(20) {
        p.push((
            20,
            "rgbtransitionpercent".into(),
            def.alpha_percent.to_string(),
        ));
    }
    if has(21) {
        p.push((
            21,
            "alphatransitionpercent".into(),
            def.speed_percent.to_string(),
        ));
    }
    if has(22) {
        p.push((
            22,
            "speedtarget".into(),
            def.target_speed.unwrap_or(-1).to_string(),
        ));
    }
    if has(23) {
        p.push((
            23,
            "speedtargetpercent".into(),
            def.size_percent.to_string(),
        ));
    }
    if has(25) {
        p.push((25, "unknown25".into(), joined(&def.effectors25)));
    }
    if has(27) {
        p.push((
            27,
            "sizetarget".into(),
            def.target_size.unwrap().to_string(),
        ));
    }
    if has(28) {
        p.push((
            28,
            "sizetargetpercent".into(),
            def.target_size_percent.to_string(),
        ));
    }
    if has(29) {
        p.push((29, "unknown29".into(), def.unused29.to_string()));
    }
    if has(31) {
        p.push((31, "size".into(), joined(&def.size_range)));
    }
    // A flag can repeat in a record: one line per occurrence.
    for &code in &def.opcode_order {
        let (name, value) = match code {
            24 => ("linkcolourchannels", "no"),
            26 => ("enabled", "no"),
            30 => ("unknown30", "yes"),
            32 => ("lighting", "no"),
            33 => ("unknown33", "yes"),
            34 => ("unknown34", "no"),
            _ => continue,
        };
        p.push((code, name.into(), value.into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
