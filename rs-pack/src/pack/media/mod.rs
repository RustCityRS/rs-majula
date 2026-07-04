use std::collections::HashMap;
use std::path::Path;

use crate::pack::util::media;
use rs_io::jag::{JagCompression, JagFile};
use tracing::debug;

pub fn pack_media_jag(content_dir: &Path) -> Vec<u8> {
    let sprite_dir = content_dir.join("sprites");
    if !sprite_dir.exists() {
        panic!("Could not find sprites dir");
    }

    let jag_entry_order: Vec<String> =
        std::fs::read_to_string(sprite_dir.join("meta").join("sprite.order"))
            .expect("Missing sprite.order")
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

    let index_order = media::read_index_order(&sprite_dir);
    let mut index: Vec<u8> = Vec::new();
    let mut dat_map: HashMap<String, Vec<u8>> = HashMap::new();

    for name in &index_order {
        let group = media::read_group(&sprite_dir.join(format!("{name}.tga")));
        let data = media::emit_group(&mut index, &group);
        dat_map.insert(name.clone(), data);
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

    debug!("Packed sprite groups into media Jag");
    jag.build(JagCompression::PerFile)
}
