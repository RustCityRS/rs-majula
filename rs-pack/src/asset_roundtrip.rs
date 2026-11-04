//! Phase 2 — Opaque asset unpack/repack.
//!
//! Asset archives contain binary blobs (models, sprites, textures, sounds,
//! MIDI, fonts, etc.) that we round-trip without format interpretation.
//!
//! **Unpack**: each group becomes `<group_id>.<ext>` (decompressed payload)
//! under `_unpack/<archive_name>/`, with a `manifest.json` recording the
//! compression type, version suffix, original CRC, and file extension.
//!
//! **Repack**: reads `.<ext>` files + `manifest.json`, re-compresses each
//! group with the recorded codec, and writes back to the cache. Byte-
//! identical output is guaranteed by the codec-parity proven in Phase 0.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use rs_dat2::CacheStore;
use rs_dat2::compression::{self, CompressionType};
use serde::{Deserialize, Serialize};

/// Archive IDs that Phase 2 covers (opaque assets).
/// Excludes: 2 (CONFIG, Phase 1), 3 (INTERFACES, Phase 4),
/// 5 (MAPS/XTEA, Phase 3), 12 (CLIENTSCRIPT, Phase 4),
/// 16–22 (CONFIG_*, Phase 1), 255 (REFERENCE_TABLES, Phase 5).
pub const PHASE2_ARCHIVES: &[(u8, &str)] = &[
    (0, "animations"),
    (1, "skeletons"),
    (4, "synth"),
    (6, "songs"),
    (7, "models"),
    (8, "sprites"),
    (10, "binary"),
    (11, "jingles"),
    (13, "fontmetrics"),
    (14, "vorbis"),
    (15, "music_samples"),
    (23, "worldmap_data"),
    (24, "quickchat"),
    (25, "quickchat_global"),
    (26, "materials"),
    (27, "particles"),
    (28, "defaults"),
    (29, "billboards"),
    (30, "dlls"),
    (31, "shaders"),
    (32, "loading_sprites"),
    (33, "loading_backgrounds"),
    (36, "worldmap_locs"),
];

/// All archives that should be passthrough-copied in a full cache build
/// (anything not rebuilt by Phase 1 config repack or excluded entirely).
/// This covers Phase 2 assets + Phase 3/4 archives as raw passthrough.
/// Archive 5 (MAPS) removed in Phase 3 — now rebuilt from text source.
/// Archive 3 (INTERFACES) removed in Phase 4 — now rebuilt from text source.
pub const PASSTHROUGH_ARCHIVES: &[u8] = &[
    0, 1, 4, 6, 7, 8, 10, 11, 12, 13, 14, 15, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 36,
];

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetManifest {
    pub archive_id: u8,
    pub archive_name: String,
    #[serde(default = "default_extension")]
    pub file_extension: String,
    pub groups: Vec<GroupEntry>,
}

fn default_extension() -> String {
    "bin".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupEntry {
    pub group_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_extension: Option<String>,
    pub compression: u8,
    pub version: u16,
    pub compressed_len: u32,
    pub decompressed_len: u32,
    pub crc: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_hash: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<u32>>,
}

/// Unpack a single asset archive: each group → `<gid>.bin` + manifest.json.
///
/// `archive_dir` is the final output directory (e.g. `content/sprites/_unpack/`).
pub fn unpack_asset_archive(
    store: &mut CacheStore,
    archive_id: u8,
    archive_name: &str,
    archive_dir: &Path,
) -> Result<u32> {
    std::fs::create_dir_all(archive_dir)?;

    let group_count = store.group_count(archive_id).unwrap_or(0);
    let mut manifest = AssetManifest {
        archive_id,
        archive_name: archive_name.to_string(),
        file_extension: "bin".to_string(),
        groups: Vec::new(),
    };

    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(archive_id, gid)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let comp_type = raw[0];
        let compressed_len = u32::from_be_bytes([raw[1], raw[2], raw[3], raw[4]]) as usize;

        let header_size = if comp_type == 0 { 5 } else { 9 };
        let decompressed_len = if comp_type != 0 && raw.len() >= 9 {
            u32::from_be_bytes([raw[5], raw[6], raw[7], raw[8]])
        } else {
            compressed_len as u32
        };

        // Version suffix is the last 2 bytes (after header + compressed payload)
        let container_end = header_size + compressed_len;
        let version = if raw.len() >= container_end + 2 {
            u16::from_be_bytes([raw[container_end], raw[container_end + 1]])
        } else {
            0
        };

        // CRC is computed on the container (excluding version suffix)
        let crc = rs_io::crc::getcrc(&raw, 0, container_end) as u32;

        let decompressed = match compression::decompress(&raw) {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "  warning: archive {} group {}: decompress failed ({}), storing raw container",
                    archive_id, gid, e
                );
                // Store the raw container bytes as-is for passthrough
                let bin_path = archive_dir.join(format!("{gid}.bin"));
                std::fs::write(&bin_path, &raw)?;
                manifest.groups.push(GroupEntry {
                    group_id: gid,
                    file_name: None,
                    file_extension: None,
                    compression: 0xFF, // sentinel: raw passthrough
                    version: 0,
                    compressed_len: raw.len() as u32,
                    decompressed_len: raw.len() as u32,
                    crc: rs_io::crc::getcrc(&raw, 0, raw.len()) as u32,
                    name_hash: None,
                    file_ids: None,
                });
                written += 1;
                continue;
            }
        };
        let bin_path = archive_dir.join(format!("{gid}.bin"));
        std::fs::write(&bin_path, &decompressed)?;

        manifest.groups.push(GroupEntry {
            group_id: gid,
            file_name: None,
            file_extension: None,
            compression: comp_type,
            version,
            compressed_len: compressed_len as u32,
            decompressed_len,
            crc,
            name_hash: None,
            file_ids: None,
        });
        written += 1;
    }

    let manifest_path = archive_dir.join("manifest.json");
    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, json)?;

    Ok(written)
}

