//! Pack NPC spawns from `n<mx>_<mz>.jm2` text squares → server map.dat/map.idx,
//! and ground-obj spawns from `o<mx>_<mz>.jm2` → objspawn.dat/objspawn.idx.
//!
//! Squares are gathered across [`map_format::source_dirs`], so a spawn square
//! may live with the cache's own (`maps/_unpack`) or in `maps/spawns`.

use std::path::{Path, PathBuf};

use anyhow::Result;
use tracing::info;

use crate::map_format;
use crate::packed_data::PackedData;

pub fn pack(map_dirs: &[PathBuf], server_dir: &Path) -> Result<()> {
    let entries = collect(map_dirs, 'n', map_format::is_npc_spawns, |text| {
        let spawns = map_format::parse_npc_spawn_text(text);
        (!spawns.is_empty()).then(|| map_format::encode_npc_spawn_binary(&spawns))
    })?;
    save(&entries, server_dir, "map")?;
    info!("  {} NPC spawn regions packed", entries.len());
    Ok(())
}

/// Pack global ground-item spawns. Same on-disk container shape as the NPC
/// packer; the engine merges these as the `o` mapsquare layer and loads them
/// as `EntityLifeTime::Respawn` objs.
pub fn pack_objs(map_dirs: &[PathBuf], server_dir: &Path) -> Result<()> {
    let entries = collect(map_dirs, 'o', map_format::is_obj_spawns, |text| {
        let spawns = map_format::parse_obj_spawn_text(text);
        (!spawns.is_empty()).then(|| map_format::encode_obj_spawn_binary(&spawns))
    })?;
    save(&entries, server_dir, "objspawn")?;
    info!("  {} obj spawn regions packed", entries.len());
    Ok(())
}

type Entry = (u8, u8, Vec<u8>);

fn collect(
    map_dirs: &[PathBuf],
    prefix: char,
    is_kind: fn(&str) -> bool,
    encode: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for (stem, path) in map_format::squares(map_dirs) {
        if !is_kind(&stem) {
            continue;
        }
        let stripped = stem.strip_prefix(prefix).unwrap_or(&stem);
        let Some((mx, mz)) = stripped.split_once('_') else {
            continue;
        };
        let (Ok(mx), Ok(mz)) = (mx.parse::<u8>(), mz.parse::<u8>()) else {
            continue;
        };
        if let Some(binary) = encode(&std::fs::read_to_string(&path)?) {
            entries.push((mx, mz, binary));
        }
    }
    entries.sort_by_key(|(mx, mz, _)| (*mx as u16) << 8 | *mz as u16);
    Ok(entries)
}

fn save(entries: &[Entry], server_dir: &Path, stem: &str) -> Result<()> {
    let mut packed = PackedData::new(entries.len() as u32);
    for (mx, mz, binary) in entries {
        packed.start_entry();
        packed.p1(*mx);
        packed.p1(*mz);
        packed.dat.extend_from_slice(binary);
        packed.finish_entry_raw();
    }
    packed.save(server_dir, stem)
}
