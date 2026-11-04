//! Unpack Loc (location/scenery) configs from JS5 cache to text config.
//!
//! Produces `_unpack/all.loc` such that
//! `pack::loc::pack_loc_def(...).encode()` reproduces the cache bytes
//! byte-for-byte. Mirrors the `inv` gold-template shape: every cache entry
//! becomes a `[section]` block, every emission becomes one or more text
//! lines via [`emit_loc_props`], no overlay-key gating.
//!
//! Loc lives in archive 16 with strided-u16 ids (stride 256), giving
//! ~52,230 entries.
//!
//! Section header is the cache's loc name from `loc.pack`, falling back to
//! `loc_<global_id>` if the registry doesn't have an entry. Collision-
//! renamed sections use `<name>__<id>`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache647::{CacheType, LocType};
use crate::pack::loc::emit_loc_props;
use crate::pack_registry::PackRegistry;

const ARCHIVE_LOCS: u8 = 16;
const LOC_STRIDE: u32 = 256;

pub fn unpack_locs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_LOCS).unwrap_or(0);

    let out_path = output_dir.join("all.loc");
    let mut file = std::fs::File::create(&out_path)?;

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for group_id in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_LOCS, group_id)? else {
            continue;
        };

        // Sort entries by id within group for deterministic output.
        let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
        sorted.sort_by_key(|e| e.id);

        for entry in sorted {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = group_id * LOC_STRIDE + entry.id;
            let mut def = LocType::new(global_id as u16);
            def.decode(&mut Packet::from(entry.data.to_vec()));

            let base = registry
                .loc
                .get_by_id(global_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("loc_{}", global_id));
            let section = if seen.contains_key(&base) {
                format!("{base}__{}", global_id)
            } else {
                base
            };
            seen.insert(section.clone(), global_id);

            super::common::write_config_section(
                &mut file,
                &section,
                emit_loc_props(&def, registry),
            )?;
            written += 1;
        }
    }

    info!("  Wrote {written} loc configs to {}", out_path.display());
    Ok(())
}

/// Server-only keys for LOC configs. These fields exist only in the server
/// stream and are NOT present in the original Jagex cache, so a decode of the
/// cache cannot reproduce them. [`update_specific_loc_file`] keeps them
/// verbatim when it refreshes an authored file's cache fields.
///
/// Note: `op1`-`op5` exist in the client cache as opcodes 30-34 and are
/// emitted by `emit_loc_props`, so they are NOT server-only.
const LOC_SERVER_ONLY_KEYS: &[&str] = &["param", "paramint", "paramstr", "category"];

/// Update a specific `.loc` file in-place: regenerate client fields from the
/// original cache while preserving server-only additions.
pub fn update_specific_loc_file(
    cache_dir: &Path,
    registry: &PackRegistry,
    file_path: &Path,
) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    super::common::update_specific_file::<LocType, _, _>(
        &mut store,
        file_path,
        ARCHIVE_LOCS,
        LOC_STRIDE,
        LOC_SERVER_ONLY_KEYS,
        |name| registry.loc.get_by_name(name),
        |def| emit_loc_props(def, registry),
    )
}
