//! Unpack VarBit configs from JS5 cache to text config format.
//!
//! Produces `_unpack/all.varbit` such that
//! `pack::varbit::pack_varbit_def` reading the file produces byte-identical
//! cache bytes for every varbit. Mirrors 2004scape's
//! `tools/unpack/config/VarbitConfig.ts` text shape: each `Bits` emission
//! becomes three lines — `basevar=<varp_name>`, `startbit=<lsb>`,
//! `endbit=<msb>`.
//!
//! Archive 22 is strided (stride 1024): every group holds up to 1024
//! entries, and the global varbit id is `group_id * 1024 + entry.id`.
//!
//! Section header is the registry's varbit name; missing entries fall back
//! to `varbit_<global_id>`. Collisions append `__<id>`.
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

use crate::cache647::{CacheType, VarBitType};
use crate::pack::varbit::emit_varbit_props;
use crate::pack_registry::PackRegistry;

const ARCHIVE_VARBITS: u8 = 22;
const STRIDE_VARBITS: u32 = 1024;

pub fn unpack_varbits(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_VARBITS).unwrap_or(0);

    let out_path = output_dir.join("all.varbit");
    let mut file = std::fs::File::create(&out_path)?;

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for group_id in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_VARBITS, group_id)? else {
            continue;
        };

        // Sort entries by id within group for deterministic output.
        let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
        sorted.sort_by_key(|e| e.id);

        for entry in sorted {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = group_id * STRIDE_VARBITS + entry.id;
            let mut def = VarBitType::new(global_id as u16);
            def.decode(&mut Packet::from(entry.data.to_vec()));

            let base = registry
                .varbit
                .get_by_id(global_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("varbit_{}", global_id));
            let section = if seen.contains_key(&base) {
                format!("{base}__{}", global_id)
            } else {
                base
            };
            seen.insert(section.clone(), global_id);

            writeln!(file, "[{section}]")?;
            for (k, v) in emit_varbit_props(&def, registry) {
                writeln!(file, "{k}={v}")?;
            }
            writeln!(file)?;
            written += 1;
        }
    }

    info!("  Wrote {written} varbit configs to {}", out_path.display());
    Ok(())
}
