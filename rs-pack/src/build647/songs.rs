//! Archive 6 — Songs: standard MIDI → Jagex columnar MIDI.

use std::path::Path;

use anyhow::Result;
use rayon::prelude::*;
use tracing::info;

use rs_dat2::archive::{self, ArchiveVersions};
use rs_dat2::reference::FileInfo;

use super::helpers::{
    PreparedGroup, group_version, load_name_hashes, stamp_path, touch_stamp, write_archive,
};

use crate::startup::{is_up_to_date, newest_mtime};

pub fn build(
    store: &mut rs_dat2::CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    pack_dir: &Path,
    versions: &std::collections::HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    let aid = archive::SONGS;
    let format = archive::archive_format(aid).unwrap();
    let unpack_dir = content_dir.join(format.content_dir).join("_unpack");

    let songs_stamp = stamp_path(output_dir, aid);
    let songs_input = newest_mtime(&unpack_dir, &["mid"]);
    if !force_rebuild && is_up_to_date(&songs_stamp, songs_input) {
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
            if path.extension().and_then(|s| s.to_str()) != Some("mid") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if let Ok(gid) = stem.parse::<u32>() {
                group_files.push((gid, path));
            }
        }
    }
    group_files.sort_by_key(|(gid, _)| *gid);

    let name_hashes: std::collections::HashMap<u32, i32> = if format.named {
        let gids: Vec<u32> = group_files.iter().map(|(gid, _)| *gid).collect();
        load_name_hashes(pack_dir, aid, &gids)
    } else {
        std::collections::HashMap::new()
    };

    let encoded: Vec<(u32, Vec<u8>)> = group_files
        .par_iter()
        .filter_map(|(gid, path)| {
            let midi_bytes = std::fs::read(path).ok()?;
            match crate::repack::midi::encode_jagex_midi(&midi_bytes) {
                Ok(b) => Some((*gid, b)),
                Err(e) => {
                    tracing::warn!("song group {}: encode failed ({}), skipping", gid, e);
                    None
                }
            }
        })
        .collect();

    let prepared: Vec<PreparedGroup> = encoded
        .par_iter()
        .map(|(gid, data)| {
            let ver = group_version(versions, aid, gid);
            let (raw, crc) =
                rs_dat2::prepare_group(aid, *gid, data, ver as u16, format.allow_uncompressed)
                    .unwrap();
            let name_hash = if format.named {
                name_hashes.get(gid).copied().unwrap_or(-1)
            } else {
                -1
            };
            PreparedGroup {
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
            }
        })
        .collect();

    write_archive(store, aid, format, versions, prepared, "from .mid encode")?;
    touch_stamp(&songs_stamp)?;
    Ok(())
}
