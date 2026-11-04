//! Unpack Obj (item) configs from JS5 cache to text config format.
//!
//! Produces `_unpack/all.obj` such that `pack::obj::pack_obj_def` reading
//! the file produces byte-identical cache bytes for every item. Mirrors
//! 2004scape's `tools/unpack/config/ObjConfig.ts` text shape: 2D display
//! tags use `2dzoom`/`2dxan` etc., `op<N>=`/`iop<N>=` are 1-indexed,
//! `recol<N>s=`/`recol<N>d=` are interleaved, `count<N>=name,count` is
//! 1-indexed.
//!
//! Section header is the obj name from `obj.pack` (id->name); IDs without a
//! pack entry fall back to `obj_<id>`. Collision-renamed sections use
//! `<name>__<id>` (some pack files share names across multiple ids).
//!
//! Phase 1 contract: this writes EVERY cache entry, every emission, in
//! source order — no overlay-key gating.
//!
//! Archive 19 is strided: `item_id = group_id * 256 + entry_id`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache647::{CacheType, ObjType};
use crate::pack::obj::emit_obj_props;
use crate::pack_registry::PackRegistry;

const ARCHIVE_ITEMS: u8 = 19;
const STRIDE: u32 = 256;

pub fn unpack_objs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;

    let out_path = output_dir.join("all.obj");
    let mut file = std::fs::File::create(&out_path)?;

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    let group_count = store.group_count(ARCHIVE_ITEMS).unwrap_or(0);
    for group_id in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_ITEMS, group_id)? else {
            continue;
        };

        // Sort entries by id within the group for deterministic output.
        let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
        sorted.sort_by_key(|e| e.id);

        for entry in sorted {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = group_id * STRIDE + entry.id;

            let mut def = ObjType::new(global_id as u16);
            def.decode(&mut Packet::from(entry.data.to_vec()));

            let base = registry
                .obj
                .get_by_id(global_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("obj_{global_id}"));
            let section = if seen.contains_key(&base) {
                format!("{base}__{global_id}")
            } else {
                base
            };
            seen.insert(section.clone(), global_id);

            super::common::write_config_section(
                &mut file,
                &section,
                emit_obj_props(&def, registry),
            )?;
            written += 1;
        }
    }

    info!("  Wrote {written} obj configs to {}", out_path.display());
    Ok(())
}

/// Server-only keys for OBJ configs: fields that exist only in the server
/// stream and have no opcode in the Jagex cache, so a decode of the cache
/// cannot reproduce them. [`update_specific_obj_file`] keeps them verbatim
/// when it refreshes an authored file's cache fields.
///
/// The set is taken from the server-stream branch of
/// [`crate::pack::obj::pack_objs`] -- every key it turns into a server opcode
/// and that [`emit_obj_props`] does not emit back.
const OBJ_SERVER_ONLY_KEYS: &[&str] = &[
    "param",
    "paramint",
    "paramstr",
    "desc",
    "wearpos",
    "wearpos2",
    "wearpos3",
    "weight",
    "category",
    "respawnrate",
    "dummyitem",
    "tradeable",
];

/// Update a specific `.obj` file in-place: regenerate client fields from the
/// original cache while preserving server-only additions (`param=...`).
pub fn update_specific_obj_file(
    cache_dir: &Path,
    registry: &PackRegistry,
    file_path: &Path,
) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    super::common::update_specific_file::<ObjType, _, _>(
        &mut store,
        file_path,
        ARCHIVE_ITEMS,
        STRIDE,
        OBJ_SERVER_ONLY_KEYS,
        |name| registry.obj.get_by_name(name),
        |def| emit_obj_props(def, registry),
    )
}
