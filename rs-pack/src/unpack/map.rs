#[cfg(rev = "225")]
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[cfg(since_244)]
use crate::versionlist::VersionList;
use rs_io::Packet;
#[cfg(rev = "225")]
use rs_io::bz2::bz2_decompress;
#[cfg(since_244)]
use rs_io::js5::Js5Store;
use tracing::debug;

#[cfg(rev = "225")]
pub fn unpack_maps(maps_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let out_dir = output_dir.join("maps");
    std::fs::create_dir_all(&out_dir)?;

    let Ok(entries) = std::fs::read_dir(maps_dir) else {
        return Ok(());
    };

    let mut files: Vec<_> = entries.flatten().collect();
    files.sort_by_key(|e| e.file_name());

    let mut terrain_files: Vec<(u8, u8, Vec<u8>)> = Vec::new();
    let mut loc_files: Vec<(u8, u8, Vec<u8>)> = Vec::new();

    for entry in files {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let prefix = name.chars().next().unwrap_or(' ');
        let rest = &name[1..];
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() != 2 {
            continue;
        }
        let Ok(map_x) = parts[0].parse::<u8>() else {
            continue;
        };
        let Ok(map_z) = parts[1].parse::<u8>() else {
            continue;
        };

        let data = std::fs::read(&path)?;
        if data.len() < 4 {
            continue;
        }

        let uncompressed_size = ((data[0] as u32) << 24)
            | ((data[1] as u32) << 16)
            | ((data[2] as u32) << 8)
            | (data[3] as u32);

        let decompressed = bz2_decompress(&data[4..], uncompressed_size as usize, true, 0);

        match prefix {
            'm' => terrain_files.push((map_x, map_z, decompressed)),
            'l' => loc_files.push((map_x, map_z, decompressed)),
            _ => continue,
        }
    }

    let mut terrain_map: HashMap<(u8, u8), Vec<u8>> = HashMap::new();
    for (x, z, data) in &terrain_files {
        terrain_map.insert((*x, *z), data.clone());
    }
    let mut loc_map: HashMap<(u8, u8), Vec<u8>> = HashMap::new();
    for (x, z, data) in &loc_files {
        loc_map.insert((*x, *z), data.clone());
    }

    let mut all_squares: HashSet<(u8, u8)> = HashSet::new();
    for (x, z, _) in &terrain_files {
        all_squares.insert((*x, *z));
    }

    let mut count = 0;
    let mut squares: Vec<(u8, u8)> = all_squares.into_iter().collect();
    squares.sort();

    for (map_x, map_z) in squares {
        let mut lines = Vec::new();

        if let Some(terrain_data) = terrain_map.get(&(map_x, map_z)) {
            lines.push("==== MAP ====".to_string());
            decode_terrain(terrain_data, &mut lines);
        }

        if let Some(loc_data) = loc_map.get(&(map_x, map_z)) {
            lines.push(String::new());
            lines.push("==== LOC ====".to_string());
            decode_locs(loc_data, &mut lines);
        }

        lines.push(String::new());
        lines.push("==== NPC ====".to_string());

        lines.push(String::new());
        lines.push("==== OBJ ====".to_string());

        let filename = format!("m{map_x}_{map_z}.jm2");
        let content = lines.join("\n") + "\n";
        std::fs::write(out_dir.join(&filename), &content)?;
        count += 1;
    }

    debug!("Unpacked {} map squares", count);
    Ok(())
}

#[cfg(since_244)]
pub fn unpack_maps(
    cache: &Js5Store,
    version_list: &VersionList,
    output_dir: &Path,
) -> anyhow::Result<()> {
    let out_dir = output_dir.join("maps");
    std::fs::create_dir_all(&out_dir)?;

    let mut count = 0;
    for entry in &version_list.maps {
        let (map_x, map_z) = (entry.map_x(), entry.map_z());
        let mut lines = Vec::new();

        if let Some(land) = cache.read(4, entry.land_file as usize, true) {
            lines.push("==== MAP ====".to_string());
            decode_terrain(&land, &mut lines);
        }

        if let Some(loc) = cache.read(4, entry.loc_file as usize, true) {
            lines.push(String::new());
            lines.push("==== LOC ====".to_string());
            decode_locs(&loc, &mut lines);
        }

        lines.push(String::new());
        lines.push("==== NPC ====".to_string());

        lines.push(String::new());
        lines.push("==== OBJ ====".to_string());

        let content = lines.join("\n") + "\n";
        std::fs::write(out_dir.join(format!("m{map_x}_{map_z}.jm2")), &content)?;
        count += 1;
    }

    debug!("Unpacked {} map squares", count);
    Ok(())
}

fn decode_terrain(data: &[u8], lines: &mut Vec<String>) {
    let mut buf = Packet::from(data.to_vec());

    for y in 0..4 {
        for x in 0..64 {
            for z in 0..64 {
                let mut parts = Vec::new();

                loop {
                    if buf.pos >= buf.data.len() {
                        return;
                    }
                    let opcode = buf.g1();

                    if opcode == 0 {
                        break;
                    } else if opcode == 1 {
                        let h = buf.g1();
                        if h != 0 {
                            parts.push(format!("h{h}"));
                        }
                        break;
                    } else if opcode <= 49 {
                        let overlay_id = buf.g1();
                        let adjusted = opcode as i32 - 2;
                        let shape = adjusted >> 2;
                        let rotation = adjusted & 0x3;
                        let mut overlay_str = format!("o{overlay_id}");
                        if shape != 0 || rotation != 0 {
                            overlay_str.push_str(&format!(";{shape}"));
                            if rotation != 0 {
                                overlay_str.push_str(&format!(";{rotation}"));
                            }
                        }
                        parts.push(overlay_str);
                    } else if opcode <= 81 {
                        let flags = opcode as i32 - 49;
                        parts.push(format!("f{flags}"));
                    } else {
                        let underlay = opcode as i32 - 81;
                        parts.push(format!("u{underlay}"));
                    }
                }

                if !parts.is_empty() {
                    lines.push(format!("{y} {x} {z}: {}", parts.join(" ")));
                }
            }
        }
    }
}

fn decode_locs(data: &[u8], lines: &mut Vec<String>) {
    let mut buf = Packet::from(data.to_vec());
    let mut loc_id: i32 = -1;

    loop {
        let delta = buf.gsmart1or2();
        if delta == 0 {
            break;
        }
        loc_id += delta;

        let mut last_pos: i32 = 0;
        loop {
            let pos_delta = buf.gsmart1or2();
            if pos_delta == 0 {
                break;
            }
            last_pos += pos_delta - 1;

            let y = (last_pos >> 12) & 0x3;
            let x = (last_pos >> 6) & 0x3F;
            let z = last_pos & 0x3F;

            let loc_info = buf.g1();
            let shape = loc_info >> 2;
            let angle = loc_info & 0x3;

            if angle != 0 {
                lines.push(format!("{y} {x} {z}: {loc_id} {shape} {angle}"));
            } else {
                lines.push(format!("{y} {x} {z}: {loc_id} {shape}"));
            }
        }
    }
}
