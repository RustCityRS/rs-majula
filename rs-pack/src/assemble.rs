//! JS5 cache assembler.
//!
//! Three assembly modes:
//!
//! ## PackedData config archives
//! Reads the flat PackedData format (`.dat/.idx`) and writes entries back into
//! a JS5 `CacheStore` grouped by stride or single-group layout.
//!
//! ## From-scratch binary archives
//! Creates an empty JS5 cache and assembles binary passthrough archives
//! (animations, skeletons, DLLs, etc.) from manifest-described content
//! directories. Each group is read from disk, compressed, and written to
//! the cache with a freshly-built `ReferenceTable`.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use rs_dat2::archive::{self, ArchiveFormat, VersionTable};
use rs_dat2::reference::{FileInfo, GroupInfo, ReferenceTable};
use rs_dat2::{CacheStore, encode_group_entries};
use serde::Deserialize;

/// Read a PackedData `.dat/.idx` file pair and return a map of entry_id to entry bytes.
///
/// Only entries with non-zero length are included (empty entries are skipped).
///
/// File format (both files share the same layout):
///   - `.dat`: `p2(count)` header, then concatenated entry bytes
///   - `.idx`: `p2(count)` header, then `p2(len)` for each entry
pub fn read_packed_data(path: &Path, name: &str) -> Result<BTreeMap<u32, Vec<u8>>> {
    let dat_path = path.join(format!("{name}.dat"));
    let idx_path = path.join(format!("{name}.idx"));

    let dat =
        std::fs::read(&dat_path).with_context(|| format!("reading {}", dat_path.display()))?;
    let idx =
        std::fs::read(&idx_path).with_context(|| format!("reading {}", idx_path.display()))?;

    anyhow::ensure!(dat.len() >= 2, "dat file too short (missing count header)");
    anyhow::ensure!(idx.len() >= 2, "idx file too short (missing count header)");

    let dat_count = u16::from_be_bytes([dat[0], dat[1]]) as u32;
    let idx_count = u16::from_be_bytes([idx[0], idx[1]]) as u32;
    anyhow::ensure!(
        dat_count == idx_count,
        "dat count ({dat_count}) != idx count ({idx_count})"
    );

    let count = dat_count;
    anyhow::ensure!(
        idx.len() >= 2 + (count as usize) * 2,
        "idx file too short for {count} entries"
    );

    let mut entries = BTreeMap::new();
    let mut dat_offset = 2usize; // skip count header

    for id in 0..count {
        let idx_off = 2 + (id as usize) * 2;
        let entry_len = u16::from_be_bytes([idx[idx_off], idx[idx_off + 1]]) as usize;

        if entry_len > 0 {
            anyhow::ensure!(
                dat_offset + entry_len <= dat.len(),
                "dat underflow at entry {id}: need {entry_len} bytes at offset {dat_offset}, have {}",
                dat.len()
            );
            entries.insert(id, dat[dat_offset..dat_offset + entry_len].to_vec());
        }

        dat_offset += entry_len;
    }

    Ok(entries)
}

/// Assemble a strided archive from PackedData entries.
///
/// Entries are grouped by stride: `entry_id / stride = group_id`,
/// `entry_id % stride = child_id` within the group. Each group is encoded via
/// `encode_group_entries()` and written to the JS5 archive.
///
/// The reference table is updated in-place to reflect the new CRCs.
pub fn assemble_strided_archive(
    store: &mut CacheStore,
    archive_id: u8,
    stride: u32,
    entries: &BTreeMap<u32, Vec<u8>>,
) -> Result<AssembleStats> {
    let mut table = store
        .reference_table(archive_id)
        .ok_or_else(|| anyhow::anyhow!("no reference table for archive {archive_id}"))?
        .clone();

    // Group entries by stride bucket.
    let mut groups: BTreeMap<u32, Vec<(u32, &[u8])>> = BTreeMap::new();
    for (&global_id, data) in entries {
        let group_id = global_id / stride;
        let child_id = global_id % stride;
        groups
            .entry(group_id)
            .or_default()
            .push((child_id, data.as_slice()));
    }

    let mut stats = AssembleStats::default();

    for (&group_id, children) in &mut groups {
        children.sort_by_key(|(id, _)| *id);
        let blob = encode_group_entries(children);

        write_group_to_table(store, &mut table, archive_id, group_id, &blob)?;
        stats.groups_written += 1;
        stats.entries_written += children.len() as u32;
    }

    if stats.groups_written > 0 {
        store
            .write_reference_table(archive_id, &table)
            .context("write_reference_table")?;
    }

    Ok(stats)
}

