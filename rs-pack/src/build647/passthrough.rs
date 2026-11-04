//! Passthrough archives — raw binary groups from _unpack/ directories.
//!
//! 18 archives use this path: they already contain the correct JS5 group
//! blobs, so we just compress and write them. No encoding step.
//!
//! `_unpack` is derived from the reference cache. Groups it lacks, taken from
//! donor caches, are authored: they sit beside it in `<kind>/donor` and win
//! over it, as `maps/donor` squares do.

use std::path::Path;

use anyhow::Result;
use rayon::prelude::*;
use tracing::info;

use rs_dat2::archive::{self, ArchiveVersions};

use super::helpers::{
    PreparedGroup, build_file_infos, load_group_files, load_name_hashes, stamp_path, touch_stamp,
    write_archive, write_empty_archive,
};

use crate::startup::{is_up_to_date, max_of, newest_mtime};

const SIMPLE_ARCHIVES: &[u8] = &[
    archive::BASES,
    archive::MODELS,
    archive::BINARY,
    archive::JINGLES,
    archive::CLIENTSCRIPT,
    archive::FONTMETRICS,
    archive::VORBIS,
    archive::MUSIC_SAMPLES,
    archive::WORLDMAP_DATA,
    archive::QUICKCHAT,
    archive::QUICKCHAT_GLOBAL,
    archive::MATERIALS,
    archive::PARTICLES,
    archive::DEFAULTS,
    archive::BILLBOARDS,
    archive::DLLS,
    archive::SHADERS,
    archive::LOADING_SCREENS,
    archive::WORLDMAP_LOCS,
];

pub fn build_passthrough_archives(
    store: &mut rs_dat2::CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    pack_dir: &Path,
    versions: &std::collections::HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    for &aid in SIMPLE_ARCHIVES {
        let format = archive::archive_format(aid).unwrap();
        let unpack_dir = content_dir.join(format.content_dir).join("_unpack");
        let donor_dir = content_dir.join(format.content_dir).join("donor");

        let arch_stamp = stamp_path(output_dir, aid);
        let arch_input = max_of(&[
            newest_mtime(&unpack_dir, &[]),
            newest_mtime(&donor_dir, &[]),
        ]);
        if !force_rebuild && is_up_to_date(&arch_stamp, arch_input) {
            info!(
                "  archive {:>2} ({}) -- up to date, skipping",
                aid, format.content_dir
            );
            continue;
        }

        let group_files =
            overlay_groups(load_group_files(&unpack_dir), load_group_files(&donor_dir));

        if group_files.is_empty() {
            write_empty_archive(store, aid, format, versions)?;
            touch_stamp(&arch_stamp)?;
            continue;
        }

        let name_hashes: std::collections::HashMap<u32, i32> = if format.named {
            let gids: Vec<u32> = group_files.iter().map(|(gid, _)| *gid).collect();
            load_name_hashes(pack_dir, aid, &gids)
        } else {
            std::collections::HashMap::new()
        };

        let prepared: Vec<PreparedGroup> = group_files
            .par_iter()
            .map(|(gid, path)| {
                let data = std::fs::read(path).unwrap();
                let ver = versions
                    .get(&aid)
                    .and_then(|v| v.group_versions.get(gid))
                    .copied()
                    .unwrap_or(0);

                let (raw, crc) =
                    rs_dat2::prepare_group(aid, *gid, &data, ver as u16, format.allow_uncompressed)
                        .unwrap();

                let file_count = rs_dat2::detect_file_count(&data);

                let name_hash = if format.named {
                    name_hashes.get(gid).copied().unwrap_or(-1)
                } else {
                    -1
                };

                let whirlpool = if format.whirlpooled {
                    Some(rs_crypto::whirlpool(&raw[..raw.len() - 2]))
                } else {
                    None
                };

                let files = build_file_infos(versions, aid, *gid, file_count as u32);

                PreparedGroup {
                    id: *gid,
                    raw,
                    crc,
                    version: ver,
                    size: data.len(),
                    name_hash,
                    whirlpool,
                    files,
                }
            })
            .collect();

        write_archive(store, aid, format, versions, prepared, "from _unpack/")?;
        touch_stamp(&arch_stamp)?;
    }
    Ok(())
}

/// `derived`'s groups with `donor`'s laid over them: a donor group replaces
/// the derived one of its id, or adds one the cache lacks. Sorted by id.
fn overlay_groups(
    derived: Vec<(u32, std::path::PathBuf)>,
    donor: Vec<(u32, std::path::PathBuf)>,
) -> Vec<(u32, std::path::PathBuf)> {
    let mut groups: std::collections::BTreeMap<_, _> = derived.into_iter().collect();
    groups.extend(donor);
    groups.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::overlay_groups;
    use std::path::PathBuf;

    #[test]
    fn a_donor_group_adds_or_replaces_by_id() {
        let derived = vec![(1, PathBuf::from("u/1.bin")), (3, PathBuf::from("u/3.bin"))];
        let donor = vec![(2, PathBuf::from("d/2.bin")), (3, PathBuf::from("d/3.bin"))];
        assert_eq!(
            overlay_groups(derived, donor),
            vec![
                (1, PathBuf::from("u/1.bin")),
                (2, PathBuf::from("d/2.bin")),
                (3, PathBuf::from("d/3.bin")),
            ]
        );
    }
}
