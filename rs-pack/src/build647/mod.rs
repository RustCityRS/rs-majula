#![allow(clippy::type_complexity)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Build a complete JS5 cache from source.
//!
//! Modules:
//! - `helpers` — shared types (PreparedGroup, write_archive, stamp files)
//! - `sprites_encode` — PNG → Jagex sprite binary encoder

mod helpers;
mod interfaces;
mod maps;
mod passthrough;
mod songs;
mod sprites;
mod sprites_encode;
mod synth;
mod textures;
mod verify;

pub use sprites_encode::encode_sprite_group_from_dir;

use crate::config_blob::ConfigBlob;
use crate::pack::model;
use crate::pack::pack_registry::PackFile;
use crate::startup::{
    config_inputs, current_exe_mtime, file_mtime, is_up_to_date, max_of, newest_mtime,
};

use crate::cache::seq_frame::split_frame_into_stripes;
use anyhow::Result;
use tracing::{info, warn};

#[allow(unused_imports)]
use helpers::{
    PreparedGroup, build_file_infos, group_version, load_group_files, load_name_hashes, stamp_path,
    touch_stamp, write_archive, write_empty_archive,
};

/// Newest modification time among every source the JS5 build reads. The
/// build is skipped wholesale while its stamp is newer than this, so a source
/// missing here is a source whose edits silently never ship.
///
/// Landscapes, terrains and spawns are `.jm2` (what `build647/maps.rs` and the
/// unpacker read and write), in any of `map_format::source_dirs`; the 63
/// squares nobody can decrypt are opaque `encrypted/<gid>.bin`.
pub fn cache_build_inputs(content_dir: &std::path::Path) -> Option<std::time::SystemTime> {
    max_of(&[
        config_inputs(&content_dir.join("scripts"), &content_dir.join("pack")),
        newest_mtime(&content_dir.join("scripts"), &["rs2", "base", "anim"]),
        newest_mtime(&content_dir.join("versions"), &["versions"]),
        newest_mtime(&content_dir.join("interfaces").join("_unpack"), &["if"]),
        max_of(
            &crate::map_format::source_dirs(content_dir)
                .iter()
                .map(|dir| newest_mtime(dir, &["jm2", "bin"]))
                .collect::<Vec<_>>(),
        ),
        newest_mtime(&content_dir.join("models").join("donor"), &["bin"]),
        newest_mtime(&content_dir.join("songs").join("_unpack"), &["mid"]),
        newest_mtime(&content_dir.join("textures").join("_unpack"), &["texture"]),
        file_mtime(&content_dir.join("pack").join("map.pack")),
        file_mtime(&content_dir.join("pack").join("interface.pack")),
    ])
}

