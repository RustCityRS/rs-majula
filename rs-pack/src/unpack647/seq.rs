//! Unpack Seq (animation sequence) configs from JS5 cache to text config
//! format.
//!
//! Produces `_unpack/all.seq` such that `pack::seq::pack_seq_def` reading the
//! file produces byte-identical cache bytes for every cache-resident seq.
//!
//! Seq lives in archive 20, **strided u16** ids (stride 128): the cache
//! group id is `global_id >> 7` and the within-group entry id is
//! `global_id & 0x7F`. We iterate every group (`store.group_count(20)`) and
//! every entry within, yielding all populated seq ids.
//!
//! Phase 1 contract: this writes EVERY cache entry, every emission, in
//! source order — no overlay-key gating, no skipping of entries with empty
//! emission lists.
//!
//! Section header: registry-mapped name from `seq.pack` if present, else
//! `seq_<global_id>` fallback. Seq has no DebugName (op 250) opcode in the
//! cache format — the section name is purely for human/registry use and is
//! NOT round-tripped into cache bytes.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache647::{CacheType, SeqType};
use crate::pack::seq::emit_seq_props;
use crate::pack_registry::PackRegistry;

const ARCHIVE_SEQS: u8 = 20;
const STRIDE_SEQS: u32 = 128;

pub fn unpack_seqs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_SEQS).unwrap_or(0);
    if group_count == 0 {
        info!("  Archive {ARCHIVE_SEQS} (seq) absent — skipping");
        return Ok(());
    }

    let out_path = output_dir.join("all.seq");
    let mut file = std::fs::File::create(&out_path)?;

    // Collect every populated entry with its global id so we can emit sorted
    // for deterministic output.
    let mut entries: Vec<(u32, rs_dat2::Entry)> = Vec::new();
    for group_id in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_SEQS, group_id)? else {
            continue;
        };
        for entry in group.entries {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = group_id * STRIDE_SEQS + entry.id;
            entries.push((global_id, entry));
        }
    }
    entries.sort_by_key(|(gid, _)| *gid);

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for (global_id, entry) in &entries {
        let mut def = SeqType::new(*global_id as u16);
        def.decode(&mut Packet::from(entry.data.to_vec()));

        let base = registry
            .seq
            .get_by_id(*global_id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("seq_{global_id}"));
        let section = if seen.contains_key(&base) {
            format!("{base}__{global_id}")
        } else {
            base
        };
        seen.insert(section.clone(), *global_id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_seq_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} seq configs to {}", out_path.display());
    Ok(())
}
