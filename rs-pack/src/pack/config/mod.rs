#[cfg(since_647)]
use crate::archive::{self, ArchiveVersions};
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, insert_archive};
use crate::pack::pack::{ConfigProperties, ConfigSections};
#[cfg(since_647)]
use crate::pack::pack::{
    FileCache, collect_config_files, parse_config_sections, parse_config_sections_cached,
};
#[cfg(since_647)]
use crate::pack::pack_registry::PackedFile;
use crate::pack::pack_registry::{PackFile, PackId};
#[cfg(since_647)]
use crate::pack::packed_data::PackedData;
#[cfg(since_647)]
use anyhow::{Context, Result, ensure};
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(since_647)]
use rs_io::js5::{encode_group_entries, reference::FileInfo};
#[cfg(since_647)]
use std::collections::{BTreeMap, BTreeSet, HashMap};
#[cfg(since_647)]
use std::path::Path;

#[cfg(since_647)]
pub mod area;
#[cfg(since_647)]
pub mod bas;
#[cfg(since_647)]
pub mod billboard;
pub mod category;
#[cfg(since_647)]
pub mod controller;
#[cfg(since_647)]
pub mod cursor;
pub mod dbrow;
pub mod dbtable;
pub mod r#enum;
pub mod flo;
#[cfg(since_647)]
pub mod flu;
#[cfg(since_647)]
pub mod hitmark;
pub mod hunt;
pub mod idk;
pub mod inv;
#[cfg(since_647)]
pub mod light;
pub mod loc;
#[cfg(since_647)]
pub mod mel;
#[cfg(all(since_274, before_647))]
pub mod mes;
pub mod mesanim;
#[cfg(since_647)]
pub mod msi;
pub mod npc;
pub mod obj;
pub mod param;
#[cfg(since_647)]
pub mod particleeffector;
#[cfg(since_647)]
pub mod particleemitter;
#[cfg(since_647)]
pub mod quest;
#[cfg(since_647)]
pub mod quickchatcat;
#[cfg(since_647)]
pub mod quickchatphrase;
pub mod seq;
pub mod spotanim;
pub mod r#struct;
#[cfg(since_254)]
pub mod varbit;
#[cfg(since_647)]
pub mod varc;
#[cfg(since_647)]
pub mod varclansetting;
#[cfg(since_647)]
pub mod varcon;
#[cfg(since_647)]
pub mod varobj;
pub mod varp;
pub mod vars;

/// A table the cache holds empty: an entry per section, numbered by its name,
/// or by `pack` for a table only the server reads, whose sections keep their
/// properties to the server.
#[cfg(since_647)]
pub fn pack_empty_table(
    source_dir: &Path,
    type_name: &str,
    pack: Option<&PackFile>,
) -> Result<PackedData> {
    let files = collect_config_files(source_dir, type_name);
    let sections = parse_config_sections(&files, &HashMap::new())?;
    let mut ids = BTreeSet::new();
    for (name, props) in &sections {
        let id = match pack {
            Some(pack) => pack
                .get_by_debugname(name)
                .with_context(|| format!("[{name}]: no {type_name}.pack id"))?,
            None => {
                ensure!(props.is_empty(), "[{name}]: {type_name} entries are empty");
                let id = name.rsplit('_').next().unwrap_or(name);
                id.parse::<u32>().expect("Invalid config section id")
            }
        };
        ids.insert(id);
    }
    let count = ids.last().map_or(0, |id| id + 1);
    let mut packed = PackedData::new(count);
    for id in 0..count {
        if ids.contains(&id) {
            packed.start_entry();
            packed.finish_entry();
        } else {
            packed.skip_entry();
        }
    }
    Ok(packed)
}

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
    for &(name, gid, ..) in archive::CONFIG_EMPTY_TABLES {
        let entries = read_config_entries(config, name)?;
        if !entries.is_empty() {
            config_groups.insert(gid, entries);
        }
    }
    write_config_archive(store, archive::CONFIG, versions, config_groups)?;
    write_config_archive(
        store,
        archive::BILLBOARDS,
        versions,
        BTreeMap::from([(0, read_config_entries(config, "billboard")?)]),
    )?;
    write_config_archive(
        store,
        archive::PARTICLES,
        versions,
        BTreeMap::from([
            (0, read_config_entries(config, "particleemitter")?),
            (1, read_config_entries(config, "particleeffector")?),
        ]),
    )?;

    let mut quickchat = BTreeMap::new();
    let mut global = BTreeMap::new();
    for (gid, name) in [(0, "quickchatcat"), (1, "quickchatphrase")] {
        let (own, shared): (Vec<_>, Vec<_>) = read_config_entries(config, name)?
            .into_iter()
            .partition(|&(id, _)| id < archive::QUICKCHAT_GLOBAL_FIRST as u32);
        quickchat.insert(gid, own);
        global.insert(
            gid,
            shared
                .into_iter()
                .map(|(id, data)| (id - archive::QUICKCHAT_GLOBAL_FIRST as u32, data))
                .collect(),
        );
    }
    write_config_archive(store, archive::QUICKCHAT, versions, quickchat)?;
    write_config_archive(store, archive::QUICKCHAT_GLOBAL, versions, global)?;

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

