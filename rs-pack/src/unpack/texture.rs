#![allow(clippy::needless_range_loop)]
#[cfg(since_647)]
use crate::texture_format::{self, TEXTURE_ARCHIVE, TextureGroupEntry, TextureManifest};
#[cfg(before_647)]
use crate::unpack;
#[cfg(since_647)]
use anyhow::{Context, Result};
#[cfg(since_647)]
use rs_dat2::CacheStore;
#[cfg(before_647)]
use rs_io::jag::JagFile;
#[cfg(since_647)]
use std::collections::HashMap;
use std::path::Path;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::{info, warn};

#[cfg(before_647)]
pub const TEXTURE_NAMES: &[&str] = &[
    "door",
    "water",
    "wall",
    "planks",
    "elfdoor",
    "darkwood",
    "roof",
    "damage",
    "leafytree",
    "treestump",
    "leafybase",
    "mossy",
    "railings",
    "painting1",
    "painting2",
    "marble",
    "wood2",
    "fountain",
    "thatched",
    "cargonet",
    "books",
    "elfroof2",
    "elfwood",
    "mossybricks",
    "water_animated",
    "gungywater",
    "web",
    "elfroof",
    "mossydamage",
    "bamboo",
    "willowtex3",
    "lava",
    "bark",
    "mapletree",
    "yewtree",
    #[cfg(rev = "225")]
    "elfbrick",
    #[cfg(since_244)]
    "empty",
    "elfwall",
    "chainmail",
    "mummy",
    "elfpainting",
    "jungleleaf4",
    "plant",
    "jungleleaf2",
    "plant2",
    "roof2",
    "door2",
    "pebblefloor",
    "rockwall",
    "glyphs",
    "canvas",
];

#[cfg(before_647)]
pub fn unpack_textures(jag: &JagFile, output_dir: &Path, pack_dir: &Path) -> anyhow::Result<()> {
    let tex_dir = output_dir.join("textures");
    let meta_dir = tex_dir.join("meta");
    std::fs::create_dir_all(&meta_dir)?;

    let index_data = jag
        .read("index.dat")
        .ok_or_else(|| anyhow::anyhow!("Missing index.dat in textures JAG"))?;

    let mut textures: Vec<(String, Vec<u8>)> = Vec::new();

    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if hash == JagFile::hash("index.dat") {
            continue;
        }
        if let Some(id_str) = find_texture_id(hash) {
            let dat_name = format!("{id_str}.dat");
            if let Some(dat) = jag.read(&dat_name) {
                textures.push((id_str, dat.data));
            }
        }
    }

    let mut index_positions: Vec<(String, u16, Vec<u8>)> = textures
        .into_iter()
        .map(|(id_str, data)| {
            let pos = if data.len() >= 2 {
                ((data[0] as u16) << 8) | data[1] as u16
            } else {
                0
            };
            (id_str, pos, data)
        })
        .collect();

    index_positions.sort_by_key(|(_, pos, _)| *pos);

    let index_order: Vec<String> = index_positions.iter().map(|(n, _, _)| n.clone()).collect();

    let mut keys = Vec::new();
    for (id_str, _, dat_data) in &index_positions {
        if let Some(mut g) = unpack::decode_group(&index_data.data, dat_data) {
            let id: u16 = id_str.parse().unwrap_or(u16::MAX);
            let name = texture_name(id);
            g.detect_scan_cols(&name);
            unpack::write_group_sheet(&tex_dir.join(format!("{name}.tga")), &g)?;
            keys.push(id_str.clone());
        }
    }
    unpack::write_index_order(&tex_dir, &keys)?;

    let mut jag_entry_order = Vec::new();
    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if hash == JagFile::hash("index.dat") {
            jag_entry_order.push("index".to_string());
        } else if let Some(id_str) = find_texture_id(hash) {
            jag_entry_order.push(id_str);
        }
    }
    let texture_order_content = jag_entry_order.join("\n") + "\n";
    std::fs::write(meta_dir.join("texture.order"), &texture_order_content)?;

    let max_id: u16 = index_order
        .iter()
        .filter_map(|s| s.parse::<u16>().ok())
        .max()
        .unwrap_or(0);
    let mut pack_lines = Vec::new();
    for id in 0..=max_id {
        pack_lines.push(format!("{id}={}", texture_name(id)));
    }
    std::fs::write(pack_dir.join("texture.pack"), pack_lines.join("\n") + "\n")?;

    debug!("Unpacked {} textures from textures JAG", index_order.len());
    Ok(())
}

#[cfg(before_647)]
fn texture_name(id: u16) -> String {
    TEXTURE_NAMES
        .get(id as usize)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("texture_{id}"))
}

#[cfg(before_647)]
fn find_texture_id(hash: i32) -> Option<String> {
    for id in 0..256u16 {
        if JagFile::hash(&format!("{id}.dat")) == hash {
            return Some(id.to_string());
        }
    }
    None
}

