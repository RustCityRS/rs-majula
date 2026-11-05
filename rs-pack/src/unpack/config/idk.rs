#[cfg(before_647)]
use super::ModelCategory;
use super::UnpackedPacks;
use crate::cache::idk::IdkTypeRaw;
use std::collections::HashMap;

fn bodytype_name(v: u8) -> Option<&'static str> {
    Some(match v {
        0 => "man_hair",
        1 => "man_jaw",
        2 => "man_torso",
        3 => "man_arms",
        4 => "man_hands",
        5 => "man_legs",
        6 => "man_feet",
        7 => "woman_hair",
        8 => "woman_jaw",
        9 => "woman_torso",
        10 => "woman_arms",
        11 => "woman_hands",
        12 => "woman_legs",
        13 => "woman_feet",
        _ => return None,
    })
}

fn model_name(registry: &UnpackedPacks, id: u16) -> String {
    registry
        .config_name("model", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}"))
}

pub fn emit_idk_props(
    def: &IdkTypeRaw,
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let models = def.models.as_deref().unwrap_or_default();
    #[cfg(since_647)]
    let recol_s = def.recol_s.as_deref().unwrap_or_default();
    #[cfg(since_647)]
    let recol_d = def.recol_d.as_deref().unwrap_or_default();
    #[cfg(since_647)]
    let retex_s = def.retex_s.as_deref().unwrap_or_default();
    #[cfg(since_647)]
    let retex_d = def.retex_d.as_deref().unwrap_or_default();
    let heads = def.heads.as_deref().unwrap_or_default();
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(before_647)]
    {
        let cat = if def.body_type as u8 >= 7 {
            ModelCategory::IdkWoman
        } else {
            ModelCategory::IdkMan
        };
        for &id in models {
            super::model_ref(id, "idk", cat.clone(), registry);
        }
        for (slot, &id) in heads.iter().enumerate() {
            if def.opcode_order.contains(&(60 + slot as u8)) {
                super::model_ref(id, "idk_head", cat.clone(), registry);
            }
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

    if def.opcode_order.contains(&1) {
        let value = bodytype_name(def.body_type as u8)
            .map(str::to_string)
            .unwrap_or_else(|| (def.body_type as u8).to_string());
        p.push((1, "type".into(), value));
    }

    if !models.is_empty() {
        for (i, id) in models.iter().enumerate() {
            p.push((2, format!("model{}", i + 1), model_name(registry, *id)));
        }
    }

    if def.disable {
        p.push((3, "disable".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if !recol_s.is_empty() {
        for i in 0..recol_s.len() {
            let n = i + 1;
            p.push((40, format!("recol{n}s"), recol_s[i].to_string()));
            p.push((40, format!("recol{n}d"), recol_d[i].to_string()));
        }
    }

    #[cfg(since_647)]
    if !retex_s.is_empty() {
        for i in 0..retex_s.len() {
            let n = i + 1;
            p.push((41, format!("retex{n}s"), retex_s[i].to_string()));
            p.push((41, format!("retex{n}d"), retex_d[i].to_string()));
        }
    }

    for (slot, &model_id) in heads.iter().enumerate() {
        if def.opcode_order.contains(&((60 + slot) as u8)) {
            p.push((
                (60 + slot) as u8,
                format!("head{}", slot + 1),
                model_name(registry, model_id),
            ));
        }
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