/// Assemble a single-group config archive from PackedData entries.
///
/// All entries are written into the specified group within the archive (used
/// for archive-2 config types like flo, varp, idk, etc.).
pub fn assemble_config_group(
    store: &mut CacheStore,
    archive_id: u8,
    group_id: u32,
    entries: &BTreeMap<u32, Vec<u8>>,
) -> Result<AssembleStats> {
    let mut table = store
        .reference_table(archive_id)
        .ok_or_else(|| anyhow::anyhow!("no reference table for archive {archive_id}"))?
        .clone();

    let mut sorted: Vec<(u32, &[u8])> = entries
        .iter()
        .map(|(&id, data)| (id, data.as_slice()))
        .collect();
    sorted.sort_by_key(|(id, _)| *id);

    let blob = encode_group_entries(&sorted);

    write_group_to_table(store, &mut table, archive_id, group_id, &blob)?;

    store
        .write_reference_table(archive_id, &table)
        .context("write_reference_table")?;

    Ok(AssembleStats {
        groups_written: 1,
        entries_written: entries.len() as u32,
    })
}

/// Statistics from an assemble operation.
#[derive(Debug, Default)]
pub struct AssembleStats {
    pub groups_written: u32,
    pub entries_written: u32,
}

/// Write a group blob to the cache and update the reference table's CRC.
fn write_group_to_table(
    store: &mut CacheStore,
    table: &mut ReferenceTable,
    archive_id: u8,
    group_id: u32,
    blob: &[u8],
) -> Result<()> {
    let allow_uncompressed = archive::archive_format(archive_id)
        .map(|f| f.allow_uncompressed)
        .unwrap_or(false);
    let group_info = table
        .groups
        .iter()
        .find(|g| g.id == group_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "group {group_id} missing from reference table for archive {archive_id}"
            )
        })?;
    let version = group_info.version as u16;

    let new_crc = store
        .write_group_best(archive_id, group_id, blob, version, allow_uncompressed)
        .context("write_group_best")?;

    let g = table.groups.iter_mut().find(|g| g.id == group_id).unwrap();
    g.crc = new_crc;

    Ok(())
}

// ---------------------------------------------------------------------------
// Manifest-free binary archive assembly
// ---------------------------------------------------------------------------

