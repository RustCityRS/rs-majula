#![allow(clippy::single_match)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! CRC verification — compare built cache against original source cache.

use std::path::Path;

use anyhow::Result;
use tracing::{info, warn};

pub fn verify_cache(source_cache: &Path, output_dir: &Path) -> Result<()> {
    use rs_dat2::archive;

    if !source_cache.join("main_file_cache.dat2").exists() {
        info!("Step 5: Skipping CRC verification (no source cache)");
        return Ok(());
    }

    info!("Step 5: Verifying per-group CRCs against original cache...");
    let mut source_store = rs_dat2::CacheStore::open(source_cache)?;
    let mut built_store = rs_dat2::CacheStore::open(output_dir)?;

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
                            let decomp = rs_dat2::compression::decompress(&sr).ok();
                            let our_gzip_len = decomp
                                .as_ref()
                                .and_then(|d| {
                                    rs_dat2::compression::compress(
                                        d,
                                        rs_dat2::compression::CompressionType::Gzip,
                                    )
                                    .ok()
                                })
                                .map(|g| g.len());
                            let our_bzip_len = decomp
                                .as_ref()
                                .and_then(|d| {
                                    rs_dat2::compression::compress(
                                        d,
                                        rs_dat2::compression::CompressionType::Bzip2,
                                    )
                                    .ok()
                                })
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
                    if let Ok(decomp) = rs_dat2::compression::decompress(&raw) {
                        let gz = rs_dat2::compression::compress(
                            &decomp,
                            rs_dat2::compression::CompressionType::Gzip,
                        )
                        .map(|v| v.len())
                        .unwrap_or(0);
                        let bz = rs_dat2::compression::compress(
                            &decomp,
                            rs_dat2::compression::CompressionType::Bzip2,
                        )
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

        match (src_raw, built_raw) {
            (Ok(Some(src)), Ok(Some(built))) => {
                let src_crc = rs_dat2::compute_group_crc(&src);
                let built_crc = rs_dat2::compute_group_crc(&built);
                if src_crc == built_crc {
                    info!(
                        "  ref table {:>2}: ✓ CRC {:#010x} ({}B)",
                        id,
                        src_crc,
                        src.len(),
                    );
                    ref_match += 1;
                } else {
                    let src_decomp = rs_dat2::compression::decompress(&src).ok();
                    let built_decomp = rs_dat2::compression::decompress(&built).ok();
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
                                .unwrap_or(std::cmp::min(s.len(), b.len()));
                            warn!("    → first difference at byte {}", first_diff);
                        }
                    }
                    ref_mismatch += 1;
                }
            }
            _ => {}
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
