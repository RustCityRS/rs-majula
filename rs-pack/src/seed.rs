//! Reference table seeding for empty caches.
//!
//! When building a JS5 cache from scratch (no bootstrap `cache/` copy),
//! repack modules fail because `store.reference_table(archive_id)` returns
//! `None`. This module pre-seeds skeleton reference tables into archive 255
//! so every repack module finds a valid table and can proceed.
//!
//! The seeded table contains:
//! - Group IDs from `manifest.json`
//! - Versions from the manifest's per-group `version` field
//! - CRCs set to 0 (the repack module overwrites them)
//! - File IDs derived from manifest data (e.g. `component_count` for interfaces)
//! - Name hashes where the manifest provides group names
//! - Format/flags from `rs_dat2::archive::ARCHIVE_FORMATS` constants

use std::path::Path;

use anyhow::{Context, Result};
use rs_dat2::CacheStore;
use rs_dat2::archive;
use rs_dat2::reference::{FileInfo, GroupInfo, ReferenceTable};
use tracing::trace;

// ── Flexible manifest reader ─────────────────────────────────────────────────

/// Minimal group metadata extracted from any manifest variant.
#[derive(Debug)]
struct SeedGroup {
    group_id: u32,
    version: u32,
    /// Name hash for the reference table (java_string_hash of name, or -1).
    name_hash: i32,
    /// File IDs within this group. `[0]` for single-file groups.
    file_ids: Vec<u32>,
}

/// Parse a manifest.json and extract the minimal fields needed for seeding.
///
/// Supports all manifest variants:
/// - `AssetManifest` (binary passthrough): group_id, version
/// - `ModelManifest`: group_id, version
/// - `SpriteManifest`: group_id, version
/// - `MapManifest`: group_id, version, name (-> name_hash)
/// - `InterfaceManifest`: group_id, version, component_count (-> file_ids)
///
/// Uses serde_json::Value for flexible parsing since manifest schemas differ.
///
/// `named` controls whether name hashes are computed. Only archives whose
/// reference table has the named flag (maps, songs) carry name hashes.
fn read_manifest_groups(manifest_path: &Path, named: bool) -> Result<(u8, Vec<SeedGroup>)> {
    let text = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let val: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let archive_id = val
        .get("archive_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("manifest missing archive_id"))? as u8;

    let groups_arr = val
        .get("groups")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("manifest missing groups array"))?;

    let mut seed_groups = Vec::with_capacity(groups_arr.len());

    for g in groups_arr {
        let group_id =
            g.get("group_id")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("group entry missing group_id"))? as u32;

        // Version: u16 in most manifests, stored as integer
        let version = g.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        // Name hash: only compute for named archives (maps, songs).
        // Maps have a "name" field (e.g. "m50_50"), songs have "file_name"
        // (e.g. "scape_main.mid") from which we strip the extension.
        let name_hash = if named {
            if let Some(name) = g.get("name").and_then(|v| v.as_str()) {
                rs_dat2::xtea::java_string_hash(name)
            } else if let Some(file_name) = g.get("file_name").and_then(|v| v.as_str()) {
                let base = file_name
                    .rsplit_once('.')
                    .map(|(b, _)| b)
                    .unwrap_or(file_name);
                rs_dat2::xtea::java_string_hash(base)
            } else {
                -1
            }
        } else {
            -1
        };

        // File IDs: interface manifests have component_count (multi-file groups)
        let file_ids = if let Some(count) = g.get("component_count").and_then(|v| v.as_u64()) {
            // Interface groups: files 0..component_count
            (0..count as u32).collect()
        } else {
            // Single-file group
            vec![0]
        };

        seed_groups.push(SeedGroup {
            group_id,
            version,
            name_hash,
            file_ids,
        });
    }

    Ok((archive_id, seed_groups))
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Pre-seed a skeleton reference table for `archive_id` from its manifest.
///
/// If the store already has a reference table for this archive, this is a
/// no-op (the existing table is left untouched). Otherwise it reads
/// `content_dir/manifest.json`, builds a [`ReferenceTable`] with stub CRCs,
/// and writes it to archive 255 so downstream repack modules can proceed.
///
/// # Arguments
/// * `store` — writable cache store (must have idx255 available)
/// * `archive_id` — the archive whose reference table to seed
/// * `content_dir` — directory containing `manifest.json`
///   (e.g. `content/maps/_unpack/` or `content/sprites/_unpack/`)
pub fn seed_reference_table(
    store: &mut CacheStore,
    archive_id: u8,
    content_dir: &Path,
) -> Result<()> {
    // Skip if a reference table already exists
    if store.reference_table(archive_id).is_some() {
        trace!(
            "archive {}: reference table already exists, skipping seed",
            archive_id
        );
        return Ok(());
    }

    let manifest_path = content_dir.join("manifest.json");
    if !manifest_path.exists() {
        anyhow::bail!(
            "no manifest.json in {} for archive {}",
            content_dir.display(),
            archive_id
        );
    }

    let fmt = archive::archive_format(archive_id)
        .ok_or_else(|| anyhow::anyhow!("unknown archive id {}", archive_id))?;
    let (manifest_archive_id, seed_groups) = read_manifest_groups(&manifest_path, fmt.named)?;

    // Sanity check: manifest's archive_id should match the requested one
    if manifest_archive_id != archive_id {
        anyhow::bail!(
            "manifest archive_id {} does not match requested archive_id {}",
            manifest_archive_id,
            archive_id
        );
    }

    let groups: Vec<GroupInfo> = seed_groups
        .iter()
        .map(|sg| {
            let files: Vec<FileInfo> = sg
                .file_ids
                .iter()
                .map(|&fid| FileInfo {
                    id: fid,
                    name_hash: if fmt.named { Some(-1) } else { None },
                })
                .collect();

            GroupInfo {
                id: sg.group_id,
                name_hash: if fmt.named { sg.name_hash } else { -1 },
                crc: 0,
                whirlpool: if fmt.whirlpooled {
                    Some([0u8; 64])
                } else {
                    None
                },
                version: sg.version,
                size: 0,
                file_count: files.len() as u32,
                files,
            }
        })
        .collect();

    let table = ReferenceTable {
        archive_id,
        format: fmt.format,
        version: 0,
        named: fmt.named,
        whirlpooled: fmt.whirlpooled,
        groups,
    };

    store
        .write_reference_table(archive_id, &table)
        .with_context(|| format!("writing seeded reference table for archive {}", archive_id))?;

    trace!(
        "archive {}: seeded reference table with {} groups (format={}, named={}, whirlpooled={})",
        archive_id,
        table.groups.len(),
        fmt.format,
        fmt.named,
        fmt.whirlpooled,
    );

    Ok(())
}