/// Assemble a binary archive from content directory without any manifest
/// or versions file.
///
/// Groups are discovered by scanning `content_dir` for files matching the
/// extension (e.g. `*.anim`). Group IDs are parsed from filenames.
/// File counts are derived from the group blob's entry footer.
/// Compression is determined by `compress_best`.
pub fn assemble_archive_from_dir(
    store: &mut CacheStore,
    format: &ArchiveFormat,
    content_dir: &Path,
    extension: &str,
) -> Result<ReferenceTable> {
    let archive_id = format.id;

    // Scan directory for group files
    let mut group_files: Vec<(u32, std::path::PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(content_dir)
        .with_context(|| format!("reading {}", content_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(extension) {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let Ok(group_id) = stem.parse::<u32>() else {
            continue;
        };
        group_files.push((group_id, path));
    }
    group_files.sort_by_key(|(id, _)| *id);

    let mut group_infos = Vec::with_capacity(group_files.len());

    for (group_id, file_path) in &group_files {
        let raw_data =
            std::fs::read(file_path).with_context(|| format!("reading {}", file_path.display()))?;

        let file_count = rs_dat2::detect_file_count(&raw_data);

        let crc = store
            .write_group_best(
                archive_id,
                *group_id,
                &raw_data,
                0,
                format.allow_uncompressed,
            )
            .with_context(|| format!("writing archive {} group {}", archive_id, group_id))?;

        let files: Vec<FileInfo> = (0..file_count)
            .map(|i| FileInfo {
                id: i,
                name_hash: None,
            })
            .collect();

        let whirlpool = if format.whirlpooled {
            Some(rs_crypto::whirlpool(&raw_data))
        } else {
            None
        };

        group_infos.push(GroupInfo {
            id: *group_id,
            name_hash: -1,
            crc,
            whirlpool,
            version: 0,
            size: raw_data.len(),
            file_count,
            files,
        });
    }

    Ok(ReferenceTable {
        archive_id,
        format: format.format,
        version: 0,
        named: format.named,
        whirlpooled: format.whirlpooled,
        groups: group_infos,
    })
}

// ---------------------------------------------------------------------------
// Legacy manifest-based assembly (to be removed)
// ---------------------------------------------------------------------------

/// Manifest describing an archive's groups on disk.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub archive_id: u8,
    #[serde(default)]
    pub archive_name: String,
    #[serde(default = "default_ext")]
    pub file_extension: String,
    pub groups: Vec<ManifestGroup>,
}

fn default_ext() -> String {
    "bin".to_string()
}

/// A single group entry inside a manifest.
#[derive(Debug, Deserialize)]
pub struct ManifestGroup {
    pub group_id: u32,
    pub compression: u8,
    #[serde(default)]
    pub version: u16,
    #[serde(default)]
    pub crc: u32,
    #[serde(default)]
    pub compressed_len: u32,
    #[serde(default)]
    pub decompressed_len: u32,
    pub file_name: Option<String>,
    /// Per-group file extension override (e.g. shaders: "glsl", "cso").
    pub file_extension: Option<String>,
    #[serde(default)]
    pub name_hash: Option<i32>,
    #[serde(default)]
    pub file_ids: Option<Vec<u32>>,
}

/// Create an empty JS5 cache on disk.
///
/// - Writes a 520-byte zeroed sector 0 into `main_file_cache.dat2`.
/// - Creates empty `main_file_cache.idxN` for each requested archive ID.
/// - Creates an empty `main_file_cache.idx255` (reference table index).
pub fn create_empty_cache(dir: &Path, archive_ids: &[u8]) -> Result<()> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("creating cache dir {}", dir.display()))?;

    let mut dat2 = std::fs::File::create(dir.join("main_file_cache.dat2"))
        .context("creating main_file_cache.dat2")?;
    dat2.write_all(&[0u8; 520]).context("writing sector 0")?;

    for &id in archive_ids {
        std::fs::File::create(dir.join(format!("main_file_cache.idx{id}")))
            .with_context(|| format!("creating idx{id}"))?;
    }

    std::fs::File::create(dir.join("main_file_cache.idx255")).context("creating idx255")?;

    Ok(())
}

/// Create JS5 cache files on disk only if they don't already exist.
///
/// Unlike `create_empty_cache`, this never truncates existing files — it only
/// creates missing ones. This allows incremental archive rebuilds to reuse
/// the existing cache instead of wiping it every time.
pub fn open_or_create_cache(dir: &Path, archive_ids: &[u8]) -> Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(dir)
        .with_context(|| format!("creating cache dir {}", dir.display()))?;

    let dat2_path = dir.join("main_file_cache.dat2");
    if !dat2_path.exists() {
        let mut dat2 =
            std::fs::File::create(&dat2_path).context("creating main_file_cache.dat2")?;
        dat2.write_all(&[0u8; 520]).context("writing sector 0")?;
    }

    for &id in archive_ids {
        let idx_path = dir.join(format!("main_file_cache.idx{id}"));
        if !idx_path.exists() {
            std::fs::File::create(&idx_path).with_context(|| format!("creating idx{id}"))?;
        }
    }

    let idx255_path = dir.join("main_file_cache.idx255");
    if !idx255_path.exists() {
        std::fs::File::create(&idx255_path).context("creating idx255")?;
    }

    Ok(())
}

