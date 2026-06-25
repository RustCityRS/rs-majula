use std::path::Path;

use crate::unpack;
use rs_io::jag::JagFile;
use tracing::debug;

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

pub fn unpack_textures(jag: &JagFile, output_dir: &Path, pack_dir: &Path) -> anyhow::Result<()> {
    let tex_dir = output_dir.join("textures");
    std::fs::create_dir_all(&tex_dir)?;
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

    for (id_str, _, dat_data) in &index_positions {
        let id: u16 = id_str.parse().unwrap_or(u16::MAX);
        let name = texture_name(id);
        let sub_dir = tex_dir.join(&name);
        std::fs::create_dir_all(&sub_dir)?;
        unpack::decode_sprite_group(&index_data.data, dat_data, &sub_dir)?;
    }

    let index_content = index_order.join("\n") + "\n";
    std::fs::write(meta_dir.join("index.order"), &index_content)?;

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

fn texture_name(id: u16) -> String {
    TEXTURE_NAMES
        .get(id as usize)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("texture_{id}"))
}

fn find_texture_id(hash: i32) -> Option<String> {
    for id in 0..256u16 {
        if JagFile::hash(&format!("{id}.dat")) == hash {
            return Some(id.to_string());
        }
    }
    None
}
