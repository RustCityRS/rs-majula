#![allow(clippy::single_match)]
use crate::archive;
use anyhow::Result;
use rs_io::js5::Js5Store;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tracing::info;

pub fn unpack_defaults(store: &mut Js5Store, content_dir: &Path) -> Result<u32> {
    let archive_dir = content_dir.join("defaults").join("_unpack");
    fs::create_dir_all(&archive_dir)?;
    let mut written = 0u32;

    // File 1: map defaults
    if let Some(group) = store.read_group(archive::DEFAULTS, 1)? {
        if let Some(entry) = group.entries.first() {
            let out = archive_dir.join("map_defaults.txt");
            let mut file = File::create(&out)?;
            let data = entry.data();
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
    if let Some(group) = store.read_group(archive::DEFAULTS, 3)? {
        if let Some(entry) = group.entries.first() {
            let out = archive_dir.join("graphics_defaults.txt");
            let mut file = File::create(&out)?;
            let data = entry.data();
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
    if let Some(group) = store.read_group(archive::DEFAULTS, 6)? {
        if let Some(entry) = group.entries.first() {
            let out = archive_dir.join("occlusion_defaults.txt");
            let mut file = File::create(&out)?;
            let data = entry.data();
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
