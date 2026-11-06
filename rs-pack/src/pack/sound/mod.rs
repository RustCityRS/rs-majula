#[cfg(before_647)]
use crate::archive::JagArchive;
#[cfg(since_647)]
use crate::archive::{self, ArchiveVersions};
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, insert_archive, insert_empty_archive, load_group_files};
#[cfg(before_647)]
use crate::pack::pack_registry::PackRegistry;
#[cfg(before_647)]
use crate::pack::util;
#[cfg(since_647)]
use anyhow::Result;
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(before_647)]
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::jag::JagFile;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
#[cfg(before_647)]
use std::path::PathBuf;
#[cfg(before_647)]
use tracing::debug;

#[cfg(before_647)]
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
        let Ok(data) = fs::read(path) else {
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
    jag.build(JagArchive::Sounds.compression())
}

#[cfg(since_647)]
pub fn pack_archive(
    store: &mut PackedCache,
    content_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    let aid = archive::SYNTH;
    let format = archive::archive_format(aid).unwrap();
    let unpack_dir = content_dir.join(format.content_dir).join("_unpack");

    let group_files = load_group_files(&unpack_dir, "");

    if group_files.is_empty() {
        insert_empty_archive(store, aid, format, versions)?;

        return Ok(());
    }

    let prepared: Vec<PreparedGroup> = group_files
        .par_iter()
        .map(|(gid, path)| PreparedGroup::new(aid, *gid, &fs::read(path)?, versions))
        .collect::<Result<Vec<_>>>()?;

    insert_archive(store, aid, format, versions, prepared, "from synths")?;

    Ok(())
}
