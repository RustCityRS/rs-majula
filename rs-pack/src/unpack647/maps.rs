//! Map archive unpacker — cache archive 5 → text `.map` files.
//!
//! Terrain groups (`m_X_Z`) are decompressed directly. Landscape groups
//! (`l_X_Z`) are XTEA-decrypted then decompressed. Both are parsed to
//! a text format matching 2004scape conventions, one file per group.
//! Groups without a known XTEA key are stored as opaque encrypted binary.

use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_dat2::xtea::java_string_hash;
use tracing::{info, warn};

use crate::map_format::{
    self, MAP_ARCHIVE, MapGroupEntry, MapManifest, emit_landscape_text, emit_terrain_text,
    parse_landscape_binary,
};

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
