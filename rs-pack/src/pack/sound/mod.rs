use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rs_io::Packet;
use rs_io::jag::{JagCompression, JagFile};
use tracing::debug;

use crate::pack::pack_registry::PackRegistry;
use crate::pack::util;

pub fn pack_sounds(registry: &PackRegistry, content_dir: &Path, pack_dir: &Path) -> Vec<u8> {
    let synth_dir = content_dir.join("synth");
    if !synth_dir.exists() {
        panic!("Could not find synth dir");
    }

    let order = util::load_order(&pack_dir.join("synth.order"));
    let pack = &registry.synth;

    let files = util::walk(&synth_dir);
    let mut name_to_file: HashMap<String, &PathBuf> = HashMap::new();
    for f in &files {
        if f.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("synth"))
            && let Some(stem) = f.file_stem().and_then(|s| s.to_str())
            && pack.get_by_debugname(stem).is_some()
        {
            name_to_file.insert(stem.to_string(), f);
        }
    }

    let mut out = Packet::new(name_to_file.len() * 512 + 256);
    let mut count = 0;

    for &id in &order {
        let Some(name) = pack.get_by_id(id) else {
            continue;
        };
        let Some(path) = name_to_file.get(name) else {
            continue;
        };
        let Ok(data) = std::fs::read(path) else {
            continue;
        };

        out.p2(id);
        out.pdata(&data, 0, data.len());
        count += 1;
    }

    out.p2(0xFFFF);

    let mut jag = JagFile::new();
    jag.write("sounds.dat", out.data[..out.pos].to_vec());

    debug!("Packed {} synths into sounds Jag", count);
    jag.build(JagCompression::PerFile)
}
