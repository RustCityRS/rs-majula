use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::flo::FloTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::pack_registry::PackRegistry;

const ARCHIVE_CONFIG: u8 = 2;
const GROUP_FLO: u32 = 4;

pub fn unpack_flos(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let Some(group) = store.read_group(ARCHIVE_CONFIG, GROUP_FLO)? else {
        info!("  Archive 2 group {GROUP_FLO} (flo) absent — skipping");
        return Ok(());
    };

    let out_path = output_dir.join("all.flo");
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
        let mut def = FloTypeRaw::new(entry.id as u16);
        let mut buf = Packet::from(entry.data.to_vec());
        def.decode(&mut buf);

        let base = registry
            .flo
            .get_by_id(entry.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("flo_{}", entry.id));
        let section = if seen.contains_key(&base) {
            format!("{base}__{}", entry.id)
        } else {
            base
        };
        seen.insert(section.clone(), entry.id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_flo_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} flo configs to {}", out_path.display());
    Ok(())
}

pub fn emit_flo_props(def: &FloTypeRaw, _registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        p.push((1, "color".into(), def.colour.to_string()));
    }
    if def.opcode_order.contains(&2) {
        p.push((2, "texture_u8".into(), def.texture_u8_raw.to_string()));
    }
    if def.opcode_order.contains(&3) {
        p.push((3, "texture".into(), def.texture_u16_raw.to_string()));
    }
    if !def.occlude {
        p.push((5, "op5_occlude_false".into(), "yes".into()));
    }
    if def.water_color >= 0 {
        p.push((
            7,
            "water_color".into(),
            (def.water_color as u32).to_string(),
        ));
    }
    if def.op8_flomark {
        p.push((8, "op8_flomark".into(), "yes".into()));
    }
    if let Some(v) = def.op9_u16 {
        p.push((9, "op9_u16".into(), v.to_string()));
    }
    if def.op10_bool34_false {
        p.push((10, "op10_bool34_false".into(), "yes".into()));
    }
    for v in &def.op11_values {
        p.push((11, "op11_u8".into(), v.to_string()));
    }
    if def.op12_bool33_true {
        p.push((12, "op12_bool33_true".into(), "yes".into()));
    }
    if let Some(v) = def.op13_u24 {
        p.push((13, "op13_u24".into(), v.to_string()));
    }
    if let Some(v) = def.op14_u8 {
        p.push((14, "op14_u8".into(), v.to_string()));
    }
    if let Some(v) = def.op16_u8 {
        p.push((16, "op16_u8".into(), v.to_string()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
