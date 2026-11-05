#[cfg(since_647)]
use crate::config_blob::ConfigBlob;
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, stamp_path, write_archive};
#[cfg(since_647)]
use crate::startup::{
    current_exe_mtime, file_mtime, is_up_to_date, max_of, newest_mtime, write_stamp,
};
#[cfg(since_647)]
use anyhow::Result;
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(since_647)]
use rs_dat2::{
    CacheStore,
    archive::{self, ArchiveVersions},
    encode_group_entries,
    reference::FileInfo,
};
#[cfg(since_647)]
use std::collections::{BTreeMap, HashMap};
#[cfg(since_647)]
use std::path::Path;
#[cfg(since_647)]
use tracing::info;

pub mod category;
pub mod dbrow;
pub mod dbtable;
pub mod r#enum;
pub mod flo;
#[cfg(since_647)]
pub mod flu;
#[cfg(since_647)]
pub mod generic;
pub mod hunt;
pub mod idk;
pub mod inv;
pub mod loc;
#[cfg(all(since_274, before_647))]
pub mod mes;
pub mod mesanim;
pub mod npc;
pub mod obj;
pub mod param;
pub mod seq;
pub mod spotanim;
pub mod r#struct;
#[cfg(since_254)]
pub mod varbit;
pub mod varn;
pub mod varp;
pub mod vars;

#[cfg(since_647)]
pub fn pack_archives(
    store: &mut CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    config_path: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    let stamp = stamp_path(output_dir, archive::CONFIG);
    let inputs = max_of(&[
        current_exe_mtime(),
        file_mtime(config_path),
        newest_mtime(&content_dir.join("versions"), &["versions"]),
    ]);
    if !force_rebuild && is_up_to_date(&stamp, inputs) {
        info!("Config archives up to date, skipping");
        return Ok(());
    }

    let blob = ConfigBlob::load(config_path)?;
    let mut config_groups = BTreeMap::new();
    for &(name, gid) in archive::CONFIG_GROUPS {
        let entries = blob.read_packed_data(name)?;
        if !entries.is_empty() {
            config_groups.insert(gid, entries);
        }
    }
    for &gid in archive::CONFIG_UNKNOWN_GROUPS {
        let entries = blob.read_packed_data(&format!("config{gid}"))?;
        if !entries.is_empty() {
            config_groups.insert(gid, entries);
        }
    }
    write_config_archive(store, archive::CONFIG, versions, config_groups)?;

    for &(aid, name, stride) in &[
        (archive::CONFIG_LOC, "loc", 256),
        (archive::CONFIG_ENUM, "enum", 256),
        (archive::CONFIG_NPC, "npc", 128),
        (archive::CONFIG_OBJ, "obj", 256),
        (archive::CONFIG_SEQ, "seq", 128),
        (archive::CONFIG_SPOTANIM, "spotanim", 256),
        (archive::CONFIG_VARBIT, "varbit", 1024),
    ] {
        let entries = blob.read_packed_data(name)?;
        if entries.is_empty() {
            continue;
        }
        let mut groups = BTreeMap::new();
        for (id, data) in entries {
            groups
                .entry(id / stride)
                .or_insert_with(BTreeMap::new)
                .insert(id % stride, data);
        }
        write_config_archive(store, aid, versions, groups)?;
    }
    write_stamp(&stamp)?;
    Ok(())
}

#[cfg(since_647)]
fn write_config_archive(
    store: &mut CacheStore,
    aid: u8,
    versions: &HashMap<u8, ArchiveVersions>,
    groups: BTreeMap<u32, BTreeMap<u32, Vec<u8>>>,
) -> Result<()> {
    let prepared = groups
        .into_par_iter()
        .map(|(gid, entries)| {
            let refs: Vec<_> = entries
                .iter()
                .map(|(&id, data)| (id, data.as_slice()))
                .collect();
            let data = encode_group_entries(&refs);
            let mut group = PreparedGroup::new(aid, gid, &data, versions)?;
            group.files = entries
                .keys()
                .map(|&id| FileInfo {
                    id,
                    name_hash: None,
                })
                .collect();
            Ok(group)
        })
        .collect::<Result<Vec<_>>>()?;
    write_archive(
        store,
        aid,
        archive::archive_format(aid).unwrap(),
        versions,
        prepared,
        "from configs",
    )
}
