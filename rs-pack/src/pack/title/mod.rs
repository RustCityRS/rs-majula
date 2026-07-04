use std::collections::HashMap;
use std::path::Path;

use crate::pack::util::media::{emit_group, read_group, read_index_order};
use rs_io::jag::{JagCompression, JagFile};
use tracing::debug;

pub fn pack_title_jag(content_dir: &Path) -> Vec<u8> {
    let title_dir = content_dir.join("title");
    let font_dir = content_dir.join("fonts");

    let jag_entry_order: Vec<String> =
        std::fs::read_to_string(title_dir.join("meta").join("title.order"))
            .expect("Missing title.order")
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

    let index_order = read_index_order(&title_dir);
    let mut index: Vec<u8> = Vec::new();
    let mut entries: HashMap<String, Vec<u8>> = HashMap::new();

    for name in &index_order {
        let font_path = font_dir.join(format!("{name}.tga"));
        let path = if font_path.is_file() {
            font_path
        } else {
            title_dir.join(format!("{name}.tga"))
        };
        let group = read_group(&path);
        let data = emit_group(&mut index, &group);
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
