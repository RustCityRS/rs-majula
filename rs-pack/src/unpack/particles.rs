use anyhow::Result;
use rs_dat2::CacheStore;
use std::io::Write;
use std::path::Path;
use tracing::{info, warn};

const ARCHIVE_PARTICLES: u8 = 27;

fn g2(data: &[u8], pos: &mut usize) -> u16 {
    let v = u16::from_be_bytes([data[*pos], data[*pos + 1]]);
    *pos += 2;
    v
}

fn g4(data: &[u8], pos: &mut usize) -> u32 {
    let v = u32::from_be_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    v
}

pub fn unpack_particles(store: &mut CacheStore, content_dir: &Path) -> Result<u32> {
    let archive_dir = content_dir.join("particles").join("_unpack");
    std::fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(ARCHIVE_PARTICLES).unwrap_or(0);
    let out_path = archive_dir.join("all.particle");
    let mut file = std::fs::File::create(&out_path)?;
    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_PARTICLES, gid)? else {
            continue;
        };

        for entry in &group.entries {
            if entry.data.is_empty() {
                continue;
            }

            writeln!(file, "[particle_{}]", entry.id)?;
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
                        let s45 = g2(data, &mut pos);
                        let s42 = g2(data, &mut pos);
                        let s44 = g2(data, &mut pos);
                        let s43 = g2(data, &mut pos);
                        writeln!(
                            file,
                            "bounds={},{},{},{}",
                            s45 << 3,
                            s42 << 3,
                            s44 << 3,
                            s43 << 3
                        )?;
                    }
                    2 => {
                        pos += 1;
                    }
                    3 => {
                        let start = g4(data, &mut pos);
                        let end = g4(data, &mut pos);
                        writeln!(file, "lifetime={},{}", start, end)?;
                    }
                    4 => {
                        let a = data[pos];
                        pos += 1;
                        let b = data[pos] as i8;
                        pos += 1;
                        writeln!(file, "lifetime_mod={},{}", a, b)?;
                    }
                    5 => {
                        let v = g2(data, &mut pos);
                        writeln!(file, "position_y={}", (v as u32) << 14)?;
                    }
                    6 => {
                        let start_color = g4(data, &mut pos);
                        let end_color = g4(data, &mut pos);
                        writeln!(file, "color=0x{:08X},0x{:08X}", start_color, end_color)?;
                    }
                    7 => {
                        let a = g2(data, &mut pos);
                        let b = g2(data, &mut pos);
                        writeln!(file, "scale={},{}", a, b)?;
                    }
                    8 => {
                        let a = g2(data, &mut pos);
                        let b = g2(data, &mut pos);
                        writeln!(file, "size_bounds={},{}", a, b)?;
                    }
                    9 => {
                        let count = data[pos] as usize;
                        pos += 1;
                        let mut ids = Vec::new();
                        for _ in 0..count {
                            ids.push(g2(data, &mut pos));
                        }
                        writeln!(
                            file,
                            "model_ids={}",
                            ids.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                    10 => {
                        let count = data[pos] as usize;
                        pos += 1;
                        let mut ids = Vec::new();
                        for _ in 0..count {
                            ids.push(g2(data, &mut pos));
                        }
                        writeln!(
                            file,
                            "sprite_ids={}",
                            ids.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                    12 => {
                        let v = data[pos] as i8;
                        pos += 1;
                        writeln!(file, "size_start={}", v)?;
                    }
                    13 => {
                        let v = data[pos] as i8;
                        pos += 1;
                        writeln!(file, "size_end={}", v)?;
                    }
                    14 => {
                        writeln!(file, "spread={}", g2(data, &mut pos))?;
                    }
                    15 => {
                        writeln!(file, "spread_limit={}", g2(data, &mut pos))?;
                    }
                    16 => {
                        let sync = data[pos] == 1;
                        pos += 1;
                        let rate = g2(data, &mut pos);
                        let max = g2(data, &mut pos);
                        let continuous = data[pos] == 1;
                        pos += 1;
                        writeln!(file, "flow={},{},{},{}", sync, rate, max, continuous)?;
                    }
                    17 => {
                        writeln!(file, "duration={}", g2(data, &mut pos))?;
                    }
                    18 => {
                        writeln!(file, "tint=0x{:08X}", g4(data, &mut pos))?;
                    }
                    19 => {
                        writeln!(file, "tint_blend={}", data[pos])?;
                        pos += 1;
                    }
                    20 => {
                        writeln!(file, "size_scale={}", data[pos])?;
                        pos += 1;
                    }
                    21 => {
                        writeln!(file, "lifetime_scale_pct={}", data[pos])?;
                        pos += 1;
                    }
                    22 => {
                        writeln!(file, "scale_x_position={}", g4(data, &mut pos))?;
                    }
                    23 => {
                        writeln!(file, "size_pct={}", data[pos])?;
                        pos += 1;
                    }
                    24 => {
                        writeln!(file, "auto_orient=no")?;
                    }
                    25 => {
                        let count = data[pos] as usize;
                        pos += 1;
                        let mut ids = Vec::new();
                        for _ in 0..count {
                            ids.push(g2(data, &mut pos));
                        }
                        writeln!(
                            file,
                            "texture_ids={}",
                            ids.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                    26 => {
                        writeln!(file, "billboard=no")?;
                    }
                    27 => {
                        writeln!(file, "y_offset={}", (g2(data, &mut pos) as u32) << 14)?;
                    }
                    28 => {
                        writeln!(file, "y_size_pct={}", data[pos])?;
                        pos += 1;
                    }
                    29 => {
                        let _ = g2(data, &mut pos);
                    }
                    30 => {
                        writeln!(file, "flag_30=yes")?;
                    }
                    31 => {
                        let a = g2(data, &mut pos);
                        let b = g2(data, &mut pos);
                        writeln!(
                            file,
                            "position_range={},{}",
                            (a as u32) << 14,
                            (b as u32) << 14
                        )?;
                    }
                    32 => {
                        writeln!(file, "gravity=no")?;
                    }
                    33 => {
                        writeln!(file, "flag_33=yes")?;
                    }
                    34 => {
                        writeln!(file, "depth_test=no")?;
                    }
                    _ => {
                        warn!("particle {}: unknown opcode {}", entry.id, opcode);
                        break;
                    }
                }
            }
            written += 1;
        }
    }

    info!("  {} particle configs", written);
    Ok(written)
}
