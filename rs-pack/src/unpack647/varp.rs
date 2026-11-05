use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use super::param::char_to_type_name;
use crate::cache::provider::CacheType;
use crate::cache::varbit::VarbitType;
use crate::cache::varp::VarPlayerTypeRaw;
use crate::pack::pack_registry::PackRegistry;
use crate::types::VarPlayerScope;

const ARCHIVE_CONFIG: u8 = 2;
const GROUP_VARP: u32 = 16;
const ARCHIVE_VARBITS: u8 = 22;
const STRIDE_VARBITS: u32 = 1024;

/// Resolve a set of varbit IDs to their backing varp IDs via the cache.
fn resolve_varbit_varps(cache_dir: &Path, varbit_ids: &HashSet<u32>) -> Result<HashSet<u32>> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_VARBITS).unwrap_or(0);
    let mut varp_ids = HashSet::new();

    for group_id in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_VARBITS, group_id)? else {
            continue;
        };
        for entry in &group.entries {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = group_id * STRIDE_VARBITS + entry.id;
            if !varbit_ids.contains(&global_id) {
                continue;
            }
            let mut def = VarbitType::new(global_id as u16);
            def.decode(&mut Packet::from(entry.data.to_vec()));
            varp_ids.insert(def.basevar as u32);
        }
    }

    Ok(varp_ids)
}

pub fn unpack_varps(
    cache_dir: &Path,
    registry: &PackRegistry,
    output_dir: &Path,
    cs2_dir: &Path,
) -> Result<()> {
    // Scan decompiled CS2 source for %varp / %varbit references
    let (mut transmit_varps, cs2_varbits) = super::cs2::scan_cs2_varp_refs(cs2_dir)?;
    let direct_count = transmit_varps.len();
    let varbit_count = cs2_varbits.len();

    let varbit_varp_count = if !cs2_varbits.is_empty() {
        let varbit_varps = resolve_varbit_varps(cache_dir, &cs2_varbits)?;
        let added = varbit_varps
            .iter()
            .filter(|id| !transmit_varps.contains(id))
            .count();
        transmit_varps.extend(varbit_varps);
        added
    } else {
        0
    };

    // Report
    let mut sorted_ids: Vec<u32> = transmit_varps.iter().copied().collect();
    sorted_ids.sort();
    info!(
        "  CS2 varp scan: {} direct varps, {} varbits → {} additional varps, {} total transmit",
        direct_count,
        varbit_count,
        varbit_varp_count,
        transmit_varps.len()
    );
    for &id in &sorted_ids {
        let name = registry.varp.get_by_id(id).unwrap_or("?");
        info!("    varp {id} ({name}) — transmit=yes");
    }

    // Unpack cache entries
    let mut store = CacheStore::open(cache_dir)?;
    let Some(group) = store.read_group(ARCHIVE_CONFIG, GROUP_VARP)? else {
        info!("  Archive 2 group {GROUP_VARP} (varp) absent — skipping");
        return Ok(());
    };

    let out_path = output_dir.join("all.varp");
    let mut file = std::fs::File::create(&out_path)?;

    let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
    sorted.sort_by_key(|e| e.id);

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for entry in sorted {
        if entry.data.is_empty() {
            continue;
        }
        let mut def = VarPlayerTypeRaw::new(entry.id as u16);
        def.decode(&mut Packet::from(entry.data.to_vec()));

        // Mark transmit if CS2 references this varp
        if transmit_varps.contains(&entry.id) && !def.transmit {
            def.transmit = true;
            if !def.opcode_order.is_empty() {
                def.opcode_order.push(6);
            }
        }

        let base = registry
            .varp
            .get_by_id(entry.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("varp_{}", entry.id));
        let section = if seen.contains_key(&base) {
            format!("{base}__{}", entry.id)
        } else {
            base
        };
        seen.insert(section.clone(), entry.id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_varp_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} varp configs to {}", out_path.display());
    Ok(())
}

pub fn emit_varp_props(def: &VarPlayerTypeRaw, _registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    match def.scope {
        VarPlayerScope::Perm => p.push((1, "scope".into(), "perm".into())),
        VarPlayerScope::Temp => {}
    }
    if def.opcode_order.contains(&2) {
        p.push((2, "type".into(), char_to_type_name(def.var_type as u8)));
    }
    if !def.protect {
        p.push((4, "protect".into(), "no".into()));
    }
    if def.clientcode != 0 {
        p.push((5, "clientcode".into(), def.clientcode.to_string()));
    }
    if def.transmit {
        p.push((6, "transmit".into(), "yes".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
