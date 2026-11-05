//! Unpack NPC configs from JS5 cache to text config format.
//!
//! Produces `_unpack/all.npc` such that
//! `pack::npc::pack_npc_def(id, section, props, registry)` reading the
//! file produces byte-identical cache bytes for every npc. Mirrors
//! 2004scape's `tools/unpack/config/NpcConfig.ts` text shape, lifted
//! to rev-647's expanded opcode set: `name=`, `op1..op5=`, `model1=`,
//! `head1=`, `recol1s/recol1d=`, `multivar=`/`multivarp=`/`multi1..N`,
//! `cursor1=op,id`, `tint=h,s,l,o`, etc.
//!
//! Archive 18 is strided (stride 128): every group holds up to 128
//! entries, and the global npc id is `group_id * 128 + entry.id`.
//!
//! Section header is the registry's npc name; missing entries fall
//! back to `npc_<global_id>`. Collisions append `__<id>`.
//!
//! Phase 1 contract: this writes EVERY cache entry, every emission,
//! in source order — no overlay-key gating.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::npc::NpcTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::npc::emit_npc_props;
use crate::pack_registry::PackRegistry;

const ARCHIVE_NPCS: u8 = 18;
const STRIDE_NPCS: u32 = 128;

pub fn unpack_npcs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_NPCS).unwrap_or(0);

    let out_path = output_dir.join("all.npc");
    let mut file = std::fs::File::create(&out_path)?;

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for group_id in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_NPCS, group_id)? else {
            continue;
        };

        let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
        sorted.sort_by_key(|e| e.id);

        for entry in sorted {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = group_id * STRIDE_NPCS + entry.id;
            let mut def = NpcTypeRaw::new(global_id as u16);
            def.decode(&mut Packet::from(entry.data.to_vec()));

            let base = registry
                .npc
                .get_by_id(global_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("npc_{}", global_id));
            let section = if seen.contains_key(&base) {
                format!("{base}__{}", global_id)
            } else {
                base
            };
            seen.insert(section.clone(), global_id);

            writeln!(file, "[{section}]")?;
            for (k, v) in emit_npc_props(&def, registry) {
                writeln!(file, "{k}={v}")?;
            }
            writeln!(file)?;
            written += 1;
        }
    }

    info!("  Wrote {written} npc configs to {}", out_path.display());
    Ok(())
}
