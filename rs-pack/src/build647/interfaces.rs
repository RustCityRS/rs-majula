#![allow(clippy::redundant_closure, clippy::unnecessary_map_or)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Archive 3 — Interfaces: text .if → client blob → JS5 multi-file groups.

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
    client_data_dir: &Path,
    versions: &std::collections::HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    let aid = archive::INTERFACES;
    let format = archive::archive_format(aid).unwrap();
    let if_dir = content_dir.join("interfaces").join("_unpack");

    let iface_stamp = stamp_path(output_dir, aid);
    let iface_input = max_of(&[
        newest_mtime(&if_dir, &["if"]),
        crate::startup::file_mtime(&pack_dir.join("interface.pack")),
    ]);
    if !force_rebuild && is_up_to_date(&iface_stamp, iface_input) {
        info!("  archive {:>2} (interfaces) -- up to date, skipping", aid);
        return Ok(());
    }

    let interface_blob_path = client_data_dir.join("interface");

    let iface_pack = crate::pack_registry::PackFile::load(&pack_dir.join("interface.pack"))
        .unwrap_or_else(|_| crate::pack_registry::PackFile::empty());

    let iface_hashes_fallback: std::collections::HashMap<u32, i32> = {
        let hashes_path = pack_dir.join("interface.hashes");
        let mut m = std::collections::HashMap::new();
        if let Ok(text) = std::fs::read_to_string(&hashes_path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let Some((id_str, hash_str)) = line.split_once('=') else {
                    continue;
                };
                if let (Ok(gid), Ok(hash)) =
                    (id_str.trim().parse::<u32>(), hash_str.trim().parse::<i32>())
                {
                    m.insert(gid, hash);
                }
            }
        }
        m
    };

    let comp_name_maps: std::collections::HashMap<u32, std::collections::HashMap<String, u32>> = {
        let mut maps: std::collections::HashMap<u32, std::collections::HashMap<String, u32>> =
            std::collections::HashMap::new();
        let pack_path = pack_dir.join("interface.pack");
        if let Ok(text) = std::fs::read_to_string(&pack_path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Some((id_str, name)) = line.split_once('=') else {
                    continue;
                };
                let Some((gid_str, cid_str)) = id_str.split_once(':') else {
                    continue;
                };
                let Ok(gid) = gid_str.parse::<u32>() else {
                    continue;
                };
                let Ok(cid) = cid_str.parse::<u32>() else {
                    continue;
                };
                maps.entry(gid)
                    .or_default()
                    .insert(name.trim().to_string(), cid);
            }
        }
        maps
    };

    let mut if_files: Vec<(u32, std::path::PathBuf)> = Vec::new();
    if if_dir.is_dir() {
        for entry in std::fs::read_dir(&if_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("if") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if let Ok(gid) = stem.parse::<u32>() {
                if_files.push((gid, path));
            } else if let Some(gid) = iface_pack.get_by_name(stem) {
                if_files.push((gid, path));
            }
        }
    }
    if_files.sort_by_key(|(gid, _)| *gid);

    let encoded_groups: Vec<(u32, Vec<u8>)> = if_files
        .par_iter()
        .map(|(gid, path)| {
            let text = std::fs::read_to_string(path).unwrap();
            let names = comp_name_maps.get(gid);
            let components = crate::interface_format::parse_interface_text(&text, names);
            let mut file_entries: Vec<(u32, Vec<u8>)> = components
                .iter()
                .map(|(id, comp)| (*id, crate::interface_format::encode_component(comp, *gid)))
                .collect();
            file_entries.sort_by_key(|(id, _)| *id);
            let refs: Vec<(u32, &[u8])> = file_entries
                .iter()
                .map(|(id, data)| (*id, data.as_slice()))
                .collect();
            (*gid, rs_dat2::encode_group_entries(&refs))
        })
        .collect();

    let mut blob = crate::config_blob::ConfigBlob::new();
    for (gid, group_data) in encoded_groups {
        blob.add(&gid.to_string(), group_data);
    }
    blob.save(&interface_blob_path)?;

    let iface_versions = versions.get(&aid);

    let mut blob_groups: Vec<(u32, &[u8])> = blob
        .keys()
        .filter_map(|k| k.parse::<u32>().ok().map(|gid| (gid, blob.get(k).unwrap())))
        .collect();
    blob_groups.sort_by_key(|(gid, _)| *gid);

    let prepared: Vec<PreparedGroup> = blob_groups
        .par_iter()
        .map(|(gid, group_data)| {
            let ver = group_version(versions, aid, gid);

            let (raw, crc) = rs_dat2::prepare_group(
                aid,
                *gid,
                group_data,
                ver as u16,
                format.allow_uncompressed,
            )
            .unwrap();

            let name_hash = iface_pack
                .get_by_id(*gid)
                .filter(|name| {
                    let h = rs_dat2::xtea::java_string_hash(name);
                    iface_hashes_fallback
                        .get(gid)
                        .map_or(true, |&expected| h == expected)
                })
                .map(|name| rs_dat2::xtea::java_string_hash(name))
                .or_else(|| iface_hashes_fallback.get(gid).copied())
                .unwrap_or(-1);

            let files: Vec<FileInfo> = if let Some(file_entries_ver) =
                iface_versions.and_then(|v| v.group_file_ids.get(gid))
            {
                file_entries_ver
                    .iter()
                    .map(|&(id, hash)| FileInfo {
                        id,
                        name_hash: hash,
                    })
                    .collect()
            } else {
                let file_count = rs_dat2::detect_file_count(group_data);
                (0..file_count as u32)
                    .map(|i| FileInfo {
                        id: i,
                        name_hash: None,
                    })
                    .collect()
            };

            PreparedGroup {
                id: *gid,
                raw,
                crc,
                version: ver,
                size: group_data.len(),
                name_hash,
                whirlpool: None,
                files,
            }
        })
        .collect();

    write_archive(
        store,
        aid,
        format,
        versions,
        prepared,
        "from .if text files",
    )?;
    touch_stamp(&iface_stamp)?;
    Ok(())
}
