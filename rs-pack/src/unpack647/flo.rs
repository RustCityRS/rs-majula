//! Unpack Flo (floor overlay) configs from JS5 cache to text config.
//!
//! Produces `_unpack/all.flo` such that `pack::flo::pack_flo_def` reading
//! the file produces byte-identical cache bytes for every flo. Mirrors
//! 2004scape's `tools/unpack/config/FloConfig.ts` text shape extended with
//! the additional rev-647 opcodes (5/7/8/9/10/11/12/13/14/16) — see the
//! key table in `pack::flo`.
//!
//! Section header is the flo.pack registry name when one is present, with
//! a `flo_<id>` fallback for the (common) unnamed slots. Flo has no op 250
//! in its decoder, so the header contributes zero cache bytes; it exists
//! purely as a content-addressing handle for downstream tools and the
//! human eye. Collision-renamed sections use `<name>__<id>`.
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

use crate::cache647::{CacheType, FloType};
use crate::pack::flo::emit_flo_props;
use crate::pack_registry::PackRegistry;

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
        let mut def = FloType::new(entry.id as u16);
        let mut buf = Packet::from(entry.data.to_vec());
        def.decode(&mut buf);

        // Section header: registry name → `flo_<id>` fallback. No op 250
        // exists in the flo decoder, so this string contributes zero
        // cache bytes; it's purely a human-readable handle.
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
