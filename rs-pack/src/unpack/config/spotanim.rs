#[cfg(before_647)]
use super::ModelCategory;
use super::UnpackedPacks;
use crate::cache::spotanim::SpotAnimTypeRaw;
use std::collections::HashMap;

pub fn emit_spotanim_props(
    def: &SpotAnimTypeRaw,
    opcode: u8,
    _data: &[u8],
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    #[cfg(since_647)]
    let recol_s = def.recol_s.as_deref().unwrap_or_default();
    #[cfg(since_647)]
    let recol_d = def.recol_d.as_deref().unwrap_or_default();
    #[cfg(since_647)]
    let retex_s = def.retex_s.as_deref().unwrap_or_default();
    #[cfg(since_647)]
    let retex_d = def.retex_d.as_deref().unwrap_or_default();
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(before_647)]
    {
        if opcode == 1 {
            super::model_ref(def.model, "spotanim", ModelCategory::Spotanim, registry);
        }
        if def.hasalpha {
            p.push((3, "hasalpha".into(), "yes".into()));
        }
        for (base, suffix, colours) in [(40, "s", &def.recol_s), (50, "d", &def.recol_d)] {
            if let Some(colours) = colours {
                for (slot, &colour) in colours.iter().enumerate() {
                    p.push((
                        base + slot as u8,
                        format!("recol{}{suffix}", slot + 1),
                        _reverse_hsl
                            .get(&colour)
                            .copied()
                            .unwrap_or(colour)
                            .to_string(),
                    ));
                }
            }
        }
    }
    if opcode == 1 {
        let name = registry
            .config_name("model", def.model as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("model_{}", def.model));
        p.push((1, "model".into(), name));
    }
    if let Some(anim) = def.anim {
        let name = registry
            .config_name("seq", anim as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("seq_{}", anim));
        p.push((2, "anim".into(), name));
    }
    if opcode == 4 {
        p.push((4, "resizeh".into(), def.resizeh.to_string()));
    }
    if opcode == 5 {
        p.push((5, "resizev".into(), def.resizev.to_string()));
    }
    if opcode == 6 {
        #[cfg(before_647)]
        p.push((6, "angle".into(), def.angle.to_string()));
        #[cfg(since_647)]
        p.push((6, "rotation".into(), def.angle.to_string()));
    }
    if opcode == 7 {
        p.push((7, "ambient".into(), def.ambient.to_string()));
    }
    if opcode == 8 {
        p.push((8, "contrast".into(), def.contrast.to_string()));
    }
    #[cfg(since_647)]
    {
        for (code, set, key) in [
            (9, def.op9, "unknown9"),
            (10, def.op10, "allowloop"),
            (11, def.op11, "unknown11"),
            (12, def.op12, "unknown12"),
            (13, def.op13, "unknown13"),
        ] {
            if set {
                p.push((code, key.into(), "yes".into()));
            }
        }
        if opcode == 14 {
            p.push((14, "unknown14".into(), def.op14.unwrap().to_string()));
        }
        if opcode == 15 {
            p.push((15, "unknown15".into(), def.op15.unwrap().to_string()));
        }
        if opcode == 16 {
            p.push((16, "unknown16".into(), def.op16.unwrap_or(-1).to_string()));
        }
        for (i, (&s, &d)) in recol_d.iter().zip(recol_s).enumerate() {
            let [(ks, vs), (kd, vd)] = super::recol_pair(i + 1, s, d, _reverse_hsl);
            p.push((40, ks, vs));
            p.push((40, kd, vd));
        }
        for (i, (&s, &d)) in retex_d.iter().zip(retex_s).enumerate() {
            let n = i + 1;
            p.push((41, format!("retex{n}s"), registry.named("texture", s)));
            p.push((41, format!("retex{n}d"), registry.named("texture", d)));
        }
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
