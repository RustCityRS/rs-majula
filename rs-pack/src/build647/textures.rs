//! Archive 9 — Textures: .texture text → binary.

use std::path::Path;

use anyhow::Result;
use rayon::prelude::*;
use tracing::info;

use rs_dat2::archive::{self, ArchiveVersions};
use rs_dat2::reference::FileInfo;

use super::helpers::{PreparedGroup, group_version, stamp_path, touch_stamp, write_archive};

use crate::startup::{is_up_to_date, newest_mtime};

pub fn build(
    store: &mut rs_dat2::CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    client_data_dir: &Path,
    versions: &std::collections::HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    let aid = archive::TEXTURES;
    let format = archive::archive_format(aid).unwrap();
    let unpack_dir = content_dir.join(format.content_dir).join("_unpack");

    let textures_stamp = stamp_path(output_dir, aid);
    let textures_input = newest_mtime(&unpack_dir, &["texture"]);
    if !force_rebuild && is_up_to_date(&textures_stamp, textures_input) {
        info!(
            "  archive {:>2} ({}) -- up to date, skipping",
            aid, format.content_dir
        );
        return Ok(());
    }

    if !unpack_dir.is_dir() {
        info!(
            "  archive {:>2} ({}) -- no _unpack dir, skipping",
            aid, format.content_dir
        );
        return Ok(());
    }

    let mut group_files: Vec<(u32, std::path::PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&unpack_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("texture") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if let Ok(gid) = stem.parse::<u32>() {
                group_files.push((gid, path));
            }
        }
    }
    group_files.sort_by_key(|(gid, _)| *gid);

    let encoded: Vec<(u32, Vec<u8>)> = group_files
        .par_iter()
        .filter_map(|(gid, path)| {
            let text = std::fs::read_to_string(path).ok()?;
            let def = crate::texture_format::parse_texture_text(&text);
            Some((*gid, crate::texture_format::encode_texture(&def)))
        })
        .collect();

    let mut blob = crate::config_blob::ConfigBlob::new();
    for (gid, data) in encoded {
        blob.add(&gid.to_string(), data);
    }
    blob.save(&client_data_dir.join("textures"))?;

    let gids: Vec<u32> = blob.keys().filter_map(|k| k.parse::<u32>().ok()).collect();

    let prepared: Vec<PreparedGroup> = gids
        .par_iter()
        .map(|gid| {
            let data = blob.get(&gid.to_string()).unwrap();
            let ver = group_version(versions, aid, gid);
            let (raw, crc) =
                rs_dat2::prepare_group(aid, *gid, data, ver as u16, format.allow_uncompressed)
                    .unwrap();
            PreparedGroup {
                id: *gid,
                raw,
                crc,
                version: ver,
                size: data.len(),
                name_hash: -1,
                whirlpool: None,
                files: vec![FileInfo {
                    id: 0,
                    name_hash: None,
                }],
            }
        })
        .collect();

    write_archive(
        store,
        aid,
        format,
        versions,
        prepared,
        "from .texture encode",
    )?;
    touch_stamp(&textures_stamp)?;
    Ok(())
}