pub fn run_build_cache(
    source_cache: &std::path::Path,
    content_dir: &std::path::Path,
    output_dir: &std::path::Path,
    client_data_dir: &std::path::Path,
    force_rebuild: bool,
    prebuilt_anims: Option<std::collections::BTreeMap<(u32, u32), Vec<u8>>>,
) -> Result<()> {
    use crate::assemble;
    use rs_dat2::archive;

    // Fast path: if the built cache already exists and no source content has
    // changed since the last successful build, skip the entire function.
    let config_path = client_data_dir.join("config");
    let master_stamp = output_dir.join(".stamp_cache_built");
    if !force_rebuild && master_stamp.exists() {
        let cache_exists = output_dir.join("main_file_cache.dat2").exists();
        if cache_exists {
            let master_input = max_of(&[cache_build_inputs(content_dir), file_mtime(&config_path)]);
            if is_up_to_date(&master_stamp, master_input) {
                info!("JS5 cache up to date — skipping build");
                return Ok(());
            }
        }
    }

    // Load version data from content/versions/*.versions
    let versions_dir = content_dir.join("versions");
    let versions = archive::load_versions_dir(&versions_dir)?;
    info!("Loaded version data for {} archives", versions.len());

    // Step 1: Open or create cache (never truncate existing data)
    info!("Step 1: Opening cache (incremental)...");
    assemble::open_or_create_cache(output_dir, archive::ARCHIVE_IDS)?;

    // Copy XTEA keys (needed for map encryption)
    let keys_dst = output_dir.join("keys.json");
    let keys_src = content_dir.join("keys.json");
    if keys_src.exists() {
        std::fs::copy(&keys_src, &keys_dst)?;
    } else if !keys_dst.exists() {
        warn!("No keys.json found — map encryption will fail without XTEA keys");
    }

    let mut store = rs_dat2::CacheStore::open(output_dir)?;

    // Pack registries for name hash resolution (named archives)
    let pack_dir = content_dir.join("pack");

    // Step 2: Assemble archives from content/ (manifest-free)
    info!("Step 2: Assembling archives from content...");

    // Archive 0 — Animations: unpack → per-animation files → .anim blobs → JS5
    {
        let anim_stamp = stamp_path(output_dir, archive::ANIMS);
        let scripts_dir = content_dir.join("scripts");
        let anim_dir = scripts_dir.join("_unpack").join("animations");
        let anim_input = max_of(&[
            newest_mtime(&anim_dir, &["anim"]),
            newest_mtime(&scripts_dir.join("_unpack"), &["base"]),
        ]);
        if !force_rebuild && is_up_to_date(&anim_stamp, anim_input) {
            info!(
                "  archive {:>2} (animations) -- up to date, skipping",
                archive::ANIMS
            );
        } else {
            use rayon::prelude::*;

            let format = archive::archive_format(archive::ANIMS).unwrap();

            if !anim_dir.is_dir() {
                anyhow::bail!(
                    "No unpacked animations at {} — run unpack first",
                    anim_dir.display()
                );
            }

            let packed_entries = if let Some(ref prebuilt) = prebuilt_anims {
                prebuilt.clone()
            } else {
                model::pack_anims(&scripts_dir, &pack_dir)?
            };

            // Group by group_id, preserving file IDs
            let mut groups: std::collections::BTreeMap<u32, Vec<(u32, Vec<u8>)>> =
                std::collections::BTreeMap::new();
            for ((gid, fid), data) in packed_entries {
                groups.entry(gid).or_default().push((fid, data));
            }

            // Parallel: encode + compress all groups
            let anim_versions = versions.get(&archive::ANIMS);
            let anim_pack =
                PackFile::load(&pack_dir.join("anim.pack")).unwrap_or_else(|_| PackFile::empty());
            let anim_out_dir = content_dir.join("models");
            std::fs::create_dir_all(&anim_out_dir)?;

            let prepared: Vec<_> = groups
                .par_iter_mut()
                .map(|(gid, frames)| {
                    frames.sort_by_key(|(fid, _)| *fid);
                    let refs: Vec<(u32, &[u8])> = frames
                        .iter()
                        .map(|(fid, data)| (*fid, data.as_slice()))
                        .collect();

                    let blob = if refs.len() > 1 {
                        rs_dat2::encode_group_entries_striped(&refs, 3, split_frame_into_stripes)
                    } else {
                        rs_dat2::encode_group_entries(&refs)
                    };

                    let group_ver = anim_versions
                        .and_then(|v| v.group_versions.get(gid))
                        .copied()
                        .unwrap_or(0);

                    let (raw, crc) = rs_dat2::prepare_group(
                        archive::ANIMS,
                        *gid,
                        &blob,
                        group_ver as u16,
                        format.allow_uncompressed,
                    )
                    .unwrap();

                    let files: Vec<rs_dat2::reference::FileInfo> = frames
                        .iter()
                        .map(|(fid, _)| rs_dat2::reference::FileInfo {
                            id: *fid,
                            name_hash: None,
                        })
                        .collect();

                    let name = anim_pack
                        .get_by_id(*gid)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("anim_{}", gid));

                    (*gid, raw, blob, crc, group_ver, files, name)
                })
                .collect();

            // Sequential: write to cache + .anim files (direct-to-cache, no blob)
            let mut group_infos = Vec::with_capacity(prepared.len());
            for (gid, raw, blob, crc, group_ver, files, name) in &prepared {
                store.write_raw_group(archive::ANIMS, *gid, raw)?;
                std::fs::write(anim_out_dir.join(format!("{name}.anim")), blob)?;

                group_infos.push(rs_dat2::reference::GroupInfo {
                    id: *gid,
                    name_hash: -1,
                    crc: *crc,
                    whirlpool: None,
                    version: *group_ver,
                    size: blob.len(),
                    file_count: files.len() as u32,
                    files: files.clone(),
                });
            }

            let anim_table_ver = versions
                .get(&archive::ANIMS)
                .map(|v| v.table_version)
                .unwrap_or(0);
            let table = rs_dat2::reference::ReferenceTable {
                archive_id: archive::ANIMS,
                format: format.format,
                version: anim_table_ver,
                named: format.named,
                whirlpooled: format.whirlpooled,
                groups: group_infos,
            };
            store.write_reference_table(archive::ANIMS, &table)?;
            info!(
                "  archive {:>2} ({}) -- {} groups, table_version={} (from all.anim → content/models/*.anim)",
                archive::ANIMS,
                format.content_dir,
                table.groups.len(),
                anim_table_ver
            );
            touch_stamp(&anim_stamp)?;
        } // end else (not up to date)
    }

    // Archives 2 + 16-22 — Config + strided config types from config blob
    {
        let config_stamp = stamp_path(output_dir, archive::CONFIG);
        let config_input = max_of(&[
            current_exe_mtime(),
            file_mtime(&config_path),
            newest_mtime(&versions_dir, &["versions"]),
        ]);
        if !force_rebuild && is_up_to_date(&config_stamp, config_input) {
            info!(
                "  archive {:>2} (config + strided 16-22) -- up to date, skipping",
                archive::CONFIG
            );
        } else {
            let config_blob = ConfigBlob::load(&config_path)?;

            // Archive 2 — Config groups from config blob
            {
                let format = archive::archive_format(archive::CONFIG).unwrap();
                let mut config_types: Vec<(String, u32)> = archive::CONFIG_GROUPS
                    .iter()
                    .map(|&(name, gid)| (name.to_string(), gid))
                    .collect();
                for &gid in archive::CONFIG_UNKNOWN_GROUPS {
                    config_types.push((format!("config{gid}"), gid));
                }

                // Collect encoded group data (sequential — reads config blob)
                let config_versions = versions.get(&archive::CONFIG);
                let mut encoded_groups: Vec<(u32, Vec<u8>, Vec<u32>, u32)> = Vec::new();

                for (type_name, group_id) in &config_types {
                    if !config_blob.has(type_name) {
                        continue;
                    }
                    let entries = config_blob.read_packed_data(type_name)?;
                    if entries.is_empty() {
                        continue;
                    }

                    let sorted: Vec<(u32, &[u8])> = entries
                        .iter()
                        .map(|(&id, data)| (id, data.as_slice()))
                        .collect();

                    let file_ids: Vec<u32> = sorted.iter().map(|(fid, _)| *fid).collect();
                    let group_data = rs_dat2::encode_group_entries(&sorted);
                    let group_ver = config_versions
                        .and_then(|v| v.group_versions.get(group_id))
                        .copied()
                        .unwrap_or(0);

                    encoded_groups.push((*group_id, group_data, file_ids, group_ver));
                }

                // Parallel: compress all config groups
                use rayon::prelude::*;
                let prepared_configs: Vec<_> = encoded_groups
                    .par_iter()
                    .map(|(gid, data, file_ids, ver)| {
                        let (raw, crc) = rs_dat2::prepare_group(
                            archive::CONFIG,
                            *gid,
                            data,
                            *ver as u16,
                            format.allow_uncompressed,
                        )
                        .unwrap();
                        (*gid, raw, crc, data.len(), file_ids.clone(), *ver)
                    })
                    .collect();

                // Sequential: write to cache
                let mut group_infos = Vec::new();
                for (gid, raw, crc, blob_len, file_ids, ver) in &prepared_configs {
                    store.write_raw_group(archive::CONFIG, *gid, raw)?;

                    let files: Vec<rs_dat2::reference::FileInfo> = file_ids
                        .iter()
                        .map(|fid| rs_dat2::reference::FileInfo {
                            id: *fid,
                            name_hash: None,
                        })
                        .collect();

                    group_infos.push(rs_dat2::reference::GroupInfo {
                        id: *gid,
                        name_hash: -1,
                        crc: *crc,
                        whirlpool: None,
                        version: *ver,
                        size: *blob_len,
                        file_count: files.len() as u32,
                        files,
                    });
                }

                group_infos.sort_by_key(|g| g.id);

                let config_table_ver = versions
                    .get(&archive::CONFIG)
                    .map(|v| v.table_version)
                    .unwrap_or(0);
                let table = rs_dat2::reference::ReferenceTable {
                    archive_id: archive::CONFIG,
                    format: format.format,
                    version: config_table_ver,
                    named: format.named,
                    whirlpooled: format.whirlpooled,
                    groups: group_infos,
                };
                store.write_reference_table(archive::CONFIG, &table)?;
                info!(
                    "  archive {:>2} ({}) -- {} groups, table_version={} from config blob",
                    archive::CONFIG,
                    format.content_dir,
                    table.groups.len(),
                    config_table_ver,
                );
            }

            // Archives 16-22 — Strided config types from config blob
            //
            // Each archive holds one config type with strided grouping:
            //   group_id = entry_id / stride, file_id = entry_id % stride
            {
                use rayon::prelude::*;

                let strided_types: &[(u8, &str, u32)] = &[
                    (archive::CONFIG_LOC, "loc", 256),
                    (archive::CONFIG_ENUM, "enum", 256),
                    (archive::CONFIG_NPC, "npc", 128),
                    (archive::CONFIG_OBJ, "obj", 256),
                    (archive::CONFIG_SEQ, "seq", 128),
                    (archive::CONFIG_SPOTANIM, "spotanim", 256),
                    (archive::CONFIG_VARBIT, "varbit", 1024),
                ];

                for &(aid, type_name, stride) in strided_types {
                    if !config_blob.has(type_name) {
                        info!(
                            "  archive {:>2} ({}) -- not in config blob, skipping",
                            aid, type_name
                        );
                        continue;
                    }
                    let entries = config_blob.read_packed_data(type_name)?;
                    if entries.is_empty() {
                        info!(
                            "  archive {:>2} ({}) -- 0 entries, skipping",
                            aid, type_name
                        );
                        continue;
                    }

                    let format = archive::archive_format(aid).unwrap();
                    let arch_versions = versions.get(&aid);

                    // Group entries by stride bucket
                    let mut groups: std::collections::BTreeMap<u32, Vec<(u32, &[u8])>> =
                        std::collections::BTreeMap::new();
                    for (&global_id, data) in &entries {
                        let group_id = global_id / stride;
                        let child_id = global_id % stride;
                        groups
                            .entry(group_id)
                            .or_default()
                            .push((child_id, data.as_slice()));
                    }

                    // Parallel: encode + compress
                    let mut sorted_groups: Vec<(u32, Vec<(u32, &[u8])>)> =
                        groups.into_iter().collect();
                    let prepared: Vec<_> = sorted_groups
                        .par_iter_mut()
                        .map(|(gid, children)| {
                            children.sort_by_key(|(id, _)| *id);
                            let blob = rs_dat2::encode_group_entries(children);

                            let group_ver = arch_versions
                                .and_then(|v| v.group_versions.get(gid))
                                .copied()
                                .unwrap_or(0);

                            let (raw, crc) = rs_dat2::prepare_group(
                                aid,
                                *gid,
                                &blob,
                                group_ver as u16,
                                format.allow_uncompressed,
                            )
                            .unwrap();

                            let files: Vec<rs_dat2::reference::FileInfo> = children
                                .iter()
                                .map(|(fid, _)| rs_dat2::reference::FileInfo {
                                    id: *fid,
                                    name_hash: None,
                                })
                                .collect();

                            (*gid, raw, crc, group_ver, blob.len(), files)
                        })
                        .collect();

                    // Sequential: write to cache
                    let mut group_infos = Vec::with_capacity(prepared.len());
                    for (gid, raw, crc, ver, blob_len, files) in &prepared {
                        store.write_raw_group(aid, *gid, raw)?;
                        group_infos.push(rs_dat2::reference::GroupInfo {
                            id: *gid,
                            name_hash: -1,
                            crc: *crc,
                            whirlpool: None,
                            version: *ver,
                            size: *blob_len,
                            file_count: files.len() as u32,
                            files: files.clone(),
                        });
                    }

                    group_infos.sort_by_key(|g| g.id);

                    let table_ver = arch_versions.map(|v| v.table_version).unwrap_or(0);
                    let table = rs_dat2::reference::ReferenceTable {
                        archive_id: aid,
                        format: format.format,
                        version: table_ver,
                        named: format.named,
                        whirlpooled: format.whirlpooled,
                        groups: group_infos,
                    };
                    store.write_reference_table(aid, &table)?;
                    info!(
                        "  archive {:>2} ({}) -- {} groups, {} entries, table_version={} from config blob",
                        aid,
                        type_name,
                        table.groups.len(),
                        entries.len(),
                        table_ver,
                    );
                }
            }

            touch_stamp(&config_stamp)?;
        } // end else (config not up to date)
    } // end config+strided scope

    // Archive 3 — Interfaces (independent of config blob)
    interfaces::build(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        client_data_dir,
        &versions,
        force_rebuild,
    )?;

    // Step 3: Passthrough archives (raw binary groups from _unpack/)
    info!("Step 3: Assembling remaining archives from content...");
    passthrough::build_passthrough_archives(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        &versions,
        force_rebuild,
    )?;

    synth::build(
        &mut store,
        content_dir,
        output_dir,
        client_data_dir,
        &versions,
        force_rebuild,
    )?;

    info!("Step 4: Assembling encode-based archives...");

    songs::build(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        &versions,
        force_rebuild,
    )?;
    textures::build(
        &mut store,
        content_dir,
        output_dir,
        client_data_dir,
        &versions,
        force_rebuild,
    )?;
    sprites::build(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        client_data_dir,
        &versions,
        force_rebuild,
    )?;
    maps::build(
        &mut store,
        content_dir,
        output_dir,
        &pack_dir,
        &versions,
        force_rebuild,
    )?;

    // All archives should be built from source — no passthrough needed.
    // The master index (255, 255) is rebuilt at server startup by Js5Cache::preload.

    drop(store);
    if source_cache.join("main_file_cache.dat2").exists() {
        verify::verify_cache(source_cache, output_dir)?;
    }
    std::fs::write(&master_stamp, b"ok")?;
    info!("Cache built at {}", output_dir.display());
    Ok(())
}
