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
    opcode: u8,
    _data: &[u8],
    _packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if opcode == 1 {
        p.push((1, "coneangle".into(), joined(&def.angles)));
    }
    if opcode == 2 {
        p.push((2, "unknown2".into(), def.unused2.to_string()));
    }
    if opcode == 3 {
        p.push((3, "speed".into(), joined(&def.speed)));
    }
    if opcode == 4 {
        p.push((
            4,
            "speedcontrol".into(),
            format!("{},{}", def.speed_change_type, def.speed_change),
        ));
    }
    if opcode == 5 {
        p.push((5, "size".into(), def.size.to_string()));
    }
    if opcode == 6 {
        p.push((6, "rgba".into(), joined(&def.colour.map(|v| v as i32))));
    }
    if opcode == 7 {
        p.push((7, "lifetime".into(), joined(&def.lifetime)));
    }
    if opcode == 8 {
        p.push((8, "emissionrate".into(), joined(&def.rate)));
    }
    if opcode == 9 {
        p.push((9, "worldeffectors".into(), joined(&def.effectors)));
    }
    if opcode == 10 {
        p.push((10, "localeffectors".into(), joined(&def.effectors10)));
    }
    if opcode == 12 {
        p.push((12, "killbelowlevel".into(), def.priority12.to_string()));
    }
    if opcode == 13 {
        p.push((13, "killabovelevel".into(), def.priority13.to_string()));
    }
    if opcode == 14 {
        p.push((14, "unknown14".into(), def.max_particles.to_string()));
    }
    if opcode == 15 {
        p.push((15, "material".into(), def.material.unwrap().to_string()));
    }
    if opcode == 16 {
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
    if opcode == 17 {
        p.push((17, "unknown17".into(), def.material17.unwrap().to_string()));
    }
    if opcode == 18 {
        p.push((
            18,
            "rgbatarget".into(),
            (def.target_colour as i32).to_string(),
        ));
    }
    if opcode == 19 {
        p.push((19, "unknown19".into(), def.colour_percent.to_string()));
    }
    if opcode == 20 {
        p.push((
            20,
            "rgbtransitionpercent".into(),
            def.alpha_percent.to_string(),
        ));
    }
    if opcode == 21 {
        p.push((
            21,
            "alphatransitionpercent".into(),
            def.speed_percent.to_string(),
        ));
    }
    if opcode == 22 {
        p.push((
            22,
            "speedtarget".into(),
            def.target_speed.unwrap_or(-1).to_string(),
        ));
    }
    if opcode == 23 {
        p.push((
            23,
            "speedtargetpercent".into(),
            def.size_percent.to_string(),
        ));
    }
    if opcode == 25 {
        p.push((25, "globaleffectors".into(), joined(&def.globaleffectors)));
    }
    if opcode == 27 {
        p.push((
            27,
            "sizetarget".into(),
            def.target_size.unwrap().to_string(),
        ));
    }
    if opcode == 28 {
        p.push((
            28,
            "sizetargetpercent".into(),
            def.target_size_percent.to_string(),
        ));
    }
    if opcode == 29 {
        p.push((29, "unknown29".into(), def.unused29.to_string()));
    }
    if opcode == 31 {
        p.push((31, "size".into(), joined(&def.size_range)));
    }
    let flag = match opcode {
        24 => Some(("linkcolourchannels", "no")),
        26 => Some(("enabled", "no")),
        30 => Some(("unknown30", "yes")),
        32 => Some(("lighting", "no")),
        33 => Some(("unknown33", "yes")),
        34 => Some(("unknown34", "no")),
        _ => None,
    };
    if let Some((name, value)) = flag {
        p.push((opcode, name.into(), value.into()));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
