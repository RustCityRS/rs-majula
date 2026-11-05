use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::provider::CacheType;
use crate::cache::spotanim::SpotAnimTypeRaw;
use crate::pack::pack_registry::PackRegistry;

const ARCHIVE_SPOTANIM: u8 = 21;
const STRIDE_SPOTANIM: u32 = 256;

pub fn unpack_spotanims(
    cache_dir: &Path,
    registry: &PackRegistry,
    output_dir: &Path,
) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_SPOTANIM).unwrap_or(0);

    let out_path = output_dir.join("all.spotanim");
    let mut file = std::fs::File::create(&out_path)?;

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_SPOTANIM, gid)? else {
            continue;
        };

        // Sort entries by id for deterministic output within a group.
        let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
        sorted.sort_by_key(|e| e.id);

        for entry in sorted {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = gid * STRIDE_SPOTANIM + entry.id;
            let mut def = SpotAnimTypeRaw::new(global_id as u16);
            let mut buf = Packet::from(entry.data.to_vec());
            def.decode(&mut buf);

            let base = registry
                .spotanim
                .get_by_id(global_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("spotanim_{global_id}"));
            let section = if seen.contains_key(&base) {
                format!("{base}__{global_id}")
            } else {
                base
            };
            seen.insert(section.clone(), global_id);

            writeln!(file, "[{section}]")?;
            for (k, v) in emit_spotanim_props(&def, registry) {
                writeln!(file, "{k}={v}")?;
            }
            writeln!(file)?;
            written += 1;
        }
    }

    info!(
        "  Wrote {written} spotanim configs to {}",
        out_path.display()
    );
    Ok(())
}

pub fn emit_spotanim_props(
    def: &SpotAnimTypeRaw,
    registry: &PackRegistry,
) -> Vec<(String, String)> {
    let recol_s = def.recol_s.as_deref().unwrap_or_default();
    let recol_d = def.recol_d.as_deref().unwrap_or_default();
    let retex_s = def.retex_s.as_deref().unwrap_or_default();
    let retex_d = def.retex_d.as_deref().unwrap_or_default();
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        let name = registry
            .model
            .get_by_id(def.model as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("model_{}", def.model));
        p.push((1, "model".into(), name));
    }
    if let Some(anim) = def.anim {
        let name = registry
            .seq
            .get_by_id(anim as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("seq_{}", anim));
        p.push((2, "anim".into(), name));
    }
    if def.opcode_order.contains(&4) || def.resizeh != 128 {
        p.push((4, "resizeh".into(), def.resizeh.to_string()));
    }
    if def.opcode_order.contains(&5) || def.resizev != 128 {
        p.push((5, "resizev".into(), def.resizev.to_string()));
    }
    if def.opcode_order.contains(&6) || def.angle != 0 {
        p.push((6, "angle".into(), def.angle.to_string()));
    }
    if def.opcode_order.contains(&7) || def.ambient != 0 {
        p.push((7, "ambient".into(), def.ambient.to_string()));
    }
    if def.opcode_order.contains(&8) || def.contrast != 0 {
        p.push((8, "contrast".into(), def.contrast.to_string()));
    }
    if def.op9 {
        p.push((9, "op9_animmode3_param8224".into(), "yes".into()));
    }
    if def.op10 {
        p.push((10, "op10_flag".into(), "yes".into()));
    }
    if def.op11 {
        p.push((11, "op11_animmode1".into(), "yes".into()));
    }
    if def.op12 {
        p.push((12, "op12_animmode4".into(), "yes".into()));
    }
    if def.op13 {
        p.push((13, "op13_animmode5".into(), "yes".into()));
    }
    if def.opcode_order.contains(&14) {
        p.push((14, "op14_animmode2_u8".into(), def.op14.to_string()));
    }
    if def.opcode_order.contains(&15) {
        p.push((15, "op15_animmode3_u16".into(), def.op15.to_string()));
    }
    if def.opcode_order.contains(&16) {
        p.push((16, "op16_animmode3_i32".into(), def.op16.to_string()));
    }
    if !recol_d.is_empty() {
        for i in 0..recol_d.len() {
            let n = i + 1;
            p.push((40, format!("recol{n}d"), recol_d[i].to_string()));
            p.push((40, format!("recol{n}s"), recol_s[i].to_string()));
        }
    }
    if !retex_d.is_empty() {
        for i in 0..retex_d.len() {
            let n = i + 1;
            p.push((41, format!("retex{n}d"), retex_d[i].to_string()));
            p.push((41, format!("retex{n}s"), retex_s[i].to_string()));
        }
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