/// Assemble a binary passthrough archive from a manifest and raw group files.
///
/// Reads `manifest.json` from `content_dir`, compresses each group with its
/// declared compression type, writes it to `store`, and returns a fully-
/// populated `ReferenceTable` ready to be written to archive 255.
///
/// For archive 30 (DLLs) the archive format has `whirlpooled: true`; in that
/// case a 64-byte Whirlpool digest is computed over the decompressed group data
/// and stored in each `GroupInfo`.
pub fn assemble_binary_archive(
    store: &mut CacheStore,
    archive_id: u8,
    content_dir: &Path,
    versions: &VersionTable,
    format: &ArchiveFormat,
) -> Result<ReferenceTable> {
    let manifest_path = content_dir.join("manifest.json");
    let manifest_text = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading manifest for archive {archive_id}"))?;
    let manifest: Manifest = serde_json::from_str(&manifest_text)
        .with_context(|| format!("parsing manifest for archive {archive_id}"))?;

    anyhow::ensure!(
        manifest.archive_id == archive_id,
        "manifest archive_id {} != expected {archive_id}",
        manifest.archive_id
    );

    let archive_versions = versions
        .get(&archive_id)
        .ok_or_else(|| anyhow::anyhow!("archive {archive_id} not in versions.json"))?;

    let mut group_infos = Vec::with_capacity(manifest.groups.len());

    for mg in &manifest.groups {
        // Resolve file path: prefer file_name, fall back to group_id.extension.
        // Per-group file_extension overrides the manifest-level default.
        let file_path = if let Some(ref name) = mg.file_name {
            content_dir.join(name)
        } else {
            let ext = mg
                .file_extension
                .as_deref()
                .unwrap_or(&manifest.file_extension);
            content_dir.join(format!("{}.{}", mg.group_id, ext))
        };

        let raw_data = std::fs::read(&file_path)
            .with_context(|| format!("reading {}", file_path.display()))?;

        let version = archive_versions
            .group_versions
            .get(&mg.group_id)
            .copied()
            .unwrap_or(mg.version as u32) as u16;

        let crc = store
            .write_group_best(
                archive_id,
                mg.group_id,
                &raw_data,
                version,
                format.allow_uncompressed,
            )
            .with_context(|| format!("writing archive {archive_id} group {}", mg.group_id))?;

        let name_hash = mg.name_hash.unwrap_or(-1);

        let (file_count, files) = if let Some(ref ids) = mg.file_ids {
            let files: Vec<FileInfo> = ids
                .iter()
                .map(|&id| FileInfo {
                    id,
                    name_hash: None,
                })
                .collect();
            (files.len() as u32, files)
        } else {
            (
                1,
                vec![FileInfo {
                    id: 0,
                    name_hash: None,
                }],
            )
        };

        // Whirlpool digest for whirlpooled archives (archive 30)
        let whirlpool = if format.whirlpooled {
            Some(rs_crypto::whirlpool(&raw_data))
        } else {
            None
        };

        group_infos.push(GroupInfo {
            id: mg.group_id,
            name_hash,
            crc,
            whirlpool,
            version: version as u32,
            size: raw_data.len(),
            file_count,
            files,
        });
    }

    Ok(ReferenceTable {
        archive_id,
        format: format.format,
        version: archive_versions.table_version,
        named: format.named,
        whirlpooled: format.whirlpooled,
        groups: group_infos,
    })
}

/// Convenience: assemble a binary archive and write its reference table in one call.
///
/// Returns the `ReferenceTable` that was written.
pub fn assemble_and_write_binary_archive(
    store: &mut CacheStore,
    archive_id: u8,
    content_dir: &Path,
    versions: &VersionTable,
) -> Result<ReferenceTable> {
    let format = archive::archive_format(archive_id)
        .ok_or_else(|| anyhow::anyhow!("unknown archive format for id {archive_id}"))?;

    let table = assemble_binary_archive(store, archive_id, content_dir, versions, format)?;
    store
        .write_reference_table(archive_id, &table)
        .with_context(|| format!("writing reference table for archive {archive_id}"))?;

    Ok(table)
}