/// Repack a single asset archive from `.<ext>` files + manifest.json.
pub fn repack_asset_archive(
    store: &mut CacheStore,
    archive_id: u8,
    input_dir: &Path,
) -> Result<RepackAssetStats> {
    let manifest_path = input_dir.join("manifest.json");
    let manifest: AssetManifest = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?,
    )?;

    let ext = &manifest.file_extension;

    let mut table = store
        .reference_table(archive_id)
        .ok_or_else(|| anyhow::anyhow!("no reference table for archive {}", archive_id))?
        .clone();

    let mut stats = RepackAssetStats {
        groups_written: 0,
        groups_unchanged: 0,
    };

    for entry in &manifest.groups {
        let file_path = if let Some(ref name) = entry.file_name {
            input_dir.join(name)
        } else {
            let group_ext = entry.file_extension.as_deref().unwrap_or(ext);
            input_dir.join(format!("{}.{}", entry.group_id, group_ext))
        };
        let data = std::fs::read(&file_path)
            .with_context(|| format!("reading {}", file_path.display()))?;

        // 0xFF sentinel means raw passthrough — write bytes directly
        if entry.compression == 0xFF {
            store
                .write_raw_group(archive_id, entry.group_id, &data)
                .with_context(|| {
                    format!(
                        "write_raw_group archive {} group {}",
                        archive_id, entry.group_id
                    )
                })?;
            stats.groups_unchanged += 1;
            continue;
        }

        let comp = CompressionType::try_from(entry.compression)
            .map_err(|_| anyhow::anyhow!("unknown compression {}", entry.compression))?;

        let new_crc = store
            .write_group(archive_id, entry.group_id, &data, comp, entry.version)
            .with_context(|| {
                format!(
                    "write_group archive {} group {}",
                    archive_id, entry.group_id
                )
            })?;

        if new_crc != entry.crc {
            stats.groups_written += 1;
            if let Some(g) = table.groups.iter_mut().find(|g| g.id == entry.group_id) {
                g.crc = new_crc;
            }
        } else {
            stats.groups_unchanged += 1;
        }
    }

    if stats.groups_written > 0 {
        store
            .write_reference_table(archive_id, &table)
            .context("write_reference_table")?;
    }

    Ok(stats)
}

pub struct RepackAssetStats {
    pub groups_written: u32,
    pub groups_unchanged: u32,
}

/// Unpack all Phase 2 asset archives.
///
/// Layout: `content_dir/<archive_name>/_unpack/<gid>.bin` (matches 2004scape).
pub fn unpack_all_assets(store: &mut CacheStore, content_dir: &Path) -> Result<BTreeMap<u8, u32>> {
    let mut counts = BTreeMap::new();
    for &(archive_id, name) in PHASE2_ARCHIVES {
        if !store.has_archive(archive_id) {
            continue;
        }
        let archive_dir = content_dir.join(name).join("_unpack");
        let n = unpack_asset_archive(store, archive_id, name, &archive_dir)?;
        counts.insert(archive_id, n);
    }
    Ok(counts)
}

/// Repack all Phase 2 asset archives from unpacked .bin + manifest.
///
/// Layout: `content_dir/<archive_name>/_unpack/<gid>.bin` (matches 2004scape).
pub fn repack_all_assets(store: &mut CacheStore, content_dir: &Path) -> Result<(u32, u32)> {
    let mut total_written = 0u32;
    let mut total_unchanged = 0u32;
    for &(archive_id, name) in PHASE2_ARCHIVES {
        let archive_dir = content_dir.join(name).join("_unpack");
        if !archive_dir.join("manifest.json").exists() {
            continue;
        }
        let stats = repack_asset_archive(store, archive_id, &archive_dir)?;
        total_written += stats.groups_written;
        total_unchanged += stats.groups_unchanged;
    }
    Ok((total_written, total_unchanged))
}

/// Copy an archive verbatim from source cache to target cache (raw passthrough).
pub fn copy_archive_raw(
    source: &mut CacheStore,
    target: &mut CacheStore,
    archive_id: u8,
) -> Result<u32> {
    let group_count = source.group_count(archive_id).unwrap_or(0);
    let mut copied = 0u32;
    for gid in 0..group_count {
        let Some(raw) = source.read_raw_group_bytes(archive_id, gid)? else {
            continue;
        };
        target.write_raw_group(archive_id, gid, &raw)?;
        copied += 1;
    }

    // Copy the reference table too
    if let Some(table) = source.reference_table(archive_id) {
        target.write_reference_table(archive_id, &table.clone())?;
    }

    Ok(copied)
}
