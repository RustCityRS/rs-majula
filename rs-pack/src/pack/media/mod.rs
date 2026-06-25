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

    let meta_dir = sprite_dir.join("meta");

    let index_order: Vec<String> = std::fs::read_to_string(meta_dir.join("index.order"))
        .expect("Missing index.order")
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let jag_entry_order: Vec<String> = std::fs::read_to_string(meta_dir.join("sprite.order"))
        .expect("Missing sprite.order")
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let mut index: Vec<u8> = Vec::new();
    let mut all: Vec<(String, Vec<u8>)> = Vec::new();

    for name in &index_order {
        let sub_dir = sprite_dir.join(name);
        if !sub_dir.is_dir() {
            continue;
        }
        let data = media::convert_image(&mut index, &sub_dir);
        all.push((name.clone(), data));
    }

    if all.is_empty() {
        return Vec::new();
    }

    let mut dat_map: HashMap<String, Vec<u8>> = all.into_iter().collect();
    let mut jag = JagFile::new();

    if jag_entry_order.is_empty() {
        jag.write("index.dat", index);
        for name in &index_order {
            if let Some(dat) = dat_map.remove(name.as_str()) {
                jag.write(&format!("{name}.dat"), dat);
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

    debug!("Packed {} sprite groups into media Jag", dat_map.len());
    jag.build(JagCompression::PerFile)
}
