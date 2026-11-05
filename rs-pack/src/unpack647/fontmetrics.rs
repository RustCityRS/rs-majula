#![allow(clippy::needless_range_loop)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Unpack font metrics (archive 13) to text format.
//!
//! From `FontMetrics.java`: version, glyph widths, optional complex
//! spacing/kerning with delta-encoded bearing arrays.

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use tracing::info;

use crate::pack::pack_registry::PackFile;

const ARCHIVE_FONTMETRICS: u8 = 13;

pub fn unpack_fontmetrics(
    store: &mut CacheStore,
    pack_dir: &Path,
    content_dir: &Path,
) -> Result<u32> {
    let fontmetrics_pack = PackFile::load(&pack_dir.join("fontmetrics.pack")).ok();
    let archive_dir = content_dir.join("fontmetrics").join("_unpack");
    std::fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(ARCHIVE_FONTMETRICS).unwrap_or(0);
    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_FONTMETRICS, gid)? else {
            continue;
        };
        let Some(entry) = group.entries.first() else {
            continue;
        };
        if entry.data.is_empty() {
            continue;
        }

        let name = fontmetrics_pack
            .as_ref()
            .and_then(|p| p.get_by_id(gid).map(|s| s.to_string()))
            .unwrap_or_else(|| format!("fontmetrics_{}", gid));

        let out_path = archive_dir.join(format!("{}.fontmetrics", name));
        let mut file = std::fs::File::create(&out_path)?;

        let data = &entry.data;
        let mut pos = 0;

        let version = data[pos];
        pos += 1;
        writeln!(file, "version={}", version)?;

        let has_spacing = data[pos] == 1;
        pos += 1;
        writeln!(
            file,
            "has_spacing={}",
            if has_spacing { "yes" } else { "no" }
        )?;

        // 256 glyph widths
        writeln!(
            file,
            "glyph_widths={}",
            (0..256)
                .map(|i| {
                    let v = data[pos + i];
                    v.to_string()
                })
                .collect::<Vec<_>>()
                .join(",")
        )?;
        pos += 256;

        if has_spacing {
            // glyph heights
            let mut glyph_heights = [0u8; 256];
            for i in 0..256 {
                glyph_heights[i] = data[pos];
                pos += 1;
            }
            writeln!(
                file,
                "glyph_heights={}",
                glyph_heights
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )?;

            // glyph spacings (baseline offsets)
            let mut glyph_spacings = [0u8; 256];
            for i in 0..256 {
                glyph_spacings[i] = data[pos];
                pos += 1;
            }
            writeln!(
                file,
                "glyph_spacings={}",
                glyph_spacings
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )?;

            // delta-encoded left bearings per glyph
            for g in 0..256 {
                let h = glyph_heights[g] as usize;
                if h > 0 {
                    let mut vals = Vec::with_capacity(h);
                    for _ in 0..h {
                        vals.push(data[pos] as i8);
                        pos += 1;
                    }
                    writeln!(
                        file,
                        "left_bearing_{}={}",
                        g,
                        vals.iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )?;
                }
            }

            // delta-encoded right bearings per glyph
            for g in 0..256 {
                let h = glyph_heights[g] as usize;
                if h > 0 {
                    let mut vals = Vec::with_capacity(h);
                    for _ in 0..h {
                        vals.push(data[pos] as i8);
                        pos += 1;
                    }
                    writeln!(
                        file,
                        "right_bearing_{}={}",
                        g,
                        vals.iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )?;
                }
            }
        } else {
            let space_height = data[pos];
            pos += 1;
            writeln!(file, "space_height={}", space_height)?;
        }

        // Trailing fields
        if pos + 3 < data.len() {
            let _pad1 = data[pos];
            pos += 1;
            let _pad2 = data[pos];
            pos += 1;
            let baseline = data[pos];
            pos += 1;
            let ascent = data[pos];
            writeln!(file, "baseline_adjust={}", baseline)?;
            writeln!(file, "ascent_adjust={}", ascent)?;
        }

        written += 1;
    }

    info!("  {} font metrics files", written);
    Ok(written)
}
