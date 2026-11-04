//! Unpack Inv configs from JS5 cache to text config format.
//!
//! Produces `_unpack/all.inv` such that `pack::inv::pack_inv_def` reading
//! the file produces byte-identical cache bytes for every inv. Mirrors
//! 2004scape's `tools/unpack/config/InvConfig.ts` text shape (where they
//! had one — inv is server-only in 2004scape so we use their pack-config
//! key names): `scope=`, `size=`, `stockN=name,count[,rate]`, etc.
//!
//! Section header is the cache's debugname (op 250). Empty debugnames fall
//! back to `inv_<id>`; collision-renamed sections use `<dbg>__<id>`.
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

use crate::cache647::{CacheType, InvType};
use crate::pack::inv::emit_inv_props;
use crate::pack_registry::PackRegistry;

const ARCHIVE_CONFIG: u8 = 2;
const GROUP_INV: u32 = 5;

pub fn unpack_invs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let Some(group) = store.read_group(ARCHIVE_CONFIG, GROUP_INV)? else {
        info!("  Archive 2 group {GROUP_INV} (inv) absent — skipping");
        return Ok(());
    };

    let out_path = output_dir.join("all.inv");
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
        let mut def = InvType::new(entry.id as u16);
        let mut buf = Packet::from(entry.data.to_vec());
        def.decode(&mut buf);

        let base = registry
            .inv
            .get_by_id(entry.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("inv_{}", entry.id));
        let section = if seen.contains_key(&base) {
            format!("{base}__{}", entry.id)
        } else {
            base
        };
        seen.insert(section.clone(), entry.id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_inv_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} inv configs to {}", out_path.display());
    Ok(())
}
