//! Archives 8/32 — Sprites + Loading Sprites: PNG → Jagex sprite binary.
//!
//! Archive 8 (SPRITES) writes a "media" client blob.
//! Archive 32 (LOADING_SPRITES) writes direct-to-cache.

use std::path::Path;

use anyhow::Result;
use rayon::prelude::*;
use tracing::info;

use rs_dat2::archive::{self, ArchiveVersions};
use rs_dat2::reference::FileInfo;

use super::helpers::{
    PreparedGroup, group_version, load_name_hashes, stamp_path, touch_stamp, write_archive,
};
use super::sprites_encode::encode_sprite_group_from_dir;

use crate::startup::{is_up_to_date, newest_mtime};

pub fn build(
    store: &mut rs_dat2::CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    pack_dir: &Path,
    client_data_dir: &Path,
    versions: &std::collections::HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    for &(aid, dir_name) in &[
        (archive::SPRITES, "sprites"),
        (archive::LOADING_SPRITES, "loading_sprites"),
    ] {
        let format = archive::archive_format(aid).unwrap();
        let unpack_dir = content_dir.join(dir_name).join("_unpack");

        if !unpack_dir.is_dir() {
            info!(
                "  archive {:>2} ({}) -- no _unpack dir, skipping",
                aid, dir_name
            );
            continue;
        }

        let sprite_stamp = stamp_path(output_dir, aid);
        let sprite_input = newest_mtime(&unpack_dir, &[]);
        if !force_rebuild && is_up_to_date(&sprite_stamp, sprite_input) {
            info!(
                "  archive {:>2} ({}) -- up to date, skipping",
                aid, dir_name
            );
            continue;
        }

        let mut group_ids: Vec<u32> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&unpack_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if let Ok(gid) = name.parse::<u32>() {
                            group_ids.push(gid);
                        }
                    }
                }
            }
        }
        group_ids.sort();

        if group_ids.is_empty() {
            info!(
                "  archive {:>2} ({}) -- no sprite groups found, skipping",
                aid, dir_name
            );
            continue;
        }

        let name_hashes: std::collections::HashMap<u32, i32> = if format.named {
            load_name_hashes(pack_dir, aid, &group_ids)
        } else {
            std::collections::HashMap::new()
        };

        let encoded: Vec<(u32, Vec<u8>)> = group_ids
            .par_iter()
            .filter_map(|&gid| {
                let group_dir = unpack_dir.join(gid.to_string());
                match encode_sprite_group_from_dir(&group_dir) {
                    Ok(Some(b)) => Some((gid, b)),
                    Ok(None) => None,
                    Err(e) => {
                        tracing::warn!("sprite group {}: encode failed ({}), skipping", gid, e);
                        None
                    }
                }
            })
            .collect();

        if aid == archive::SPRITES {
            let mut blob = crate::config_blob::ConfigBlob::new();
            for (gid, data) in &encoded {
                blob.add(&gid.to_string(), data.clone());
            }
            blob.save(&client_data_dir.join("media"))?;
        }

        let prepared: Vec<PreparedGroup> = encoded
            .par_iter()
            .filter_map(|(gid, data)| {
                let ver = group_version(versions, aid, gid);
                let (raw, crc) =
                    rs_dat2::prepare_group(aid, *gid, data, ver as u16, format.allow_uncompressed)
                        .ok()?;
                let name_hash = if format.named {
                    name_hashes.get(gid).copied().unwrap_or(-1)
                } else {
                    -1
                };
                Some(PreparedGroup {
                    id: *gid,
                    raw,
                    crc,
                    version: ver,
                    size: data.len(),
                    name_hash,
                    whirlpool: None,
                    files: vec![FileInfo {
                        id: 0,
                        name_hash: None,
                    }],
                })
            })
            .collect();

        let blob_note = if aid == archive::SPRITES {
            "PNG encode → media blob"
        } else {
            "PNG encode"
        };
        write_archive(store, aid, format, versions, prepared, blob_note)?;
        touch_stamp(&sprite_stamp)?;
    }
    Ok(())
}