#[cfg(before_647)]
pub(crate) fn known_hashes() -> Vec<i32> {
    let mut hashes = vec![JagFile::hash("index.dat")];
    hashes.extend((0..256).map(|id| JagFile::hash(&format!("{id}.dat"))));
    hashes
}

#[cfg(since_647)]
const ARCHIVE_MATERIALS: u8 = 26;

#[cfg(since_647)]
pub struct TextureUnpackStats {
    pub textures_total: u32,
    pub textures_named: u32,
    pub textures_numeric: u32,
    pub opt_files: u32,
}

#[cfg(since_647)]
pub fn unpack_named_textures(
    store: &mut CacheStore,
    content_dir: &Path,
    hash_names_path: &Path,
) -> Result<TextureUnpackStats> {
    let tex_dir = content_dir.join("textures").join("_unpack");
    let meta_dir = tex_dir.join("meta");
    std::fs::create_dir_all(&tex_dir)?;
    std::fs::create_dir_all(&meta_dir)?;

    let sprite_names = load_sprite_names(hash_names_path)?;
    let materials = load_materials(store)?;
    let sprite_dir = content_dir.join("sprites").join("_unpack");

    let group_count = store.group_count(TEXTURE_ARCHIVE).unwrap_or(0);
    let mut stats = TextureUnpackStats {
        textures_total: 0,
        textures_named: 0,
        textures_numeric: 0,
        opt_files: 0,
    };

    for group_id in 0..group_count {
        let Some(group) = store.read_group(TEXTURE_ARCHIVE, group_id)? else {
            continue;
        };
        let Some(entry) = group.entries.first() else {
            continue;
        };
        if entry.data.is_empty() {
            continue;
        }

        stats.textures_total += 1;

        let tex_def = texture_format::parse_texture(&entry.data);
        let primary_sprite = tex_def.primary_sprite();

        let tex_name = if let Some(sprite_id) = primary_sprite {
            if let Some(name) = sprite_names.get(&(sprite_id as u32)) {
                if !name.is_empty() {
                    name.clone()
                } else {
                    format!("{}", group_id)
                }
            } else {
                format!("{}", group_id)
            }
        } else {
            format!("{}", group_id)
        };

        let is_named = !tex_name.chars().next().is_none_or(|c| c.is_ascii_digit());
        if is_named {
            stats.textures_named += 1;
        } else {
            stats.textures_numeric += 1;
        }

        if let Some(sprite_id) = primary_sprite {
            let src_png = sprite_dir.join(format!("{}.png", sprite_id));
            let dst_png = tex_dir.join(format!("{}.png", tex_name));
            if src_png.exists() {
                std::fs::copy(&src_png, &dst_png).with_context(|| {
                    format!("copy {} → {}", src_png.display(), dst_png.display())
                })?;
            } else {
                let src_sub = sprite_dir.join(format!("{}/0.png", sprite_id));
                if src_sub.exists() {
                    std::fs::copy(&src_sub, &dst_png)?;
                }
            }
        }

        if let Some(mat) = materials.get(&group_id) {
            let opt_path = meta_dir.join(format!("{}.opt", tex_name));
            let repeat_mode = match (mat.repeat_s, mat.repeat_t) {
                (true, true) => "both",
                (true, false) => "col",
                (false, true) => "row",
                (false, false) => "none",
            };
            let opt_content = format!(
                "{},{},{},{},{}\n",
                primary_sprite.unwrap_or(0),
                mat.speed_u,
                mat.speed_v,
                mat.averagecolor,
                repeat_mode,
            );
            std::fs::write(&opt_path, opt_content)?;
            stats.opt_files += 1;
        }
    }

    info!(
        "Named textures: {} total, {} named, {} numeric, {} .opt files",
        stats.textures_total, stats.textures_named, stats.textures_numeric, stats.opt_files
    );

    Ok(stats)
}

#[cfg(since_647)]
pub fn unpack_texture_text(store: &mut CacheStore, content_dir: &Path) -> Result<u32> {
    let tex_dir = content_dir.join("textures").join("_unpack");
    std::fs::create_dir_all(&tex_dir)?;

    let sprite_dir = content_dir.join("sprites").join("_unpack");

    let group_count = store.group_count(TEXTURE_ARCHIVE).unwrap_or(0);
    let mut manifest = TextureManifest {
        archive_id: TEXTURE_ARCHIVE,
        groups: Vec::new(),
    };
    let mut written = 0u32;
    let mut sprite_pngs = 0u32;

    for group_id in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(TEXTURE_ARCHIVE, group_id)? else {
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
        let crc = rs_io::crc::getcrc(&raw, 0, container_end.min(raw.len())) as u32;

        let Some(group) = store
            .read_group(TEXTURE_ARCHIVE, group_id)
            .with_context(|| format!("read texture group {}", group_id))?
        else {
            continue;
        };
        let Some(entry) = group.entries.first() else {
            continue;
        };
        if entry.data.is_empty() {
            continue;
        }

        let def = texture_format::parse_texture(&entry.data);
        let text = texture_format::emit_texture_text(&def);

        let file_name = format!("{}.texture", group_id);
        std::fs::write(tex_dir.join(&file_name), &text)?;

        if let Some(sprite_id) = def.primary_sprite() {
            let src = sprite_dir.join(format!("{}.png", sprite_id));
            let dst = tex_dir.join(format!("{}.png", group_id));
            if src.exists() {
                std::fs::copy(&src, &dst)?;
                sprite_pngs += 1;
            } else {
                let src_sub = sprite_dir.join(format!("{}/0.png", sprite_id));
                if src_sub.exists() {
                    std::fs::copy(&src_sub, &dst)?;
                    sprite_pngs += 1;
                }
            }
        }

        manifest.groups.push(TextureGroupEntry {
            group_id,
            file_name,
            compression: comp_type,
            version,
            compressed_len: compressed_len as u32,
            decompressed_len,
            crc,
        });
        written += 1;
    }

    let manifest_path = tex_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    info!(
        "Textures unpacked: {} groups, {} sprite PNGs copied",
        written, sprite_pngs
    );

    Ok(written)
}

