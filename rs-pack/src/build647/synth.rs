//! Archive 4 — Synth: passthrough binary + "sounds" client blob.

use std::path::Path;

use anyhow::Result;
use rayon::prelude::*;
use tracing::info;

use rs_dat2::archive::{self, ArchiveVersions};

use super::helpers::{
    PreparedGroup, build_file_infos, load_group_files, stamp_path, touch_stamp, write_archive,
    write_empty_archive,
};

use crate::startup::{is_up_to_date, newest_mtime};

pub fn build(
    store: &mut rs_dat2::CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    client_data_dir: &Path,
    versions: &std::collections::HashMap<u8, ArchiveVersions>,
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

    let group_files = load_group_files(&unpack_dir);

    if group_files.is_empty() {
        write_empty_archive(store, aid, format, versions)?;
        touch_stamp(&synth_stamp)?;
        return Ok(());
    }

    let mut blob = crate::config_blob::ConfigBlob::new();
    for (gid, path) in &group_files {
        let data = std::fs::read(path).unwrap();
        blob.add(&gid.to_string(), data);
    }
    blob.save(&client_data_dir.join("sounds"))?;

    let prepared: Vec<PreparedGroup> = group_files
        .par_iter()
        .map(|(gid, _)| {
            let data = blob.get(&gid.to_string()).unwrap();
            let ver = versions
                .get(&aid)
                .and_then(|v| v.group_versions.get(gid))
                .copied()
                .unwrap_or(0);

            let (raw, crc) =
                rs_dat2::prepare_group(aid, *gid, data, ver as u16, format.allow_uncompressed)
                    .unwrap();

            let files =
                build_file_infos(versions, aid, *gid, rs_dat2::detect_file_count(data) as u32);

            PreparedGroup {
                id: *gid,
                raw,
                crc,
                version: ver,
                size: data.len(),
                name_hash: -1,
                whirlpool: None,
                files,
            }
        })
        .collect();

    write_archive(
        store,
        aid,
        format,
        versions,
        prepared,
        "→ data/pack/client/sounds blob",
    )?;
    touch_stamp(&synth_stamp)?;
    Ok(())
}