/// The cache's table for a type whose bodies only the server reads: one empty
/// entry per `_unpack` section, at the ids the cache has them.
#[cfg(since_647)]
pub(super) fn client_table(
    file_cache: &FileCache,
    kind: &str,
    pack: &PackFile,
    constants: &HashMap<String, String>,
) -> Result<PackedData> {
    client_table_with(file_cache, kind, pack, constants, |_, _, _| {})
}

/// A param the client's table holds is written where it stands in the record,
/// for both sides. Any other stays on the server.
#[cfg(since_647)]
pub(super) fn mark_client_params(
    props: &[(String, String)],
    param: &PackFile,
    client_params: PackId,
) -> Vec<(String, String)> {
    props
        .iter()
        .map(|(key, value)| {
            let client = key == "param"
                && value
                    .split_once(',')
                    .and_then(|(name, _)| param.get_by_debugname(name.trim()))
                    .is_some_and(|id| id < client_params);
            let key = if client { "clientparam" } else { key.as_str() };
            (key.to_owned(), value.clone())
        })
        .collect()
}

/// The cache's table with each `_unpack` property written by `write`.
#[cfg(since_647)]
pub(super) fn client_table_with(
    file_cache: &FileCache,
    kind: &str,
    pack: &PackFile,
    constants: &HashMap<String, String>,
    mut write: impl FnMut(&mut PackedData, &str, &str),
) -> Result<PackedData> {
    let configs = file_cache.client_configs(kind, constants)?;
    let count = file_cache.client_count(kind, pack, constants)?.unwrap_or(0);
    let mut client = PackedData::new(count);
    for id in 0..count {
        if let Some(props) = pack.get_by_id(id).and_then(|name| configs.get(name)) {
            client.start_entry();
            for (key, value) in props {
                write(&mut client, key, value);
            }
            client.finish_entry();
        } else {
            client.skip_entry();
        }
    }
    Ok(client)
}

/// A table only the client reads: an entry per section, by registry id.
#[cfg(since_647)]
pub(super) fn client_config_table(
    file_cache: &FileCache,
    kind: &str,
    pack: &PackFile,
    constants: &HashMap<String, String>,
    mut write: impl FnMut(&mut PackedData, &str, &ConfigProperties),
) -> Result<PackedFile> {
    let configs = parse_config_sections_cached(file_cache, kind, constants)?;
    let mut client = PackedData::new(pack.max);
    for id in 0..pack.max {
        let section = pack
            .get_by_id(id)
            .and_then(|name| configs.get(name).map(|props| (name, props)));
        if let Some((name, props)) = section {
            client.start_entry();
            write(&mut client, name, props);
            client.finish_entry();
        } else {
            client.skip_entry();
        }
    }
    Ok(PackedFile {
        server: PackedData::default(),
        client: Some(client),
    })
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
