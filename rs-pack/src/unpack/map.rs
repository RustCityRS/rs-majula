#[cfg(since_647)]
use crate::map_format::{
    self, MAP_ARCHIVE, MapGroupEntry, MapManifest, emit_landscape_text, emit_terrain_text,
    parse_landscape_binary,
};
#[cfg(all(since_244, before_647))]
use crate::versionlist::VersionList;
#[cfg(since_647)]
use anyhow::Result;
#[cfg(since_647)]
use rs_dat2::CacheStore;
#[cfg(since_647)]
use rs_dat2::xtea::java_string_hash;
#[cfg(before_647)]
use rs_io::Packet;
#[cfg(rev = "225")]
use rs_io::bz2::bz2_decompress;
#[cfg(all(since_244, before_647))]
use rs_io::js5::Js5Store;
#[cfg(rev = "225")]
use std::collections::{HashMap, HashSet};
use std::path::Path;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::{info, warn};

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

#[cfg(all(since_244, before_647))]
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

#[cfg(before_647)]
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

#[cfg(before_647)]
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

#[cfg(since_647)]
pub fn unpack_maps(store: &mut CacheStore, cache_dir: &Path, content_dir: &Path) -> Result<u32> {
    let map_dir = content_dir.join("maps").join("_unpack");
    std::fs::create_dir_all(&map_dir)?;

    let name_lookup = map_format::build_name_lookup();
    // XTEA map keys: prefer the committed content/<REV>/keys.json, fall back
    // to a keys.json sitting next to the raw cache (OpenRS2 download layout).
    let content_keys = Path::new(crate::CONTENT_DIR).join("keys.json");
    let cache_keys = cache_dir.join("keys.json");
    let keys_path = if content_keys.exists() {
        content_keys
    } else {
        cache_keys
    };
    let xtea_keys = if keys_path.exists() {
        map_format::load_xtea_keys(&keys_path)?
    } else {
        warn!(
            "No keys.json found at {}, landscape groups will be opaque",
            keys_path.display()
        );
        std::collections::HashMap::new()
    };

    let group_count = store.group_count(MAP_ARCHIVE).unwrap_or(0);

    let ref_table = store.reference_table(MAP_ARCHIVE);
    let ref_groups: Vec<_> = ref_table.map(|t| t.groups.clone()).unwrap_or_default();

    let mut manifest = MapManifest {
        archive_id: MAP_ARCHIVE,
        groups: Vec::new(),
    };

    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(MAP_ARCHIVE, gid)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let comp_type = raw[0];
        let compressed_len = u32::from_be_bytes([raw[1], raw[2], raw[3], raw[4]]) as usize;
        let header_size = if comp_type == 0 { 5 } else { 9 };
        let decompressed_len = if comp_type != 0 && raw.len() >= 9 {
            u32::from_be_bytes([raw[5], raw[6], raw[7], raw[8]])
        } else {
            compressed_len as u32
        };
        let container_end = header_size + compressed_len;
        let version = if raw.len() >= container_end + 2 {
            u16::from_be_bytes([raw[container_end], raw[container_end + 1]])
        } else {
            0
        };
        let crc = map_format::container_crc(&raw, comp_type);

        // Resolve the group name from the reference table's name_hash
        let name_hash = ref_groups.iter().find(|g| g.id == gid).map(|g| g.name_hash);

        let name = name_hash
            .and_then(|h| name_lookup.get(&h).cloned())
            .unwrap_or_else(|| format!("group_{}", gid));

        let is_landscape = map_format::is_landscape(&name);

        if is_landscape {
            // Landscape group — XTEA encrypted
            let group_hash = java_string_hash(&name);
            let key = xtea_keys.get(&group_hash).copied();

            // Use explicit key if available, otherwise try zero key (unencrypted)
            let effective_key = key.unwrap_or([0, 0, 0, 0]);
            match map_format::decrypt_and_decompress(&raw, &effective_key) {
                Ok(decompressed) => {
                    let locs = parse_landscape_binary(&decompressed);
                    let text = emit_landscape_text(&locs);

                    let file_name = format!("{}.jm2", name);
                    let file_path = map_dir.join(&file_name);
                    std::fs::write(&file_path, &text)?;

                    manifest.groups.push(MapGroupEntry {
                        group_id: gid,
                        name: name.clone(),
                        file_name,
                        compression: comp_type,
                        version,
                        compressed_len: compressed_len as u32,
                        decompressed_len,
                        crc,
                        encrypted: true,
                        keyed: key.is_some(),
                    });
                }
                Err(_) => {
                    // Truly encrypted with unknown key — opaque passthrough
                    let encrypted_dir = map_dir.join("encrypted");
                    std::fs::create_dir_all(&encrypted_dir)?;
                    let file_name = format!("encrypted/{}.bin", gid);
                    let file_path = map_dir.join(&file_name);
                    std::fs::write(&file_path, &raw)?;

                    manifest.groups.push(MapGroupEntry {
                        group_id: gid,
                        name: name.clone(),
                        file_name,
                        compression: comp_type,
                        version,
                        compressed_len: compressed_len as u32,
                        decompressed_len,
                        crc,
                        encrypted: true,
                        keyed: false,
                    });
                }
            }
        } else if map_format::is_npc_spawns(&name) {
            // NPC spawn group — decompress and emit text
            let decompressed = rs_dat2::compression::decompress(&raw)
                .map_err(|e| anyhow::anyhow!("decompress {} (group {}): {}", name, gid, e))?;

            // Format: series of (u16 pos_key, u8 count, count × u16 npc_id) blocks
            let text = map_format::emit_npc_spawn_text(&decompressed);

            let file_name = format!("{}.jm2", name);
            let file_path = map_dir.join(&file_name);
            std::fs::write(&file_path, &text)?;

            manifest.groups.push(MapGroupEntry {
                group_id: gid,
                name: name.clone(),
                file_name,
                compression: comp_type,
                version,
                compressed_len: compressed_len as u32,
                decompressed_len,
                crc,
                encrypted: false,
                keyed: false,
            });
        } else {
            // Terrain group (m_ or um_) — plaintext, decompress directly
            let decompressed = rs_dat2::compression::decompress(&raw)
                .map_err(|e| anyhow::anyhow!("decompress {} (group {}): {}", name, gid, e))?;

            let levels = map_format::terrain_levels(&name);
            let terrain = map_format::parse_terrain_binary_levels(&decompressed, levels);
            let text = emit_terrain_text(&terrain);

            let file_name = format!("{}.jm2", name);
            let file_path = map_dir.join(&file_name);
            std::fs::write(&file_path, &text)?;

            manifest.groups.push(MapGroupEntry {
                group_id: gid,
                name: name.clone(),
                file_name,
                compression: comp_type,
                version,
                compressed_len: compressed_len as u32,
                decompressed_len,
                crc,
                encrypted: false,
                keyed: false,
            });
        }

        written += 1;
    }

    let manifest_path = map_dir.join("manifest.json");
    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, json)?;

    info!(
        "Maps unpacked: {} groups ({} landscape keyed, {} unkeyed, {} terrain)",
        written,
        manifest
            .groups
            .iter()
            .filter(|g| g.encrypted && g.keyed)
            .count(),
        manifest
            .groups
            .iter()
            .filter(|g| g.encrypted && !g.keyed)
            .count(),
        manifest.groups.iter().filter(|g| !g.encrypted).count(),
    );

    Ok(written)
}
