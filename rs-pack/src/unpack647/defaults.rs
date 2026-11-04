#![allow(clippy::single_match)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Unpack defaults archive (28) to text config format.
//!
//! Three file IDs within the archive:
//! - File 1: map defaults (TextureOp_Sub30.decode_maps_defaults)
//! - File 3: graphics defaults (VarbitTypeList.decode_graphics_defaults)
//! - File 6: wearpos defaults (WearPosDefaults)

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use tracing::info;

const ARCHIVE_DEFAULTS: u8 = 28;

pub fn unpack_defaults(store: &mut CacheStore, content_dir: &Path) -> Result<u32> {
    let archive_dir = content_dir.join("defaults").join("_unpack");
    std::fs::create_dir_all(&archive_dir)?;
    let mut written = 0u32;

    // File 1: map defaults
    if let Some(group) = store.read_group(ARCHIVE_DEFAULTS, 1)? {
        if let Some(entry) = group.entries.first() {
            let out = archive_dir.join("map_defaults.txt");
            let mut file = std::fs::File::create(&out)?;
            let data = &entry.data;
            let mut pos = 0;
            loop {
                if pos >= data.len() {
                    break;
                }
                let op = data[pos];
                pos += 1;
                if op == 0 {
                    break;
                }
                match op {
                    1 => {
                        let mut vals = Vec::new();
                        for _ in 0..6 {
                            vals.push(u16::from_be_bytes([data[pos], data[pos + 1]]));
                            pos += 2;
                        }
                        writeln!(
                            file,
                            "player_positions={}",
                            vals.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                    4 => {
                        let count = data[pos] as usize;
                        pos += 1;
                        let mut vals = Vec::new();
                        for _ in 0..count {
                            let v = u16::from_be_bytes([data[pos], data[pos + 1]]);
                            pos += 2;
                            vals.push(if v == 0xFFFF { -1i32 } else { v as i32 });
                        }
                        writeln!(
                            file,
                            "array_288={}",
                            vals.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                    5 => {
                        let count = data[pos] as usize;
                        pos += 1;
                        let mut vals = Vec::new();
                        for _ in 0..count {
                            let v = u16::from_be_bytes([data[pos], data[pos + 1]]);
                            pos += 2;
                            vals.push(if v == 0xFFFF { -1i32 } else { v as i32 });
                        }
                        writeln!(
                            file,
                            "array_116={}",
                            vals.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                    _ => {
                        break;
                    }
                }
            }
            written += 1;
        }
    }

    // File 3: graphics defaults
    if let Some(group) = store.read_group(ARCHIVE_DEFAULTS, 3)? {
        if let Some(entry) = group.entries.first() {
            let out = archive_dir.join("graphics_defaults.txt");
            let mut file = std::fs::File::create(&out)?;
            let data = &entry.data;
            let mut pos = 0;
            loop {
                if pos >= data.len() {
                    break;
                }
                let op = data[pos];
                pos += 1;
                if op == 0 {
                    break;
                }
                match op {
                    1 => {
                        let mut xs = Vec::new();
                        let mut ys = Vec::new();
                        for _ in 0..4 {
                            let x = i16::from_be_bytes([data[pos], data[pos + 1]]);
                            pos += 2;
                            let y = i16::from_be_bytes([data[pos], data[pos + 1]]);
                            pos += 2;
                            xs.push(x);
                            ys.push(y);
                        }
                        writeln!(
                            file,
                            "hitmark_x={}",
                            xs.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                        writeln!(
                            file,
                            "hitmark_y={}",
                            ys.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                    2 => {
                        let v = u16::from_be_bytes([data[pos], data[pos + 1]]);
                        pos += 2;
                        writeln!(file, "field_826={}", v)?;
                    }
                    3 => {
                        let count = data[pos];
                        pos += 1;
                        writeln!(file, "hitmark_count={}", count)?;
                    }
                    _ => {
                        break;
                    }
                }
            }
            written += 1;
        }
    }

    // File 6: tile occlusion defaults (deob name "WearPosDefaults" is an artifact)
    if let Some(group) = store.read_group(ARCHIVE_DEFAULTS, 6)? {
        if let Some(entry) = group.entries.first() {
            let out = archive_dir.join("occlusion_defaults.txt");
            let mut file = std::fs::File::create(&out)?;
            let data = &entry.data;
            let mut pos = 0;
            loop {
                if pos >= data.len() {
                    break;
                }
                let op = data[pos];
                pos += 1;
                if op == 0 {
                    break;
                }
                if op == 1 {
                    let count = data[pos] as usize;
                    pos += 1;
                    let mut solid_flags = Vec::new();
                    for _ in 0..count {
                        solid_flags.push(data[pos]);
                        pos += 1;
                    }
                    writeln!(
                        file,
                        "solid_flags={}",
                        solid_flags
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )?;
                }
            }
            written += 1;
        }
    }

    info!("  {} defaults files", written);
    Ok(written)
}
