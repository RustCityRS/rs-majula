use std::collections::HashMap;
use std::path::Path;

use super::pack_registry::PackRegistry;
use crate::pack::util::media;
use rs_io::jag::{JagCompression, JagFile};
use tracing::debug;

pub fn pack_textures_jag(registry: &PackRegistry, content_dir: &Path) -> Vec<u8> {
    let tex_dir = content_dir.join("textures");
    if !tex_dir.exists() {
        panic!("Could not find textures dir");
    }

    let meta_dir = tex_dir.join("meta");
    let pack = &registry.texture;

    let index_order: Vec<String> = std::fs::read_to_string(meta_dir.join("index.order"))
        .expect("Missing index.order")
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let jag_entry_order: Vec<String> = std::fs::read_to_string(meta_dir.join("texture.order"))
        .expect("Missing texture.order")
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let mut index: Vec<u8> = Vec::new();
    let mut all: Vec<(String, Vec<u8>)> = Vec::new();

    for id_str in &index_order {
        let name = pack
            .get_by_id(id_str.parse().unwrap_or(u16::MAX))
            .unwrap_or(id_str.as_str());

        let sub_dir = tex_dir.join(name);
        if !sub_dir.is_dir() {
            continue;
        }

        let data = media::convert_image(&mut index, &sub_dir);
        all.push((id_str.clone(), data));
    }

    if all.is_empty() {
        return Vec::new();
    }

    let mut dat_map: HashMap<String, Vec<u8>> = all.into_iter().collect();
    let mut jag = JagFile::new();

    if jag_entry_order.is_empty() {
        jag.write("index.dat", index);
        for id_str in &index_order {
            if let Some(dat) = dat_map.remove(id_str.as_str()) {
                jag.write(&format!("{id_str}.dat"), dat);
            }
        }
    } else {
        for entry in &jag_entry_order {
            if entry == "index" {
                jag.write("index.dat", std::mem::take(&mut index));
            } else if let Some(dat) = dat_map.remove(entry.as_str()) {
                jag.write(&format!("{entry}.dat"), dat);
            }
        }
    }

    debug!("Packed {} textures into textures Jag", dat_map.len());
    jag.build(JagCompression::PerFile)
}
