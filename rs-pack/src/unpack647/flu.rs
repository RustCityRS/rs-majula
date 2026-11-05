//! Unpack Flu (floor underlay) configs from JS5 cache to text config.
//!
//! Produces `_unpack/all.flu` such that `pack::flu::pack_flu_def` reading
//! the file produces byte-identical cache bytes for every flu. Mirrors
//! 2004scape's `tools/unpack/config/FloConfig.ts` text shape (flu has no
//! 2004scape client-cache TS — flo's pack-config key names apply since the
//! two share color/texture/size opcodes).
//!
//! Section header is the cache's debugname (op 250). Empty debugnames fall
//! back to `flu_<id>`; collision-renamed sections use `<dbg>__<id>`.
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

use crate::cache::flu::FluType;
use crate::cache::provider::CacheType;
use crate::pack::flu::emit_flu_props;
use crate::pack_registry::PackRegistry;

const ARCHIVE_CONFIG: u8 = 2;
const GROUP_FLU: u32 = 1;

pub fn unpack_flus(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let Some(group) = store.read_group(ARCHIVE_CONFIG, GROUP_FLU)? else {
        info!("  Archive 2 group {GROUP_FLU} (flu) absent — skipping");
        return Ok(());
    };

    let out_path = output_dir.join("all.flu");
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
        let mut def = FluType::new(entry.id as u16);
        let mut buf = Packet::from(entry.data.to_vec());
        def.decode(&mut buf);

        let base = registry
            .flu
            .get_by_id(entry.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("flu_{}", entry.id));
        let section = if seen.contains_key(&base) {
            format!("{base}__{}", entry.id)
        } else {
            base
        };
        seen.insert(section.clone(), entry.id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_flu_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} flu configs to {}", out_path.display());
    Ok(())
}
