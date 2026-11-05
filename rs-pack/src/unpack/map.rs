use crate::cache::map::parse_landscape_binary;
#[cfg(before_647)]
use crate::cache::map::parse_terrain_binary;
#[cfg(since_647)]
use crate::cache::map::{
    CameraPlane, LocEntry, TerrainData, parse_npc_spawn_binary, parse_terrain_binary_levels,
};
#[cfg(since_647)]
use crate::cache::map::{container_crc, decrypt_and_decompress};
#[cfg(since_647)]
use crate::pack::other::map::{is_landscape, is_npc_spawns, load_xtea_keys};
#[cfg(all(since_244, before_647))]
use crate::versionlist::VersionList;
#[cfg(since_647)]
use anyhow::Result;
#[cfg(since_647)]
use rs_dat2::CacheStore;
#[cfg(since_647)]
use rs_dat2::archive::MAPS as MAP_ARCHIVE;
#[cfg(since_647)]
use rs_dat2::xtea::java_string_hash;
#[cfg(rev = "225")]
use rs_io::bz2::bz2_decompress;
#[cfg(all(since_244, before_647))]
use rs_io::js5::Js5Store;
#[cfg(since_647)]
use serde::{Deserialize, Serialize};
#[cfg(since_647)]
use std::collections::HashMap;
#[cfg(rev = "225")]
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::{info, warn};

#[cfg(rev = "225")]
pub fn unpack_maps(maps_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let out_dir = output_dir.join("maps");
    fs::create_dir_all(&out_dir)?;

    let Ok(entries) = fs::read_dir(maps_dir) else {
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

        let data = fs::read(&path)?;
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
        fs::write(out_dir.join(&filename), &content)?;
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
    fs::create_dir_all(&out_dir)?;

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
        fs::write(out_dir.join(format!("m{map_x}_{map_z}.jm2")), &content)?;
        count += 1;
    }

    debug!("Unpacked {} map squares", count);
    Ok(())
}

#[cfg(before_647)]
fn decode_terrain(data: &[u8], lines: &mut Vec<String>) {
    for (index, tile) in parse_terrain_binary(data).tiles.iter().enumerate() {
        let mut parts = Vec::new();
        if let Some(id) = tile.overlay_id {
            let mut overlay = format!("o{id}");
            if tile.overlay_shape != 0 || tile.overlay_rotation != 0 {
                overlay.push_str(&format!(";{}", tile.overlay_shape));
                if tile.overlay_rotation != 0 {
                    overlay.push_str(&format!(";{}", tile.overlay_rotation));
                }
            }
            parts.push(overlay);
        }
        if let Some(flags) = tile.flags {
            parts.push(format!("f{flags}"));
        }
        if let Some(underlay) = tile.underlay_id {
            parts.push(format!("u{underlay}"));
        }
        if let Some(height) = tile.height.filter(|&h| h != 0) {
            parts.push(format!("h{height}"));
        }
        if !parts.is_empty() {
            lines.push(format!(
                "{} {} {}: {}",
                index >> 12,
                (index >> 6) & 0x3f,
                index & 0x3f,
                parts.join(" ")
            ));
        }
    }
}

#[cfg(before_647)]
fn decode_locs(data: &[u8], lines: &mut Vec<String>) {
    for loc in parse_landscape_binary(data) {
        let mut line = format!(
            "{} {} {}: {} {}",
            loc.level, loc.x, loc.z, loc.loc_id, loc.shape
        );
        if loc.rotation != 0 {
            line.push_str(&format!(" {}", loc.rotation));
        }
        lines.push(line);
    }
}