/// Seed reference tables for all archives that have manifests under `content_dir`.
///
/// Walks the known archive directories and seeds any that are missing from the
/// cache. This should be called before any repack modules run.
pub fn seed_all_reference_tables(store: &mut CacheStore, content_dir: &Path) -> Result<u32> {
    use crate::asset_roundtrip::PHASE2_ARCHIVES;

    let mut seeded = 0u32;

    // Phase 2 archives (binary passthrough, songs, models, sprites, etc.)
    for &(archive_id, dir_name) in PHASE2_ARCHIVES {
        let archive_dir = content_dir.join(dir_name).join("_unpack");
        if archive_dir.join("manifest.json").exists() {
            seed_reference_table(store, archive_id, &archive_dir)?;
            seeded += 1;
        }
    }

    // Phase 3: maps (archive 5)
    let maps_dir = content_dir.join("maps").join("_unpack");
    if maps_dir.join("manifest.json").exists() {
        seed_reference_table(store, 5, &maps_dir)?;
        seeded += 1;
    }

    // Phase 4: interfaces (archive 3)
    let if_dir = content_dir.join("interfaces").join("_unpack");
    if if_dir.join("manifest.json").exists() {
        seed_reference_table(store, 3, &if_dir)?;
        seeded += 1;
    }

    // Phase 4: textures (archive 9)
    let tex_dir = content_dir.join("textures").join("_unpack");
    if tex_dir.join("manifest.json").exists() {
        seed_reference_table(store, 9, &tex_dir)?;
        seeded += 1;
    }

    // Loading sprites (archive 32) — seeded via PHASE2_ARCHIVES if present,
    // but some configs may list it separately
    let ls_dir = content_dir.join("loading_sprites").join("_unpack");
    if ls_dir.join("manifest.json").exists() {
        seed_reference_table(store, 32, &ls_dir)?;
        seeded += 1;
    }

    Ok(seeded)
}
