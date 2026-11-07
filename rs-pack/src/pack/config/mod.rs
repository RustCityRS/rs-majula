#[cfg(since_647)]
use crate::archive::{self, ArchiveVersions};
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, insert_archive};
use crate::pack::pack::{ConfigProperties, ConfigSections};
#[cfg(since_647)]
use crate::pack::pack_registry::PackedFile;
use crate::pack::pack_registry::{PackFile, PackId};
#[cfg(since_647)]
use crate::pack::packed_data::PackedData;
#[cfg(since_647)]
use anyhow::{Context, Result};
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(since_647)]
use rs_io::js5::{encode_group_entries, reference::FileInfo};
#[cfg(since_647)]
use std::collections::{BTreeMap, HashMap};

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
pub mod varp;
pub mod vars;

#[cfg(since_647)]
pub fn pack_archives(
    store: &mut PackedCache,
    config: &HashMap<String, PackedFile>,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    let mut config_groups = BTreeMap::new();
    for &(name, gid) in archive::CONFIG_GROUPS {
        let entries = read_config_entries(config, name)?;
        if !entries.is_empty() {
            config_groups.insert(gid, entries);
        }
    }
    for &gid in archive::CONFIG_UNKNOWN_GROUPS {
        let entries = read_config_entries(config, &format!("config{gid}"))?;
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
        let entries = read_config_entries(config, name)?;
        if entries.is_empty() {
            continue;
        }
        let mut groups = BTreeMap::new();
        for (id, data) in entries {
            groups
                .entry(id / stride)
                .or_insert_with(Vec::new)
                .push((id % stride, data));
        }
        write_config_archive(store, aid, versions, groups)?;
    }
    Ok(())
}

#[cfg(since_647)]
fn read_config_entries<'a>(
    config: &'a HashMap<String, PackedFile>,
    name: &str,
) -> Result<Vec<(u32, &'a [u8])>> {
    let Some(client) = config.get(name).and_then(|file| file.client.as_ref()) else {
        return Ok(Vec::new());
    };
    Ok(PackedData::entries(&client.dat, &client.idx)
        .with_context(|| format!("Reading {name} configs"))?
        .filter(|(_, data)| !data.is_empty())
        .map(|(id, data)| (id as u32, data))
        .collect())
}

#[cfg(since_647)]
fn write_config_archive(
    store: &mut PackedCache,
    aid: u8,
    versions: &HashMap<u8, ArchiveVersions>,
    groups: BTreeMap<u32, Vec<(u32, &[u8])>>,
) -> Result<()> {
    let prepared = groups
        .into_par_iter()
        .map(|(gid, entries)| {
            let data = encode_group_entries(&entries);
            let mut group = PreparedGroup::new(aid, gid, &data, versions)?;
            group.files = entries
                .iter()
                .map(|&(id, _)| FileInfo {
                    id,
                    name_hash: None,
                })
                .collect();
            Ok(group)
        })
        .collect::<Result<Vec<_>>>()?;
    insert_archive(
        store,
        aid,
        archive::archive_format(aid).unwrap(),
        versions,
        prepared,
        "from configs",
    )
}

pub(super) fn config_entry<'a>(
    kind: &str,
    pack: &'a PackFile,
    configs: &'a ConfigSections,
    id: PackId,
) -> Option<(&'a str, &'a ConfigProperties)> {
    let name = pack.get_by_id(id);
    if cfg!(before_647) && name.is_none() {
        panic!("Unknown {kind} id: {id}");
    }
    let name = name?;
    let props = configs.get(name);
    if cfg!(before_647) && props.is_none() {
        panic!("Unknown {kind} config: {name}");
    }
    Some((name, props?))
}
