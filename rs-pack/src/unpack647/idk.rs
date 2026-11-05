use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::idk::IdkTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::pack_registry::PackRegistry;

const ARCHIVE_CONFIG: u8 = 2;
const GROUP_IDK: u32 = 3;

pub fn unpack_idks(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let Some(group) = store.read_group(ARCHIVE_CONFIG, GROUP_IDK)? else {
        info!("  Archive 2 group {GROUP_IDK} (idk) absent — skipping");
        return Ok(());
    };

    let out_path = output_dir.join("all.idk");
    let mut file = std::fs::File::create(&out_path)?;

    // Sort entries by id for deterministic output.
    let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
    sorted.sort_by_key(|e| e.id);

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for entry in sorted {
        if entry.data.is_empty() {
            continue;
        }
        let mut def = IdkTypeRaw::new(entry.id as u16);
        def.decode(&mut Packet::from(entry.data.to_vec()));

        let base = registry
            .idk
            .get_by_id(entry.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("idk_{}", entry.id));
        let section = if seen.contains_key(&base) {
            format!("{base}__{}", entry.id)
        } else {
            base
        };
        seen.insert(section.clone(), entry.id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_idk_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} idk configs to {}", out_path.display());
    Ok(())
}

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

fn model_name(registry: &PackRegistry, id: u16) -> String {
    registry
        .model
        .get_by_id(id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}"))
}

pub fn emit_idk_props(def: &IdkTypeRaw, registry: &PackRegistry) -> Vec<(String, String)> {
    let models = def.models.as_deref().unwrap_or_default();
    let recol_s = def.recol_s.as_deref().unwrap_or_default();
    let recol_d = def.recol_d.as_deref().unwrap_or_default();
    let retex_s = def.retex_s.as_deref().unwrap_or_default();
    let retex_d = def.retex_d.as_deref().unwrap_or_default();
    let heads = def.heads.as_deref().unwrap_or_default();
    let mut p: Vec<(u8, String, String)> = Vec::new();
    // op 1 — body part type
    if def.opcode_order.contains(&1) {
        let value = bodytype_name(def.body_type as u8)
            .map(str::to_string)
            .unwrap_or_else(|| (def.body_type as u8).to_string());
        p.push((1, "type".into(), value));
    }
    // op 2 — models
    if !models.is_empty() {
        for (i, id) in models.iter().enumerate() {
            p.push((2, format!("model{}", i + 1), model_name(registry, *id)));
        }
    }
    // op 3 — disable
    if def.disable {
        p.push((3, "disable".into(), "yes".into()));
    }
    // op 40 — recolour pairs
    if !recol_s.is_empty() {
        for i in 0..recol_s.len() {
            let n = i + 1;
            p.push((40, format!("recol{n}s"), recol_s[i].to_string()));
            p.push((40, format!("recol{n}d"), recol_d[i].to_string()));
        }
    }
    // op 41 — retexture pairs
    if !retex_s.is_empty() {
        for i in 0..retex_s.len() {
            let n = i + 1;
            p.push((41, format!("retex{n}s"), retex_s[i].to_string()));
            p.push((41, format!("retex{n}d"), retex_d[i].to_string()));
        }
    }
    // ops 60-69 — head models (each slot tagged with its actual opcode)
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
