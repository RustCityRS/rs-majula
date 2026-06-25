use std::collections::HashMap;
use std::path::Path;

use crate::pack::util::media::convert_image;
use rs_io::jag::{JagCompression, JagFile};
use tracing::debug;

pub fn pack_title_jag(content_dir: &Path) -> Vec<u8> {
    let title_dir = content_dir.join("title");
    let font_dir = content_dir.join("fonts");
    let meta_dir = title_dir.join("meta");

    let index_order: Vec<String> = std::fs::read_to_string(meta_dir.join("index.order"))
        .expect("Missing title index.order")
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let jag_entry_order: Vec<String> = std::fs::read_to_string(meta_dir.join("title.order"))
        .expect("Missing title.order")
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let mut index: Vec<u8> = Vec::new();
    let mut entries: HashMap<String, Vec<u8>> = HashMap::new();

    for name in &index_order {
        let sprite_dir = font_dir.join(name);
        let sprite_dir = if sprite_dir.is_dir() {
            sprite_dir
        } else {
            title_dir.join(name)
        };
        if !sprite_dir.is_dir() {
            continue;
        }
        let data = convert_image(&mut index, &sprite_dir);
        entries.insert(name.clone(), data);
    }

    let mut title_jpg = content_dir
        .join("binary")
        .join("title.jpg")
        .exists()
        .then(|| std::fs::read(content_dir.join("binary").join("title.jpg")).ok())
        .flatten();

    let mut jag = JagFile::new();

    for name in &jag_entry_order {
        if name == "index" {
            jag.write("index.dat", std::mem::take(&mut index));
        } else if name == "title" {
            if let Some(data) = title_jpg.take() {
                jag.write("title.dat", data);
            }
        } else if let Some(dat) = entries.remove(name.as_str()) {
            jag.write(&format!("{name}.dat"), dat);
        }
    }

    debug!("Packed title Jag ({} sprites + title.jpg)", entries.len());
    jag.build(JagCompression::PerFile)
}
