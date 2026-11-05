//! Archive 5 — Maps: text → binary → compress (+ XTEA encrypt for landscapes).

use crate::pack::pack_registry::PackFile;
use std::path::Path;

use anyhow::Result;
use rayon::prelude::*;
use tracing::info;

use rs_dat2::archive::{self, ArchiveVersions};
use rs_dat2::reference::FileInfo;

use super::helpers::{PreparedGroup, group_version, stamp_path, touch_stamp, write_archive};

use crate::startup::{is_up_to_date, max_of, newest_mtime};

pub fn build(
    store: &mut rs_dat2::CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    pack_dir: &Path,
    versions: &std::collections::HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    let aid = archive::MAPS;
    let format = archive::archive_format(aid).unwrap();
    // Opaque squares (no key exists) are cache output, so only `_unpack`
    // holds them; text squares may come from any source directory.
    let map_dir = content_dir.join("maps").join("_unpack");
    let map_dirs = crate::map_format::source_dirs(content_dir);

    let maps_stamp = stamp_path(output_dir, aid);
    let mut maps_inputs: Vec<_> = map_dirs.iter().map(|dir| newest_mtime(dir, &[])).collect();
    maps_inputs.push(crate::startup::file_mtime(&pack_dir.join("map.pack")));
    let maps_input = max_of(&maps_inputs);
    if !force_rebuild && is_up_to_date(&maps_stamp, maps_input) {
        info!(
            "  archive {:>2} ({}) -- up to date, skipping",
            aid, format.content_dir
        );
        return Ok(());
    }

    use crate::map_format;

    let map_pack = PackFile::load(&pack_dir.join("map.pack")).unwrap_or_else(|_| PackFile::empty());

    let keys_path = output_dir.join("keys.json");
    let keys_candidates = [keys_path.clone(), content_dir.join("keys.json")];
    let xtea_keys = if let Some(path) = keys_candidates.iter().find(|p| p.exists()) {
        map_format::load_xtea_keys(path).unwrap_or_default()
    } else {
        anyhow::bail!("No keys.json found in output dir or content dir");
    };

    let mut groups: Vec<(u32, String)> = map_pack
        .id_to_debugname
        .iter()
        .map(|(&id, name)| (id, name.clone()))
        .collect();
    groups.sort_by_key(|(id, _)| *id);

    let prepared: Vec<PreparedGroup> = groups
        .par_iter()
        .filter_map(|(gid, name)| {
            let ver = group_version(versions, aid, gid);
            let version = ver as u16;
            let name_hash = rs_dat2::xtea::java_string_hash(name);

            let is_landscape = map_format::is_landscape(name);
            let is_npc = map_format::is_npc_spawns(name);

            let (raw, crc) = if is_landscape {
                let text_path = map_format::find_square(&map_dirs, name);
                let encrypted_path = map_dir.join(format!("encrypted/{}.bin", gid));

                if let Some(text_path) = text_path {
                    let text = std::fs::read_to_string(&text_path).ok()?;
                    let locs = map_format::parse_landscape_text(&text);
                    let binary = map_format::encode_landscape_binary(&locs);
                    let key = xtea_keys.get(&name_hash).copied().unwrap_or([0; 4]);
                    let container = map_format::compress_and_encrypt(
                        &binary,
                        rs_dat2::compression::CompressionType::Gzip,
                        version,
                        &key,
                    )
                    .ok()?;
                    let crc = rs_dat2::compute_group_crc(&container[..container.len() - 2]);
                    (container, crc)
                } else if encrypted_path.exists() {
                    let raw = std::fs::read(&encrypted_path).ok()?;
                    let crc = map_format::container_crc(&raw, raw[0]);
                    (raw, crc)
                } else {
                    tracing::warn!(
                        "  archive 5 group {}: missing file for landscape {}",
                        gid,
                        name
                    );
                    return None;
                }
            } else if is_npc {
                let text_path = map_format::find_square(&map_dirs, name)?;
                let text = std::fs::read_to_string(&text_path).ok()?;
                let spawns = map_format::parse_npc_spawn_text(&text);
                let binary = map_format::encode_npc_spawn_binary(&spawns);
                rs_dat2::prepare_group(aid, *gid, &binary, version, false).ok()?
            } else {
                let text_path = map_format::find_square(&map_dirs, name)?;
                let text = std::fs::read_to_string(&text_path).ok()?;
                let terrain = map_format::parse_terrain_text(&text);
                let binary = map_format::encode_terrain_binary(&terrain);
                rs_dat2::prepare_group(aid, *gid, &binary, version, false).ok()?
            };

            Some(PreparedGroup {
                id: *gid,
                raw,
                crc,
                version: ver,
                size: 0,
                name_hash,
                whirlpool: None,
                files: vec![FileInfo {
                    id: 0,
                    name_hash: None,
                }],
            })
        })
        .collect();

    write_archive(
        store,
        aid,
        format,
        versions,
        prepared,
        "from text→binary encode",
    )?;
    touch_stamp(&maps_stamp)?;
    Ok(())
}
