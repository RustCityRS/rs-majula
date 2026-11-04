//! Unpack billboard configs (archive 29) to text config format.
//!
//! Opcode-based format from `BillboardRaw.java`:
//! Op 1: material (u16, 0xFFFF = -1)
//! Op 2: xan (u16+1), yan (u16+1)
//! Op 3: padding (u8)
//! Op 4: blend_src (u8)
//! Op 5: blend_dst (u8)
//! Op 6: bloom_disable = true
//! Op 7: skip_triangle = true

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use tracing::{info, warn};

const ARCHIVE_BILLBOARDS: u8 = 29;

pub fn unpack_billboards(store: &mut CacheStore, content_dir: &Path) -> Result<u32> {
    let archive_dir = content_dir.join("billboards").join("_unpack");
    std::fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(ARCHIVE_BILLBOARDS).unwrap_or(0);
    let out_path = archive_dir.join("all.billboard");
    let mut file = std::fs::File::create(&out_path)?;
    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_BILLBOARDS, gid)? else {
            continue;
        };

        for entry in &group.entries {
            if entry.data.is_empty() {
                continue;
            }

            writeln!(file, "[billboard_{}]", entry.id)?;
            let data = &entry.data;
            let mut pos = 0;

            loop {
                if pos >= data.len() {
                    break;
                }
                let opcode = data[pos];
                pos += 1;
                if opcode == 0 {
                    break;
                }

                match opcode {
                    1 => {
                        let v = u16::from_be_bytes([data[pos], data[pos + 1]]);
                        pos += 2;
                        if v != 0xFFFF {
                            writeln!(file, "material={}", v)?;
                        }
                    }
                    2 => {
                        let xan = u16::from_be_bytes([data[pos], data[pos + 1]]) + 1;
                        pos += 2;
                        let yan = u16::from_be_bytes([data[pos], data[pos + 1]]) + 1;
                        pos += 2;
                        if xan != 64 {
                            writeln!(file, "xan={}", xan)?;
                        }
                        if yan != 64 {
                            writeln!(file, "yan={}", yan)?;
                        }
                    }
                    3 => {
                        pos += 1;
                    }
                    4 => {
                        let v = data[pos];
                        pos += 1;
                        writeln!(file, "blend_src={}", v)?;
                    }
                    5 => {
                        let v = data[pos];
                        pos += 1;
                        writeln!(file, "blend_dst={}", v)?;
                    }
                    6 => {
                        writeln!(file, "bloom_disable=yes")?;
                    }
                    7 => {
                        writeln!(file, "skip_triangle=yes")?;
                    }
                    _ => {
                        warn!("billboard {}: unknown opcode {}", entry.id, opcode);
                        break;
                    }
                }
            }
            written += 1;
        }
    }

    info!("  {} billboard configs", written);
    Ok(written)
}
