//! Unpack SpotAnim configs from JS5 cache to text config format.
//!
//! Archive 21, **strided u16** ids (stride 256) — global id is
//! `gid * 256 + entry.id`. Walks the cache directly via
//! `store.group_count(21)` × `store.read_group(21, gid)` so every entry
//! survives, irrespective of how `rs_configs::loader` summarises them.
//!
//! Produces `_unpack/all.spotanim` such that
//! `pack::spotanim::pack_spotanim_def` reading the file produces
//! byte-identical cache bytes. Section header is the registry name
//! (`spotanim.pack`), falling back to `spotanim_<global_id>` for
//! unregistered ids.
//!
//! SpotAnim has no op 250 / DebugName — the section header is purely a
//! content identifier and is **not** emitted into cache bytes. Collision
//! disambiguation (rare; only happens if the registry has duplicate names)
//! suffixes the global id.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache647::{CacheType, SpotAnimType};
use crate::pack::spotanim::emit_spotanim_props;
use crate::pack_registry::PackRegistry;

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
            let mut def = SpotAnimType::new(global_id as u16);
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