#[cfg(since_647)]
fn load_sprite_names(hash_names_path: &Path) -> Result<HashMap<u32, String>> {
    let mut names = HashMap::new();
    if !hash_names_path.exists() {
        warn!("No hash names file at {}", hash_names_path.display());
        return Ok(names);
    }
    let content = std::fs::read_to_string(hash_names_path)?;
    for line in content.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }
        let archive: u8 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if archive != 8 {
            continue;
        }
        let file: i32 = match parts[2].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if file != -1 {
            continue;
        } // group-level names only
        let group: u32 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let name = parts[4].to_string();
        if name.contains(',') {
            if let Some(base) = name.split(',').next() {
                names.insert(group, base.to_string());
            }
        } else {
            names.insert(group, name);
        }
    }
    Ok(names)
}

#[cfg(since_647)]
struct MaterialInfo {
    averagecolor: u16,
    speed_u: i8,
    speed_v: i8,
    repeat_s: bool,
    repeat_t: bool,
}

#[cfg(since_647)]
fn load_materials(store: &mut CacheStore) -> Result<HashMap<u32, MaterialInfo>> {
    let mut materials = HashMap::new();
    let Some(group) = store.read_group(ARCHIVE_MATERIALS, 0)? else {
        return Ok(materials);
    };
    let Some(entry) = group.entries.first() else {
        return Ok(materials);
    };
    let data = &entry.data;
    if data.len() < 2 {
        return Ok(materials);
    }
    let mut pos = 0;

    let texture_count = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;

    let mut has_material = vec![false; texture_count];
    for i in 0..texture_count {
        has_material[i] = data.get(pos).copied().unwrap_or(0) == 1;
        pos += 1;
    }

    macro_rules! skip_pass {
        ($size:expr) => {
            for i in 0..texture_count {
                if has_material[i] {
                    pos += $size;
                }
            }
        };
    }

    skip_pass!(1); // highdetail
    skip_pass!(1); // is_lowmem
    skip_pass!(1); // lowdetail
    skip_pass!(1); // brightness
    skip_pass!(1); // intensity
    skip_pass!(1); // effect
    skip_pass!(1); // effect_param1

    // averageColor (u16)
    let mut average_color = vec![0u16; texture_count];
    for i in 0..texture_count {
        if has_material[i] {
            average_color[i] = u16::from_be_bytes([
                data.get(pos).copied().unwrap_or(0),
                data.get(pos + 1).copied().unwrap_or(0),
            ]);
            pos += 2;
        }
    }

    // speed_u (i8)
    let mut speed_u = vec![0i8; texture_count];
    for i in 0..texture_count {
        if has_material[i] {
            speed_u[i] = data.get(pos).copied().unwrap_or(0) as i8;
            pos += 1;
        }
    }

    // speed_v (i8)
    let mut speed_v = vec![0i8; texture_count];
    for i in 0..texture_count {
        if has_material[i] {
            speed_v[i] = data.get(pos).copied().unwrap_or(0) as i8;
            pos += 1;
        }
    }

    skip_pass!(1); // flag_94
    skip_pass!(1); // flag_95
    skip_pass!(1); // mipmapping

    // repeat_s
    let mut repeat_s = vec![false; texture_count];
    for i in 0..texture_count {
        if has_material[i] {
            repeat_s[i] = data.get(pos).copied().unwrap_or(0) == 1;
            pos += 1;
        }
    }

    // repeat_t
    let mut repeat_t = vec![false; texture_count];
    for i in 0..texture_count {
        if has_material[i] {
            repeat_t[i] = data.get(pos).copied().unwrap_or(0) == 1;
            pos += 1;
        }
    }

    for i in 0..texture_count {
        if has_material[i] {
            materials.insert(
                i as u32,
                MaterialInfo {
                    averagecolor: average_color[i],
                    speed_u: speed_u[i],
                    speed_v: speed_v[i],
                    repeat_s: repeat_s[i],
                    repeat_t: repeat_t[i],
                },
            );
        }
    }

    Ok(materials)
}