#[cfg(since_647)]
pub fn unpack_maps(store: &mut CacheStore, cache_dir: &Path, content_dir: &Path) -> Result<u32> {
    let map_dir = content_dir.join("maps").join("_unpack");
    fs::create_dir_all(&map_dir)?;

    let name_lookup = build_name_lookup();
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
        load_xtea_keys(&keys_path)?
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
        let crc = container_crc(&raw, comp_type);

        // Resolve the group name from the reference table's name_hash
        let name_hash = ref_groups.iter().find(|g| g.id == gid).map(|g| g.name_hash);

        let name = name_hash
            .and_then(|h| name_lookup.get(&h).cloned())
            .unwrap_or_else(|| format!("group_{}", gid));

        let is_landscape = is_landscape(&name);

        if is_landscape {
            // Landscape group — XTEA encrypted
            let group_hash = java_string_hash(&name);
            let key = xtea_keys.get(&group_hash).copied();

            // Use explicit key if available, otherwise try zero key (unencrypted)
            let effective_key = key.unwrap_or([0, 0, 0, 0]);
            match decrypt_and_decompress(&raw, &effective_key) {
                Ok(decompressed) => {
                    let locs = parse_landscape_binary(&decompressed);
                    let text = emit_landscape_text(&locs);

                    let file_name = format!("{}.jm2", name);
                    let file_path = map_dir.join(&file_name);
                    fs::write(&file_path, &text)?;

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
                    fs::create_dir_all(&encrypted_dir)?;
                    let file_name = format!("encrypted/{}.bin", gid);
                    let file_path = map_dir.join(&file_name);
                    fs::write(&file_path, &raw)?;

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
        } else if is_npc_spawns(&name) {
            // NPC spawn group — decompress and emit text
            let decompressed = rs_dat2::compression::decompress(&raw)
                .map_err(|e| anyhow::anyhow!("decompress {} (group {}): {}", name, gid, e))?;

            // Format: series of (u16 pos_key, u8 count, count × u16 npc_id) blocks
            let text = emit_npc_spawn_text(&decompressed);

            let file_name = format!("{}.jm2", name);
            let file_path = map_dir.join(&file_name);
            fs::write(&file_path, &text)?;

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

            let levels = terrain_levels(&name);
            let terrain = parse_terrain_binary_levels(&decompressed, levels);
            let text = emit_terrain_text(&terrain);

            let file_name = format!("{}.jm2", name);
            let file_path = map_dir.join(&file_name);
            fs::write(&file_path, &text)?;

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
    fs::write(&manifest_path, json)?;

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

#[derive(Debug, Serialize, Deserialize)]
#[cfg(since_647)]
pub struct MapManifest {
    pub archive_id: u8,
    pub groups: Vec<MapGroupEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg(since_647)]
pub struct MapGroupEntry {
    pub group_id: u32,
    pub name: String,
    pub file_name: String,
    pub compression: u8,
    pub version: u16,
    pub compressed_len: u32,
    pub decompressed_len: u32,
    pub crc: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub encrypted: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keyed: bool,
}

#[cfg(since_647)]
pub fn build_name_lookup() -> HashMap<i32, String> {
    let mut lookup = HashMap::new();
    for prefix in ["m", "l", "n", "um", "ul"] {
        for mx in 0..=255u32 {
            for mz in 0..=255u32 {
                let name = format!("{}{}_{}", prefix, mx, mz);
                lookup.entry(java_string_hash(&name)).or_insert(name);
            }
        }
    }
    lookup
}

#[cfg(since_647)]
pub fn terrain_levels(name: &str) -> usize {
    if name.starts_with("um") { 1 } else { 4 }
}

#[cfg(since_647)]
pub fn emit_terrain_text(terrain: &TerrainData) -> String {
    let mut out = String::new();

    if terrain.levels != 4 {
        out.push_str(&format!("levels={}\n", terrain.levels));
    }

    for (index, tile) in terrain.tiles.iter().enumerate() {
        if !tile.has_any() {
            continue;
        }

        let level = index / (64 * 64);
        let x = (index / 64) % 64;
        let z = index % 64;

        out.push_str(&level.to_string());
        out.push(' ');
        out.push_str(&x.to_string());
        out.push(' ');
        out.push_str(&z.to_string());
        out.push(':');

        if let Some(h) = tile.height {
            out.push_str(" h");
            out.push_str(&h.to_string());
        }

        if let Some(id) = tile.overlay_id {
            out.push_str(" o");
            out.push_str(&id.to_string());
            if tile.overlay_shape != 0 || tile.overlay_rotation != 0 {
                out.push(';');
                out.push_str(&tile.overlay_shape.to_string());
                if tile.overlay_rotation != 0 {
                    out.push(';');
                    out.push_str(&tile.overlay_rotation.to_string());
                }
            }
        }

        if let Some(f) = tile.flags {
            out.push_str(" f");
            out.push_str(&f.to_string());
        }

        if let Some(u) = tile.underlay_id {
            out.push_str(" u");
            out.push_str(&u.to_string());
        }

        out.push('\n');
    }

    if !terrain.env_opcode_order.is_empty() {
        let order_str: Vec<String> = terrain
            .env_opcode_order
            .iter()
            .map(|o| o.to_string())
            .collect();
        out.push_str(&format!(
            "\n==== OPCODE_ORDER {} ====\n",
            order_str.join(",")
        ));
    }

    if let Some(env) = &terrain.environment {
        out.push_str("\n==== ENV ====\n");
        if let Some(v) = env.sun_color {
            out.push_str(&format!("sun_color={}\n", v));
        }
        if let Some(v) = env.ambient {
            out.push_str(&format!("ambient={}\n", v));
        }
        if let Some(v) = env.sunlight {
            out.push_str(&format!("sunlight={}\n", v));
        }
        if let Some(v) = env.backlight {
            out.push_str(&format!("backlight={}\n", v));
        }
        if let Some(a) = env.sun_angle {
            out.push_str(&format!("sun_angle={},{},{}\n", a[0], a[1], a[2]));
        }
        if let Some(v) = env.fog_color {
            out.push_str(&format!("fog_color={}\n", v));
        }
        if let Some(v) = env.fog_depth {
            out.push_str(&format!("fog_depth={}\n", v));
        }
        if let Some(m) = env.env_map {
            out.push_str(&format!(
                "env_map={},{},{},{},{},{}\n",
                m[0], m[1], m[2], m[3], m[4], m[5]
            ));
        }
    }

    if !terrain.lights.is_empty() {
        out.push_str("\n==== LIGHTS ====\n");
        for light in &terrain.lights {
            let colors_str: Vec<String> = light.colors.iter().map(|c| c.to_string()).collect();
            out.push_str(&format!(
                "{} {} {} {}: radius={} colors={} hsl={} type={}",
                light.flags_raw,
                light.x,
                light.y,
                light.z,
                light.radius,
                colors_str.join(","),
                light.hsl,
                light.type_packed
            ));
            if let Some(r) = light.light_type_ref {
                out.push_str(&format!(" ref={}", r));
            }
            out.push('\n');
        }
    }

    if let Some(params) = &terrain.env_params {
        out.push_str("\n==== ENVPARAMS ====\n");
        out.push_str(&format!(
            "{} {} {}\n",
            params.values[0], params.values[1], params.values[2]
        ));
    }

    if let Some(sky) = &terrain.skybox {
        out.push_str("\n==== SKYBOX ====\n");
        out.push_str(&format!(
            "{} {} {} {} {}\n",
            sky.param0, sky.param1, sky.param2, sky.param3, sky.param4
        ));
    }

    if let Some(cam) = &terrain.camera {
        out.push_str("\n==== CAMERA ====\n");
        for (i, plane) in cam.planes.iter().enumerate() {
            match plane {
                CameraPlane::Clear => out.push_str(&format!("{}: clear\n", i)),
                CameraPlane::Data(cells) => {
                    let vals: Vec<String> = cells.iter().map(|c| c.to_string()).collect();
                    out.push_str(&format!("{}: data {}\n", i, vals.join(",")));
                }
                CameraPlane::CopyPrev => out.push_str(&format!("{}: copy\n", i)),
            }
        }
    }

    out
}

#[cfg(since_647)]
pub fn emit_landscape_text(locs: &[LocEntry]) -> String {
    let mut out = String::new();

    for loc in locs {
        out.push_str(&format!(
            "{} {} {}: {} {} {}\n",
            loc.level, loc.x, loc.z, loc.loc_id, loc.shape, loc.rotation
        ));
    }

    out
}

#[cfg(since_647)]
pub fn emit_npc_spawn_text(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }
    let spawns = parse_npc_spawn_binary(data);
    let mut out = String::new();
    for s in &spawns {
        out.push_str(&format!("{} {} {}: {}\n", s.level, s.x, s.z, s.npc_id));
    }
    out
}

#[cfg(all(test, since_647))]
mod tests {
    use super::*;
    use crate::cache::map::*;
    use crate::pack::other::map::*;
    const TILES_PER_MAP: usize = 4 << 12;

    #[test]
    fn terrain_text_roundtrip() {
        let text = "0 10 20: h42 o5;3;2 f1 u10\n1 0 0: h0\n2 63 63: u5\n";
        let terrain = parse_terrain_text(text);
        let binary = encode_terrain_binary(&terrain);
        let parsed = parse_terrain_binary(&binary);
        let re_emitted = emit_terrain_text(&parsed);
        assert_eq!(text, re_emitted);
    }

    #[test]
    fn terrain_binary_roundtrip() {
        let mut tiles = vec![TerrainTile::default(); TILES_PER_MAP];
        tiles[0] = TerrainTile {
            height: Some(42),
            overlay_id: Some(5),
            overlay_shape: 3,
            overlay_rotation: 2,
            flags: Some(1),
            underlay_id: Some(10),
        };
        tiles[100] = TerrainTile {
            height: None,
            overlay_id: None,
            overlay_shape: 0,
            overlay_rotation: 0,
            flags: None,
            underlay_id: Some(7),
        };

        let terrain = TerrainData {
            levels: 4,
            tiles,
            env_opcode_order: Vec::new(),
            environment: None,
            lights: Vec::new(),
            env_params: None,
            skybox: None,
            camera: None,
        };

        let binary = encode_terrain_binary(&terrain);
        let parsed = parse_terrain_binary(&binary);

        assert_eq!(parsed.tiles.len(), TILES_PER_MAP);
        assert_eq!(parsed.tiles[0].height, Some(42));
        assert_eq!(parsed.tiles[0].overlay_id, Some(5));
        assert_eq!(parsed.tiles[0].overlay_shape, 3);
        assert_eq!(parsed.tiles[0].overlay_rotation, 2);
        assert_eq!(parsed.tiles[0].flags, Some(1));
        assert_eq!(parsed.tiles[0].underlay_id, Some(10));
        assert_eq!(parsed.tiles[100].underlay_id, Some(7));
        assert_eq!(parsed.tiles[100].height, None);
    }

    #[test]
    fn landscape_text_roundtrip() {
        let text = "0 10 20: 100 5 2\n0 10 21: 100 10 0\n1 5 5: 200 0 0\n";
        let locs = parse_landscape_text(text);
        let binary = encode_landscape_binary(&locs);
        let parsed = parse_landscape_binary(&binary);
        let re_emitted = emit_landscape_text(&parsed);
        assert_eq!(text, re_emitted);
    }

    #[test]
    fn landscape_binary_roundtrip() {
        let locs = vec![
            LocEntry {
                loc_id: 5,
                level: 0,
                x: 10,
                z: 20,
                shape: 10,
                rotation: 0,
            },
            LocEntry {
                loc_id: 5,
                level: 0,
                x: 10,
                z: 21,
                shape: 22,
                rotation: 3,
            },
            LocEntry {
                loc_id: 100,
                level: 1,
                x: 0,
                z: 0,
                shape: 0,
                rotation: 1,
            },
        ];

        let binary = encode_landscape_binary(&locs);
        let parsed = parse_landscape_binary(&binary);

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].loc_id, 5);
        assert_eq!(parsed[0].x, 10);
        assert_eq!(parsed[0].z, 20);
        assert_eq!(parsed[0].shape, 10);
        assert_eq!(parsed[1].loc_id, 5);
        assert_eq!(parsed[1].z, 21);
        assert_eq!(parsed[1].shape, 22);
        assert_eq!(parsed[1].rotation, 3);
        assert_eq!(parsed[2].loc_id, 100);
        assert_eq!(parsed[2].level, 1);
    }

    #[test]
    fn smart_roundtrip() {
        for val in [0, 1, 127, 128, 255, 1000, 32766] {
            let mut buf = Vec::new();
            write_smart(&mut buf, val);
            let mut cur = buf.as_slice();
            let decoded = gsmart(&mut cur);
            assert_eq!(decoded, val, "smart roundtrip failed for {}", val);
            assert!(cur.is_empty());
        }
    }

    #[test]
    fn lsmart_roundtrip() {
        for val in [0, 1, 32766, 32767, 32768, 65534, 65535, 100_000] {
            let mut buf = Vec::new();
            write_lsmart(&mut buf, val);
            let mut cur = buf.as_slice();
            let decoded = glargesmart(&mut cur);
            assert_eq!(decoded, val, "lsmart roundtrip failed for {}", val);
            assert!(cur.is_empty());
        }
    }

    #[test]
    fn obj_spawn_text_and_binary_roundtrip() {
        let text = "0 25 23: 1351 1\n0 28 28: 1351\n2 5 6: 995 100\n";
        let spawns = parse_obj_spawn_text(text);
        assert_eq!(spawns.len(), 3);
        // count defaults to 1 when omitted
        assert_eq!(spawns[1].count, 1);
        assert_eq!(spawns[2].count, 100);

        let binary = encode_obj_spawn_binary(&spawns);
        let parsed = parse_obj_spawn_binary(&binary);
        assert_eq!(parsed, spawns);
        assert_eq!(parsed[0].obj_id, 1351);
        assert_eq!(parsed[0].x, 25);
        assert_eq!(parsed[0].z, 23);
        assert_eq!(parsed[2].level, 2);
    }
}
