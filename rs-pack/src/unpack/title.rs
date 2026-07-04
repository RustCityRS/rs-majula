use std::path::Path;

use crate::types::Font;
use crate::unpack;
use rs_io::jag::JagFile;
use tracing::debug;

const TITLE_NAMES: &[&str] = &[
    "index",
    "logo",
    #[cfg(before_274)]
    "p11",
    #[cfg(since_274)]
    "p11_full",
    #[cfg(before_274)]
    "p12",
    #[cfg(since_274)]
    "p12_full",
    #[cfg(before_274)]
    "b12",
    #[cfg(since_274)]
    "b12_full",
    #[cfg(before_274)]
    "q8",
    #[cfg(since_274)]
    "q8_full",
    "runes",
    "title",
    "titlebox",
    "titlebutton",
];

pub fn unpack_title(jag: &JagFile, output_dir: &Path) -> anyhow::Result<()> {
    let title_dir = output_dir.join("title");
    let font_dir = output_dir.join("fonts");
    let binary_dir = output_dir.join("binary");
    let meta_dir = title_dir.join("meta");
    std::fs::create_dir_all(&title_dir)?;
    std::fs::create_dir_all(&font_dir)?;
    std::fs::create_dir_all(&binary_dir)?;
    std::fs::create_dir_all(&meta_dir)?;

    let font_names: Vec<&str> = Font::ALL.iter().map(|f| f.name()).collect();

    let index_data = jag
        .read("index.dat")
        .ok_or_else(|| anyhow::anyhow!("Missing index.dat in title JAG"))?;

    let mut sprites: Vec<(String, Vec<u8>)> = Vec::new();

    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if let Some(name) = find_title_name(hash) {
            if name == "index" || name == "title" {
                continue;
            }
            if let Some(dat) = jag.read(&format!("{name}.dat")) {
                sprites.push((name.to_string(), dat.data));
            }
        }
    }

    let mut index_positions: Vec<(String, u16, Vec<u8>)> = sprites
        .into_iter()
        .map(|(name, data)| {
            let pos = if data.len() >= 2 {
                ((data[0] as u16) << 8) | data[1] as u16
            } else {
                0
            };
            (name, pos, data)
        })
        .collect();

    index_positions.sort_by_key(|(_, pos, _)| *pos);

    // Fonts and title sprites share the title JAG's index.dat; fonts are written
    // to fonts/, title sprites to title/. index.order (in title/) lists both.
    let mut keys = Vec::new();
    for (name, _, dat_data) in &index_positions {
        if let Some(g) = unpack::decode_group(&index_data.data, dat_data) {
            let dir = if font_names.contains(&name.as_str()) {
                &font_dir
            } else {
                &title_dir
            };
            unpack::write_group_sheet(&dir.join(format!("{name}.tga")), &g)?;
            keys.push(name.clone());
        }
    }
    unpack::write_index_order(&title_dir, &keys)?;

    if let Some(dat) = jag.read("title.dat") {
        std::fs::write(binary_dir.join("title.jpg"), &dat.data)?;
    }

    let mut title_order = Vec::new();
    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if let Some(name) = find_title_name(hash) {
            title_order.push(name.to_string());
        }
    }
    let title_order_content = title_order.join("\n") + "\n";
    std::fs::write(meta_dir.join("title.order"), &title_order_content)?;

    debug!("Unpacked title JAG ({} entries)", title_order.len());
    Ok(())
}

fn find_title_name(hash: i32) -> Option<&'static str> {
    for name in TITLE_NAMES {
        if JagFile::hash(&format!("{name}.dat")) == hash {
            return Some(name);
        }
    }
    None
}

pub(crate) fn known_hashes() -> Vec<i32> {
    TITLE_NAMES
        .iter()
        .map(|n| JagFile::hash(&format!("{n}.dat")))
        .collect()
}
