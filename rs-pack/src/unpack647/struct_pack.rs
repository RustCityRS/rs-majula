//! Unpack Struct configs from JS5 cache to text config format.
//!
//! Produces `_unpack/all.struct` such that
//! `pack::struct_pack::pack_struct_def` reading the file produces
//! byte-identical cache bytes for every struct. Mirrors the .struct text
//! shape: a list of `paramint=<param_name>,<i32>` and
//! `paramstr=<param_name>,<string>` lines under a `[name]` header.
//!
//! Section header is the content-side name from `struct.pack`. Struct
//! has NO op 250 / debugname in the cache, so the `[name]` value is a
//! pure content identifier; missing entries fall back to `struct_<id>`,
//! collision-renamed sections use `<name>__<id>`.
//!
//! Phase 1 contract: this writes EVERY cache entry, every emission, in
//! source order — no overlay-key gating.
//!
//! NOTE: file name is `struct_pack.rs` (not `struct.rs`) because
//! `struct` is a Rust keyword. The corresponding registry field is
//! accessed as `registry.r#struct`.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::provider::CacheType;
use crate::cache::r#struct::StructTypeRaw;
use crate::pack::struct_pack::emit_struct_props;
use crate::pack_registry::PackRegistry;

const ARCHIVE_CONFIG: u8 = 2;
const GROUP_STRUCT: u32 = 26;

pub fn unpack_structs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let Some(group) = store.read_group(ARCHIVE_CONFIG, GROUP_STRUCT)? else {
        info!("  Archive 2 group {GROUP_STRUCT} (struct) absent — skipping");
        return Ok(());
    };

    let out_path = output_dir.join("all.struct");
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
        let mut def = StructTypeRaw::new(entry.id as u16);
        def.decode(&mut Packet::from(entry.data.to_vec()));

        let base = registry
            .r#struct
            .get_by_id(entry.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("struct_{}", entry.id));
        let section = if seen.contains_key(&base) {
            format!("{base}__{}", entry.id)
        } else {
            base
        };
        seen.insert(section.clone(), entry.id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_struct_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} struct configs to {}", out_path.display());
    Ok(())
}
