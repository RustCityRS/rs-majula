//! Unpack Idk (identity kit) configs from JS5 cache to text config
//! format.
//!
//! Produces `_unpack/all.idk` such that `pack::idk::pack_idk_def` reading
//! the file produces byte-identical cache bytes for every idk. Mirrors
//! 2004scape's `tools/unpack/config/IdkConfig.ts` text shape:
//! `type=<bodytype_name>`, `disable=yes`, `modelN=<model_name>`,
//! `recolNs=`/`recolNd=`, `retexNs=`/`retexNd=`, `headN=<model_name>`.
//!
//! Section header: `registry.idk.get_by_id(id)` || `idk_<id>`. Idk has no
//! op 250 / DebugName variant — the section header is the only identity.
//! Collision-renamed sections use `<name>__<id>`.
//!
//! Phase 1 contract: this writes EVERY cache entry, every emission, in
//! source order — no overlay-key gating.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache647::{CacheType, IdkType};
use crate::pack::idk::emit_idk_props;
use crate::pack_registry::PackRegistry;

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
        let mut def = IdkType::new(entry.id as u16);
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
