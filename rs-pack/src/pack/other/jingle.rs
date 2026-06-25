use std::collections::HashMap;
use std::path::Path;

use rs_io::bz2::bz2_compress_with_size;
use tracing::debug;

pub fn pack_jingles(content_dir: &Path) -> HashMap<String, Vec<u8>> {
    let jingles_dir = content_dir.join("jingles");
    let mut result = HashMap::new();

    if !jingles_dir.exists() {
        return result;
    }

    let Ok(entries) = std::fs::read_dir(&jingles_dir) else {
        return result;
    };

    let mut files: Vec<_> = entries.flatten().collect();
    files.sort_by_key(|e| e.file_name());

    for entry in files {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mid"))
        {
            if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                if result.contains_key(name) {
                    continue;
                }
                if let Ok(data) = std::fs::read(&path) {
                    result.insert(name.to_string(), bz2_compress_with_size(&data));
                }
            }
        }
    }

    debug!("Packed {} jingles", result.len());
    result
}
