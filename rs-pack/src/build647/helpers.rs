#![allow(clippy::map_entry)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Shared helpers for the build pipeline — stamp files, group file discovery,
//! name hash loading, and the universal write-archive sequence.

use std::path::Path;

use anyhow::Result;
use tracing::info;

use rs_dat2::archive::{self, ArchiveFormat};
use rs_dat2::reference::{FileInfo, GroupInfo, ReferenceTable};

// ── Stamp files ─────────────────────────────────────────────────────────────

pub fn stamp_path(output_dir: &Path, archive_id: u8) -> std::path::PathBuf {
    output_dir.join(format!(".stamp_archive_{archive_id}"))
}

pub fn touch_stamp(path: &Path) -> Result<()> {
    std::fs::write(path, b"ok")?;
    Ok(())
}

// ── Group file discovery ────────────────────────────────────────────────────

pub fn load_group_files(unpack_dir: &Path) -> Vec<(u32, std::path::PathBuf)> {
    let mut map: std::collections::HashMap<u32, std::path::PathBuf> =
        std::collections::HashMap::new();
    if !unpack_dir.is_dir() {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(unpack_dir) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let Ok(gid) = stem.parse::<u32>() else {
            continue;
        };
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let dominated = map
            .get(&gid)
            .and_then(|p| p.extension().and_then(|e| e.to_str()))
            .is_some_and(|e| e == "bin");
        if ext == "bin" || !dominated {
            map.insert(gid, path);
        }
    }
    let mut files: Vec<_> = map.into_iter().collect();
    files.sort_by_key(|(gid, _)| *gid);
    files
}

// ── Name hash loading ───────────────────────────────────────────────────────

pub fn load_name_hashes(
    pack_dir: &Path,
    archive_id: u8,
    group_ids: &[u32],
) -> std::collections::HashMap<u32, i32> {
    let pack_name = pack_filename_for_archive(archive_id);
    let mut result = std::collections::HashMap::new();

    let hashes_path = pack_dir.join(format!("{pack_name}.hashes"));
    if hashes_path.exists() {
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
                    result.insert(gid, hash);
                }
            }
        }
    }

    let pack_path = pack_dir.join(format!("{pack_name}.pack"));
    if pack_path.exists() {
        if let Ok(pf) = crate::pack_registry::PackFile::load(&pack_path) {
            for &gid in group_ids {
                if !result.contains_key(&gid) {
                    if let Some(name) = pf.get_by_id(gid) {
                        result.insert(gid, rs_dat2::xtea::java_string_hash(name));
                    }
                }
            }
        }
    }

    result
}

fn pack_filename_for_archive(archive_id: u8) -> &'static str {
    match archive_id {
        3 => "interface",
        5 => "map",
        6 => "songs",
        8 => "sprites",
        10 => "binary",
        12 => "clientscript",
        13 => "fontmetrics",
        23 => "worldmap_data",
        30 => "dlls",
        31 => "shaders",
        32 => "loading_sprites",
        33 => "loading_backgrounds",
        _ => "unknown",
    }
}

// ── Write archive ───────────────────────────────────────────────────────────

pub struct PreparedGroup {
    pub id: u32,
    pub raw: Vec<u8>,
    pub crc: u32,
    pub version: u32,
    pub size: usize,
    pub name_hash: i32,
    pub whirlpool: Option<[u8; 64]>,
    pub files: Vec<FileInfo>,
}

pub fn write_archive(
    store: &mut rs_dat2::CacheStore,
    aid: u8,
    format: &ArchiveFormat,
    versions: &std::collections::HashMap<u8, archive::ArchiveVersions>,
    groups: Vec<PreparedGroup>,
    label: &str,
) -> Result<()> {
    let mut group_infos = Vec::with_capacity(groups.len());
    for g in &groups {
        store.write_raw_group(aid, g.id, &g.raw)?;
        group_infos.push(GroupInfo {
            id: g.id,
            name_hash: g.name_hash,
            crc: g.crc,
            whirlpool: g.whirlpool,
            version: g.version,
            size: g.size,
            file_count: g.files.len() as u32,
            files: g.files.clone(),
        });
    }

    group_infos.sort_by_key(|g| g.id);

    let table_ver = versions.get(&aid).map(|v| v.table_version).unwrap_or(0);
    let table = ReferenceTable {
        archive_id: aid,
        format: format.format,
        version: table_ver,
        named: format.named,
        whirlpooled: format.whirlpooled,
        groups: group_infos,
    };
    store.write_reference_table(aid, &table)?;
    info!(
        "  archive {:>2} ({}) -- {} groups, table_version={} {}",
        aid,
        format.content_dir,
        table.groups.len(),
        table_ver,
        label,
    );
    Ok(())
}

pub fn write_empty_archive(
    store: &mut rs_dat2::CacheStore,
    aid: u8,
    format: &ArchiveFormat,
    versions: &std::collections::HashMap<u8, archive::ArchiveVersions>,
) -> Result<()> {
    let table_ver = versions.get(&aid).map(|v| v.table_version).unwrap_or(0);
    let table = ReferenceTable {
        archive_id: aid,
        format: format.format,
        version: table_ver,
        named: format.named,
        whirlpooled: format.whirlpooled,
        groups: vec![],
    };
    store.write_reference_table(aid, &table)?;
    info!(
        "  archive {:>2} ({}) -- 0 groups (empty), table_version={}",
        aid, format.content_dir, table_ver
    );
    Ok(())
}

/// Build file info list from version data if available, else from file_count.
pub fn build_file_infos(
    versions: &std::collections::HashMap<u8, archive::ArchiveVersions>,
    aid: u8,
    gid: u32,
    file_count: u32,
) -> Vec<FileInfo> {
    if let Some(file_entries) = versions.get(&aid).and_then(|v| v.group_file_ids.get(&gid)) {
        file_entries
            .iter()
            .map(|&(id, hash)| FileInfo {
                id,
                name_hash: hash,
            })
            .collect()
    } else {
        (0..file_count)
            .map(|i| FileInfo {
                id: i,
                name_hash: None,
            })
            .collect()
    }
}

pub fn group_version(
    versions: &std::collections::HashMap<u8, archive::ArchiveVersions>,
    aid: u8,
    gid: &u32,
) -> u32 {
    versions
        .get(&aid)
        .and_then(|v| v.group_versions.get(gid))
        .copied()
        .unwrap_or(0)
}
