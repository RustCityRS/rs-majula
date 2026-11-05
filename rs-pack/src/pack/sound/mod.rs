#[cfg(since_647)]
use crate::config_blob::ConfigBlob;
#[cfg(since_647)]
use crate::pack::js5::{
    PreparedGroup, load_group_files, stamp_path, write_archive, write_empty_archive,
};
#[cfg(before_647)]
use crate::pack::pack_registry::PackRegistry;
#[cfg(before_647)]
use crate::pack::util;
#[cfg(since_647)]
use crate::startup::{is_up_to_date, newest_mtime, write_stamp};
#[cfg(since_647)]
use anyhow::Result;
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(since_647)]
use rs_dat2::CacheStore;
#[cfg(since_647)]
use rs_dat2::archive::{self, ArchiveVersions};
#[cfg(before_647)]
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::jag::{JagCompression, JagFile};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
#[cfg(before_647)]
use std::path::PathBuf;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::info;

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
    jag.build(JagCompression::PerFile)
}

#[cfg(since_647)]
pub fn pack_archive(
    store: &mut CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    client_data_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    let aid = archive::SYNTH;
    let format = archive::archive_format(aid).unwrap();
    let unpack_dir = content_dir.join(format.content_dir).join("_unpack");

    let synth_stamp = stamp_path(output_dir, aid);
    let synth_input = newest_mtime(&unpack_dir, &[]);
    if !force_rebuild && is_up_to_date(&synth_stamp, synth_input) {
        info!(
            "  archive {:>2} ({}) -- up to date, skipping",
            aid, format.content_dir
        );
        return Ok(());
    }

    let group_files = load_group_files(&unpack_dir, "");

    if group_files.is_empty() {
        write_empty_archive(store, aid, format, versions)?;
        write_stamp(&synth_stamp)?;
        return Ok(());
    }

    let mut blob = ConfigBlob::new();
    for (gid, path) in &group_files {
        let data = fs::read(path).unwrap();
        blob.add(&gid.to_string(), data);
    }
    blob.save(&client_data_dir.join("sounds"))?;

    let prepared: Vec<PreparedGroup> = group_files
        .par_iter()
        .map(|(gid, _)| {
            PreparedGroup::new(aid, *gid, blob.get(&gid.to_string()).unwrap(), versions)
        })
        .collect::<Result<Vec<_>>>()?;

    write_archive(
        store,
        aid,
        format,
        versions,
        prepared,
        "→ data/pack/client/sounds blob",
    )?;
    write_stamp(&synth_stamp)?;
    Ok(())
}
