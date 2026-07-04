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

    let pack = &registry.texture;

    let jag_entry_order: Vec<String> =
        std::fs::read_to_string(tex_dir.join("meta").join("texture.order"))
            .expect("Missing texture.order")
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

    let index_order = media::read_index_order(&tex_dir);
    let mut index: Vec<u8> = Vec::new();
    let mut dat_map: HashMap<String, Vec<u8>> = HashMap::new();

    for id_str in &index_order {
        let name = pack
            .get_by_id(id_str.parse().unwrap_or(u16::MAX))
            .unwrap_or(id_str.as_str());
        let group = media::read_group(&tex_dir.join(format!("{name}.tga")));
        let data = media::emit_group(&mut index, &group);
        dat_map.insert(id_str.clone(), data);
    }

    if dat_map.is_empty() {
        return Vec::new();
    }

    let mut jag = JagFile::new();
    for entry in &jag_entry_order {
        if entry == "index" {
            jag.write("index.dat", std::mem::take(&mut index));
        } else if let Some(dat) = dat_map.remove(entry.as_str()) {
            jag.write(&format!("{entry}.dat"), dat);
        }
    }

    debug!("Packed textures into textures Jag");
    jag.build(JagCompression::PerFile)
}
