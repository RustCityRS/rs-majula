use crate::pack::pack_registry::PackFile;
use crate::startup::{is_up_to_date, max_of, newest_mtime, write_stamp};
use anyhow::{Context, Result};
use rayon::prelude::*;
use rs_crypto::whirlpool;
use rs_dat2::archive::{self, ArchiveFormat, ArchiveVersions};
use rs_dat2::compression::{CompressionType, compress, decompress};
use rs_dat2::reference::{FileInfo, GroupInfo, ReferenceTable};
use rs_dat2::xtea::java_string_hash;
use rs_dat2::{CacheStore, compute_group_crc, detect_file_count, prepare_group};
use std::collections::{BTreeMap, HashMap, hash_map::Entry};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub fn stamp_path(output_dir: &Path, archive_id: u8) -> PathBuf {
    output_dir.join(format!(".stamp_archive_{archive_id}"))
}

pub fn load_group_files(unpack_dir: &Path, extension: &str) -> Vec<(u32, PathBuf)> {
    let mut map: HashMap<u32, PathBuf> = HashMap::new();
    if !unpack_dir.is_dir() {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(unpack_dir) else {
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
        if !extension.is_empty() && ext != extension {
            continue;
        }
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

pub fn load_name_hashes(pack_dir: &Path, archive_id: u8, group_ids: &[u32]) -> HashMap<u32, i32> {
    let pack_name = pack_filename_for_archive(archive_id);
    let mut result = HashMap::new();

    let hashes_path = pack_dir.join(format!("{pack_name}.hashes"));
    if hashes_path.exists() {
        if let Ok(text) = fs::read_to_string(&hashes_path) {
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
        if let Ok(pf) = PackFile::load(&pack_path) {
            for &gid in group_ids {
                if let Entry::Vacant(e) = result.entry(gid) {
                    if let Some(name) = pf.get_by_id(gid) {
                        e.insert(java_string_hash(name));
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

impl PreparedGroup {
    pub fn new(
        aid: u8,
        id: u32,
        data: &[u8],
        versions: &HashMap<u8, ArchiveVersions>,
    ) -> Result<Self> {
        let format = archive::archive_format(aid).unwrap();
        let version = group_version(versions, aid, &id);
        let (raw, crc) = prepare_group(aid, id, data, version as u16, format.allow_uncompressed)?;
        let whirlpool = format.whirlpooled.then(|| whirlpool(&raw[..raw.len() - 2]));
        Ok(Self {
            id,
            raw,
            crc,
            version,
            size: data.len(),
            name_hash: -1,
            whirlpool,
            files: build_file_infos(versions, aid, id, detect_file_count(data) as u32),
        })
    }
}

pub fn write_archive(
    store: &mut CacheStore,
    aid: u8,
    format: &ArchiveFormat,
    versions: &HashMap<u8, ArchiveVersions>,
    groups: Vec<PreparedGroup>,
    label: &str,
) -> Result<()> {
    let mut group_infos = Vec::with_capacity(groups.len());
    for g in groups {
        store.write_raw_group(aid, g.id, &g.raw)?;
        group_infos.push(GroupInfo {
            id: g.id,
            name_hash: g.name_hash,
            crc: g.crc,
            whirlpool: g.whirlpool,
            version: g.version,
            size: g.size,
            file_count: g.files.len() as u32,
            files: g.files,
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
    store: &mut CacheStore,
    aid: u8,
    format: &ArchiveFormat,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    write_archive(store, aid, format, versions, Vec::new(), "(empty)")
}

/// Build file info list from version data if available, else from file_count.
pub fn build_file_infos(
    versions: &HashMap<u8, ArchiveVersions>,
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

pub fn group_version(versions: &HashMap<u8, ArchiveVersions>, aid: u8, gid: &u32) -> u32 {
    versions
        .get(&aid)
        .and_then(|v| v.group_versions.get(gid))
        .copied()
        .unwrap_or(0)
}

pub fn open_or_create_cache(dir: &Path, archive_ids: &[u8]) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("creating cache dir {}", dir.display()))?;

    let dat2_path = dir.join("main_file_cache.dat2");
    if !dat2_path.exists() {
        let mut dat2 = fs::File::create(&dat2_path).context("creating main_file_cache.dat2")?;
        dat2.write_all(&[0u8; 520]).context("writing sector 0")?;
    }

    for &id in archive_ids {
        let idx_path = dir.join(format!("main_file_cache.idx{id}"));
        if !idx_path.exists() {
            fs::File::create(&idx_path).with_context(|| format!("creating idx{id}"))?;
        }
    }

    let idx255_path = dir.join("main_file_cache.idx255");
    if !idx255_path.exists() {
        fs::File::create(&idx255_path).context("creating idx255")?;
    }

    Ok(())
}

const SIMPLE_ARCHIVES: &[u8] = &[
    archive::BASES,
    archive::MODELS,
    archive::BINARY,
    archive::JINGLES,
    archive::CLIENTSCRIPT,
    archive::FONTMETRICS,
    archive::VORBIS,
    archive::MUSIC_SAMPLES,
    archive::WORLDMAP_DATA,
    archive::QUICKCHAT,
    archive::QUICKCHAT_GLOBAL,
    archive::MATERIALS,
    archive::PARTICLES,
    archive::DEFAULTS,
    archive::BILLBOARDS,
    archive::DLLS,
    archive::SHADERS,
    archive::LOADING_SCREENS,
    archive::WORLDMAP_LOCS,
];

pub fn pack_binary_archives(
    store: &mut CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    for &aid in SIMPLE_ARCHIVES {
        let format = archive::archive_format(aid).unwrap();
        let unpack_dir = content_dir.join(format.content_dir).join("_unpack");
        let donor_dir = content_dir.join(format.content_dir).join("donor");

        let arch_stamp = stamp_path(output_dir, aid);
        let arch_input = max_of(&[
            newest_mtime(&unpack_dir, &[]),
            newest_mtime(&donor_dir, &[]),
        ]);
        if !force_rebuild && is_up_to_date(&arch_stamp, arch_input) {
            info!(
                "  archive {:>2} ({}) -- up to date, skipping",
                aid, format.content_dir
            );
            continue;
        }

        let group_files = overlay_groups(
            load_group_files(&unpack_dir, ""),
            load_group_files(&donor_dir, ""),
        );

        if group_files.is_empty() {
            write_empty_archive(store, aid, format, versions)?;
            write_stamp(&arch_stamp)?;
            continue;
        }

        let name_hashes: HashMap<u32, i32> = if format.named {
            let gids: Vec<u32> = group_files.iter().map(|(gid, _)| *gid).collect();
            load_name_hashes(pack_dir, aid, &gids)
        } else {
            HashMap::new()
        };

        let prepared: Vec<PreparedGroup> = group_files
            .par_iter()
            .map(|(gid, path)| {
                let data = fs::read(path)?;
                let mut group = PreparedGroup::new(aid, *gid, &data, versions)?;
                group.name_hash = name_hashes.get(gid).copied().unwrap_or(-1);
                Ok(group)
            })
            .collect::<Result<Vec<_>>>()?;

        write_archive(store, aid, format, versions, prepared, "from _unpack/")?;
        write_stamp(&arch_stamp)?;
    }
    Ok(())
}

/// `derived`'s groups with `donor`'s laid over them: a donor group replaces
/// the derived one of its id, or adds one the cache lacks. Sorted by id.
fn overlay_groups(derived: Vec<(u32, PathBuf)>, donor: Vec<(u32, PathBuf)>) -> Vec<(u32, PathBuf)> {
    let mut groups: BTreeMap<_, _> = derived.into_iter().collect();
    groups.extend(donor);
    groups.into_iter().collect()
}

pub fn verify_cache(source_cache: &Path, output_dir: &Path) -> Result<()> {
    if !source_cache.join("main_file_cache.dat2").exists() {
        info!("Step 5: Skipping CRC verification (no source cache)");
        return Ok(());
    }

    info!("Step 5: Verifying per-group CRCs against original cache...");
    let mut source_store = CacheStore::open(source_cache)?;
    let mut built_store = CacheStore::open(output_dir)?;

    let mut archives_verified = 0u32;
    let mut archives_missing = 0u32;
    let mut total_crc_mismatches = 0u32;

    for &id in archive::ARCHIVE_IDS {
        let (Some(src_table), Some(built_table)) = (
            source_store.reference_table(id).cloned(),
            built_store.reference_table(id).cloned(),
        ) else {
            if source_store.reference_table(id).is_some() {
                warn!("  archive {:>2}: not built (source has groups)", id);
                archives_missing += 1;
            }
            continue;
        };

        if src_table.groups.len() != built_table.groups.len() {
            warn!(
                "  archive {:>2}: group count mismatch (source={}, built={})",
                id,
                src_table.groups.len(),
                built_table.groups.len()
            );
            archives_missing += 1;
            continue;
        }

        let mut archive_mismatches = 0u32;
        for (src_g, built_g) in src_table.groups.iter().zip(built_table.groups.iter()) {
            if src_g.id != built_g.id {
                warn!(
                    "  archive {:>2}: group id order mismatch (source={}, built={})",
                    id, src_g.id, built_g.id
                );
                archive_mismatches += 1;
                continue;
            }
            if src_g.crc != built_g.crc {
                warn!(
                    "  archive {:>2} group {:>5}: CRC mismatch (source={:#010x}, built={:#010x}) files: src={} built={}",
                    id, src_g.id, src_g.crc, built_g.crc, src_g.file_count, built_g.file_count,
                );
                if let (Ok(Some(sg)), Ok(Some(bg))) = (
                    source_store.read_group(id, src_g.id),
                    built_store.read_group(id, src_g.id),
                ) {
                    let mut se: Vec<_> = sg.entries.iter().collect();
                    let mut be: Vec<_> = bg.entries.iter().collect();
                    se.sort_by_key(|e| e.id);
                    be.sort_by_key(|e| e.id);
                    let mut shown = 0u32;
                    for (s, b) in se.iter().zip(be.iter()) {
                        if s.id != b.id {
                            warn!("    file ID mismatch: src={} built={}", s.id, b.id);
                            break;
                        }
                        if s.data != b.data {
                            let sh: String =
                                s.data.iter().take(40).map(|b| format!("{b:02x}")).collect();
                            let bh: String =
                                b.data.iter().take(40).map(|b| format!("{b:02x}")).collect();
                            warn!(
                                "    archive {} group {} diverge file {}: src={}B built={}B\n      src: {}\n      blt: {}",
                                id,
                                src_g.id,
                                s.id,
                                s.data.len(),
                                b.data.len(),
                                sh,
                                bh,
                            );
                            shown += 1;
                            if shown >= 3 {
                                break;
                            }
                        }
                    }
                    if se.len() != be.len() {
                        warn!("    entry count: src={} built={}", se.len(), be.len());
                    }
                    let all_match = se
                        .iter()
                        .zip(be.iter())
                        .all(|(s, b)| s.id == b.id && s.data == b.data);
                    if all_match && se.len() == be.len() {
                        if let (Ok(Some(sr)), Ok(Some(br))) = (
                            source_store.read_raw_group_bytes(id, src_g.id),
                            built_store.read_raw_group_bytes(id, src_g.id),
                        ) {
                            let s_codec = sr.first().copied().unwrap_or(255);
                            let b_codec = br.first().copied().unwrap_or(255);
                            let decomp = decompress(&sr).ok();
                            let our_gzip_len = decomp
                                .as_ref()
                                .and_then(|d| compress(d, CompressionType::Gzip).ok())
                                .map(|g| g.len());
                            let our_bzip_len = decomp
                                .as_ref()
                                .and_then(|d| compress(d, CompressionType::Bzip2).ok())
                                .map(|b| b.len());
                            warn!(
                                "    ALL entries identical — src codec={} {}B, built codec={} {}B | our gzip={:?} bzip={:?}",
                                s_codec,
                                sr.len(),
                                b_codec,
                                br.len(),
                                our_gzip_len,
                                our_bzip_len,
                            );
                        }
                    }
                }
                archive_mismatches += 1;
            }
        }

        if archive_mismatches > 0 {
            warn!("  archive {} full codec survey:", id);
            for g in &src_table.groups {
                if let Ok(Some(raw)) = source_store.read_raw_group_bytes(id, g.id) {
                    let s_codec = match raw.first().copied().unwrap_or(255) {
                        0 => "none",
                        1 => "bz2 ",
                        2 => "gzip",
                        _ => "????",
                    };
                    if let Ok(decomp) = decompress(&raw) {
                        let gz = compress(&decomp, CompressionType::Gzip)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        let bz = compress(&decomp, CompressionType::Bzip2)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        let winner = if bz <= gz { "bz2" } else { "gz" };
                        let flag = if (winner == "bz2" && s_codec == "gzip")
                            || (winner == "gz" && s_codec == "bz2 ")
                        {
                            " <<<MISMATCH"
                        } else {
                            ""
                        };
                        warn!(
                            "    g{:>3} src={} decomp={:>7} gz={:>6} bz={:>6} best={}{}",
                            g.id,
                            s_codec,
                            decomp.len(),
                            gz,
                            bz,
                            winner,
                            flag
                        );
                    }
                }
            }
        }

        if archive_mismatches == 0 {
            info!(
                "  archive {:>2}: ✓ {} groups, all CRCs match",
                id,
                src_table.groups.len()
            );
            archives_verified += 1;
        } else {
            warn!(
                "  archive {:>2}: {} CRC mismatches out of {} groups",
                id,
                archive_mismatches,
                src_table.groups.len()
            );
            total_crc_mismatches += archive_mismatches;
        }
    }

    info!(
        "  {}/{} archives fully verified, {} missing, {} total CRC mismatches",
        archives_verified,
        archive::ARCHIVE_IDS.len(),
        archives_missing,
        total_crc_mismatches
    );

    if total_crc_mismatches > 0 {
        warn!(
            "⚠ CRC verification found {} mismatches — built cache diverges from source",
            total_crc_mismatches
        );
    }

    info!("Step 6: Verifying reference table CRCs...");
    let mut ref_match = 0u32;
    let mut ref_mismatch = 0u32;

    for &id in archive::ARCHIVE_IDS {
        let src_raw = source_store.read_raw_group_bytes(255, id as u32);
        let built_raw = built_store.read_raw_group_bytes(255, id as u32);

        if let (Ok(Some(src)), Ok(Some(built))) = (src_raw, built_raw) {
            let src_crc = compute_group_crc(&src);
            let built_crc = compute_group_crc(&built);
            if src_crc == built_crc {
                info!(
                    "  ref table {:>2}: ✓ CRC {:#010x} ({}B)",
                    id,
                    src_crc,
                    src.len(),
                );
                ref_match += 1;
            } else {
                let src_decomp = decompress(&src).ok();
                let built_decomp = decompress(&built).ok();
                let src_sz = src_decomp.as_ref().map(|d| d.len()).unwrap_or(0);
                let built_sz = built_decomp.as_ref().map(|d| d.len()).unwrap_or(0);
                let data_match = src_decomp
                    .as_ref()
                    .zip(built_decomp.as_ref())
                    .map(|(s, b)| s == b)
                    .unwrap_or(false);
                warn!(
                    "  ref table {:>2}: ✗ src={:#010x} ({}B) built={:#010x} ({}B) decomp_match={} [BUILT]",
                    id,
                    src_crc,
                    src.len(),
                    built_crc,
                    built.len(),
                    data_match,
                );
                if data_match {
                    warn!("    → decompressed bytes identical — compression codec differs");
                } else {
                    warn!(
                        "    → decompressed bytes differ (src={}B built={}B)",
                        src_sz, built_sz
                    );
                    if let (Some(s), Some(b)) = (&src_decomp, &built_decomp) {
                        let first_diff = s
                            .iter()
                            .zip(b.iter())
                            .position(|(a, b)| a != b)
                            .unwrap_or(s.len().min(b.len()));
                        warn!("    → first difference at byte {}", first_diff);
                    }
                }
                ref_mismatch += 1;
            }
        }
    }

    info!(
        "  Reference tables: {}/{} match, {} differ",
        ref_match,
        ref_match + ref_mismatch,
        ref_mismatch,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::overlay_groups;
    use std::path::PathBuf;

    #[test]
    fn a_donor_group_adds_or_replaces_by_id() {
        let derived = vec![(1, PathBuf::from("u/1.bin")), (3, PathBuf::from("u/3.bin"))];
        let donor = vec![(2, PathBuf::from("d/2.bin")), (3, PathBuf::from("d/3.bin"))];
        assert_eq!(
            overlay_groups(derived, donor),
            vec![
                (1, PathBuf::from("u/1.bin")),
                (2, PathBuf::from("d/2.bin")),
                (3, PathBuf::from("d/3.bin")),
            ]
        );
    }
}
