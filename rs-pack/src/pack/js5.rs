use crate::archive::{self, ArchiveFormat, ArchiveVersions};
use crate::cache::js5::PackedCache;
use crate::pack::incremental::{
    self, Hash, Key, LOBBY_COMMANDS, Stage, dir_key, files_under, game_files_under,
};
use crate::pack::names::{AssetNames, archive_name};
#[cfg(since_647)]
use crate::pack::other::wma;
use crate::pack::other::{asset, material};
#[cfg(since_647)]
use crate::pack::pack::collect_config_files;
use crate::pack::pack_dir::PrivatePack;
use crate::pack::pack_registry::PackFile;
use anyhow::Result;
#[cfg(since_647)]
use anyhow::{Context, ensure};
use rayon::prelude::*;
use rs_crypto::whirlpool;
use rs_io::js5::compression::compress_best;
use rs_io::js5::index::{FileInfo, GroupInfo, Js5Index, encode_index};
use rs_io::js5::{compute_group_crc, detect_file_count, encode_group_entries};
use rs_io::stringtools::hash32;
use runec::clientscript;
use runec::sources::roots;
use std::collections::{HashMap, HashSet, hash_map::Entry};
use std::fs;
use std::path::{Path, PathBuf};
use std::slice;
use std::time::Instant;
use tracing::{debug, info};

pub fn load_name_hashes(pack_dir: &Path, archive_id: u8, group_ids: &[u32]) -> HashMap<u32, i32> {
    let pack_name = pack_filename_for_archive(archive_id);
    let mut result = HashMap::new();

    let hashes_path = pack_dir.join(format!("{pack_name}.hashes"));
    if hashes_path.exists() {
        if let Ok(text) = fs::read_to_string(&hashes_path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let Some((id_str, hash_str)) = line.split_once('=') else {
                    continue;
                };
                if let (Ok(gid), Ok(hash)) =
                    (id_str.trim().parse::<u32>(), hash_str.trim().parse::<i32>())
                {
                    result.insert(gid, hash);
                }
            }
        }
    }

    let pack_path = pack_dir.join(format!("{pack_name}.pack"));
    if pack_path.exists() {
        if let Ok(pf) = PackFile::load(&pack_path) {
            for &gid in group_ids {
                if let Entry::Vacant(e) = result.entry(gid) {
                    if let Some(name) = pf.get_by_id(gid) {
                        e.insert(hash32(name));
                    }
                }
            }
        }
    }

    result
}

fn pack_filename_for_archive(archive_id: u8) -> &'static str {
    match archive_id {
        3 => "interface",
        5 => "map",
        6 => "songs",
        8 => "sprites",
        10 => "binary",
        12 => "clientscript",
        13 => "fontmetrics",
        23 => "worldmap",
        30 => "dlls",
        31 => "shaders",
        32 | 34 => "loading_sprites",
        33 => "loading_backgrounds",
        _ => "unknown",
    }
}

pub struct PreparedGroup {
    pub id: u32,
    pub raw: Vec<u8>,
    pub crc: u32,
    pub version: u32,
    pub size: usize,
    pub name_hash: i32,
    pub whirlpool: Option<[u8; 64]>,
    pub files: Vec<FileInfo>,
}

impl PreparedGroup {
    /// Keeps the table's files for a group now holding `count` of them.
    pub fn fit_files(&mut self, count: usize) {
        self.files.truncate(count);
        let mut next = self.files.last().map_or(0, |file| file.id + 1);
        while self.files.len() < count {
            self.files.push(FileInfo {
                id: next,
                name_hash: None,
            });
            next += 1;
        }
    }

    pub fn set_files(&mut self, ids: impl IntoIterator<Item = u32>) {
        self.files = ids
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                let original = self
                    .files
                    .get(i)
                    .filter(|f| f.id == id)
                    .or_else(|| self.files.iter().find(|f| f.id == id));
                FileInfo {
                    id,
                    name_hash: original.and_then(|f| f.name_hash),
                }
            })
            .collect();
    }

    pub fn new(
        aid: u8,
        id: u32,
        data: &[u8],
        versions: &HashMap<u8, ArchiveVersions>,
    ) -> Result<Self> {
        let format = archive::archive_format(aid).unwrap();
        let version = group_version(versions, aid, &id);
        let (raw, crc) = prepare_group(aid, id, data, version as u16, format.allow_uncompressed)?;
        let whirlpool = format
            .has_group_whirlpools
            .then(|| whirlpool(&raw[..raw.len() - 2]));
        Ok(Self {
            id,
            raw,
            crc,
            version,
            size: data.len(),
            name_hash: -1,
            whirlpool,
            files: build_file_infos(versions, aid, id, detect_file_count(data) as u32),
        })
    }
}

pub fn insert_archive(
    store: &mut PackedCache,
    aid: u8,
    format: &ArchiveFormat,
    versions: &HashMap<u8, ArchiveVersions>,
    groups: Vec<PreparedGroup>,
    label: &str,
) -> Result<()> {
    let mut group_infos = Vec::with_capacity(groups.len());
    for g in groups {
        store.groups.insert((aid, g.id), g.raw);
        group_infos.push(GroupInfo {
            id: g.id,
            name_hash: g.name_hash,
            crc: g.crc,
            whirlpool: g.whirlpool,
            version: g.version,
            size: g.size,
            file_count: g.files.len() as u32,
            files: g.files,
        });
    }
    // Groups the cache lists without data keep their recorded entry.
    let absent = versions.get(&aid).map(|v| &v.absent_groups);
    for (&id, &(crc, name_hash)) in absent.into_iter().flatten() {
        if group_infos.iter().any(|g| g.id == id) {
            continue;
        }
        let files = build_file_infos(versions, aid, id, 1);
        store.absent.insert((aid, id));
        group_infos.push(GroupInfo {
            id,
            name_hash,
            crc,
            whirlpool: None,
            version: group_version(versions, aid, &id),
            size: 0,
            file_count: files.len() as u32,
            files,
        });
    }

    group_infos.sort_by_key(|g| g.id);

    let table_ver = versions.get(&aid).map(|v| v.table_version).unwrap_or(0);
    let table = Js5Index {
        archive_id: aid,
        format: format.format,
        index_version: table_ver,
        named: format.named,
        has_group_whirlpools: format.has_group_whirlpools,
        groups: group_infos,
    };
    let encoded = encode_index(&table);
    let key = Hash::new("table").int(u64::from(aid)).bytes(&encoded).key();
    let raw = incremental::memo(key, || {
        Ok(compress_best(
            &encoded,
            true,
            true,
            archive::codec_override(archive::REFERENCE_TABLES, aid as u32),
        )?)
    })?;
    store
        .groups
        .insert((archive::REFERENCE_TABLES, aid as u32), raw);
    info!(
        "  archive {:>2} ({}) -- {} groups, table_version={} {}",
        aid,
        format.content_dir,
        table.groups.len(),
        table_ver,
        label,
    );
    store.tables.insert(aid, table);
    Ok(())
}

pub fn insert_empty_archive(
    store: &mut PackedCache,
    aid: u8,
    format: &ArchiveFormat,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    insert_archive(store, aid, format, versions, Vec::new(), "(empty)")
}

/// Build file info list from version data if available, else from file_count.
pub fn build_file_infos(
    versions: &HashMap<u8, ArchiveVersions>,
    aid: u8,
    gid: u32,
    file_count: u32,
) -> Vec<FileInfo> {
    if let Some(file_entries) = versions.get(&aid).and_then(|v| v.group_file_ids.get(&gid)) {
        file_entries
            .iter()
            .map(|&(id, hash)| FileInfo {
                id,
                name_hash: hash,
            })
            .collect()
    } else {
        (0..file_count)
            .map(|i| FileInfo {
                id: i,
                name_hash: None,
            })
            .collect()
    }
}

pub fn group_version(versions: &HashMap<u8, ArchiveVersions>, aid: u8, gid: &u32) -> u32 {
    versions
        .get(&aid)
        .and_then(|v| v.group_versions.get(gid))
        .copied()
        .unwrap_or(0)
}

const SIMPLE_ARCHIVES: &[u8] = &[
    archive::ANIMS,
    archive::BASES,
    archive::MODELS,
    archive::BINARY,
    archive::CLIENTSCRIPT,
    archive::FONTMETRICS,
    archive::VORBIS,
    archive::INSTRUMENTS,
    archive::WORLDMAP,
    archive::MATERIALS,
    #[cfg(since_647)]
    archive::DEFAULTS,
    #[cfg(since_647)]
    archive::DLLS,
    #[cfg(since_647)]
    archive::LOADING_SCREENS,
    #[cfg(since_647)]
    archive::WORLDMAP_LOCS,
    #[cfg(since_742)]
    archive::TEXTURES,
    #[cfg(since_742)]
    archive::TEXTURES_HDR,
    #[cfg(since_742)]
    archive::CUTSCENES,
];

/// The extension of an archive whose groups are their raw bytes.
pub fn raw_extension(aid: u8) -> Option<&'static str> {
    Some(match aid {
        archive::ANIMS => "anim",
        archive::BASES => "base",
        archive::MODELS => "ob2",
        archive::VORBIS => "vorbis",
        #[cfg(since_742)]
        archive::TEXTURES | archive::TEXTURES_HDR => "png",
        #[cfg(since_742)]
        archive::CUTSCENES => "cutscene",
        _ => return None,
    })
}

/// The groups the `.wma` configs make: every map's details, and its elements.
#[cfg(since_647)]
fn pack_world_maps(
    scripts_dir: &Path,
    pack_dir: &Path,
    names: &AssetNames,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<Vec<PreparedGroup>> {
    let aid = archive::WORLDMAP;
    let mut maps = Vec::new();
    for path in collect_config_files(scripts_dir, wma::KIND) {
        let text = fs::read_to_string(&path).with_context(|| path.display().to_string())?;
        maps.extend(wma::parse(&text, names).with_context(|| path.display().to_string())?);
    }
    ensure!(
        !maps.is_empty(),
        "No .{} configs in {}",
        wma::KIND,
        scripts_dir.display()
    );
    let groups = wma::groups(maps, names)?;
    let gids: Vec<u32> = groups.iter().map(|(gid, _)| *gid).collect();
    let name_hashes = load_name_hashes(pack_dir, aid, &gids);
    groups
        .iter()
        .map(|(gid, files)| {
            let entries: Vec<(u32, &[u8])> = files
                .iter()
                .enumerate()
                .map(|(position, data)| (position as u32, data.as_slice()))
                .collect();
            let data = encode_group_entries(&entries);
            let mut group = PreparedGroup::new(aid, *gid, &data, versions)?;
            group.fit_files(files.len());
            group.name_hash = name_hashes.get(gid).copied().unwrap_or(-1);
            Ok(group)
        })
        .collect()
}

pub fn pack_asset_archives(
    store: &mut PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
    names: &AssetNames,
    verify: bool,
) -> Result<()> {
    let cache = SIMPLE_ARCHIVES
        .par_iter()
        .map(|&aid| -> Result<_> {
            let start = Instant::now();
            let mut cache = PackedCache::default();
            let format = archive::archive_format(aid).unwrap();
            let dir = content_dir.join(format.content_dir);

            if aid == archive::CLIENTSCRIPT {
                let private = PrivatePack::copy(pack_dir)?;
                let stage = Stage::new(
                    "clientscript",
                    vec![
                        versions_dep(versions, aid),
                        ("verify".to_owned(), Key::from(verify)),
                        (
                            "pack".to_owned(),
                            dir_key(private.path(), &[LOBBY_COMMANDS]),
                        ),
                    ],
                );
                let sources = game_files_under(&roots(&dir));
                let mut prepared = stage.unit_groups(&(), 0, &sources, |_| {
                    let compiled = clientscript::compile_directory(&dir, private.path())?;
                    if verify {
                        private.check_unchanged(pack_dir, "clientscript.pack")?;
                    }
                    compiled
                        .into_iter()
                        .map(|(id, data)| PreparedGroup::new(aid, id, &data, versions))
                        .collect()
                })?;
                private.pin(pack_dir, "clientscript.pack", verify)?;
                stage.finish(vec![(
                    "pack".to_owned(),
                    dir_key(private.path(), &[LOBBY_COMMANDS]),
                )]);
                let ids: Vec<_> = prepared.iter().map(|group| group.id).collect();
                let hashes = load_name_hashes(pack_dir, aid, &ids);
                for group in &mut prepared {
                    group.name_hash = hashes.get(&group.id).copied().unwrap_or(-1);
                }
                insert_archive(
                    &mut cache,
                    aid,
                    format,
                    versions,
                    prepared,
                    "from clientscript sources",
                )?;
                return Ok(cache);
            }

            // Every material is in the archive's one file.
            if aid == archive::MATERIALS {
                let stage =
                    Stage::new(&format!("archive {aid}"), vec![versions_dep(versions, aid)]);
                let sources = files_under(slice::from_ref(&dir));
                let prepared = stage.unit_groups(names, 0, &sources, |names| {
                    let data = material::pack_files(&sources, names)?;
                    let mut group = PreparedGroup::new(aid, 0, &data, versions)?;
                    group.set_files([0]);
                    Ok(vec![group])
                })?;
                stage.finish(Vec::new());
                insert_archive(
                    &mut cache,
                    aid,
                    format,
                    versions,
                    prepared,
                    "from material sources",
                )?;
                return Ok(cache);
            }

            let group_files = match (asset::extension(aid).or(raw_extension(aid)), aid) {
                (Some(extension), _) => names.files(&dir, archive_name(aid), &[extension])?,
                #[cfg(since_647)]
                (None, archive::DLLS) => names.files(&dir, "dlls", &["dll", "dylib", "so"])?,
                _ => Vec::new(),
            };

            if group_files.is_empty() {
                insert_empty_archive(&mut cache, aid, format, versions)?;
                return Ok(cache);
            }

            let stage = Stage::new(&format!("archive {aid}"), vec![versions_dep(versions, aid)]);
            let build = |gid: u32, path: &PathBuf, names: &AssetNames| -> Result<PreparedGroup> {
                let mut file_ids = None;
                let data = if asset::extension(aid).is_some() {
                    let entries = asset::pack_file(aid, gid, path, names)?;
                    file_ids = Some(entries.iter().map(|(id, _)| *id).collect::<Vec<_>>());
                    encode_group_entries(
                        &entries
                            .iter()
                            .map(|(id, data)| (*id, data.as_slice()))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    fs::read(path)?
                };
                let mut group = PreparedGroup::new(aid, gid, &data, versions)?;
                if let Some(ids) = file_ids {
                    group.set_files(ids);
                }
                Ok(group)
            };
            // A world map's sources name the squares it packs, so the archive
            // is one unit over every file it could read.
            let mut prepared = if aid == archive::WORLDMAP {
                #[cfg(since_647)]
                let scripts = content_dir.join("scripts");
                let sources = files_under(slice::from_ref(&dir));
                #[cfg(since_647)]
                let sources = [sources, collect_config_files(&scripts, wma::KIND)].concat();
                stage.unit_groups(names, 0, &sources, |names| {
                    let groups = group_files
                        .par_iter()
                        .map(|(gid, path)| build(*gid, path, names))
                        .collect::<Result<Vec<_>>>()?;
                    #[cfg(since_647)]
                    let groups: Vec<_> = groups
                        .into_iter()
                        .chain(pack_world_maps(&scripts, pack_dir, names, versions)?)
                        .collect();
                    Ok(groups)
                })?
            } else {
                stage.groups(names, &group_files, |path| vec![path.clone()], build)?
            };
            stage.finish(Vec::new());

            if format.named || aid == archive::WORLDMAP {
                let assets: HashSet<u32> = group_files.iter().map(|(gid, _)| *gid).collect();
                let ids: Vec<u32> = prepared.iter().map(|group| group.id).collect();
                let hashes = load_name_hashes(pack_dir, aid, &ids);
                for group in &mut prepared {
                    group.name_hash = if format.named || !assets.contains(&group.id) {
                        hashes.get(&group.id).copied().unwrap_or(-1)
                    } else {
                        -1
                    };
                }
            }

            insert_archive(
                &mut cache,
                aid,
                format,
                versions,
                prepared,
                "from asset sources",
            )?;
            debug!(
                "Packed archive {aid} in {:.2}s",
                start.elapsed().as_secs_f64()
            );
            Ok(cache)
        })
        .try_reduce(PackedCache::default, |mut a, b| {
            a.extend(b);
            Ok(a)
        })?;
    store.extend(cache);
    Ok(())
}

/// What an archive's groups take from its versions: their versions and file
/// ids.
pub fn versions_dep(versions: &HashMap<u8, ArchiveVersions>, aid: u8) -> (String, Key) {
    let mut hash = Hash::new("versions");
    if let Some(archive) = versions.get(&aid) {
        let mut groups: Vec<_> = archive.group_versions.iter().collect();
        groups.sort_unstable();
        for (gid, version) in groups {
            hash.int(u64::from(*gid)).int(u64::from(*version));
        }
        let mut files: Vec<_> = archive.group_file_ids.iter().collect();
        files.sort_unstable_by_key(|(gid, _)| **gid);
        for (gid, ids) in files {
            hash.int(u64::from(*gid)).int(ids.len() as u64);
            for (id, name_hash) in ids {
                hash.int(u64::from(*id))
                    .int(name_hash.map_or(u64::MAX, |name| u64::from(name as u32)));
            }
        }
    }
    ("versions".to_owned(), hash.key())
}

pub fn prepare_group(
    archive_id: u8,
    group_id: u32,
    data: &[u8],
    version: u16,
    allow_uncompressed: bool,
) -> Result<(Vec<u8>, u32)> {
    let allow_bzip2 = archive::archive_format(archive_id)
        .map(|f| f.allow_bzip2)
        .unwrap_or(true);
    let forced = archive::codec_override(archive_id, group_id);
    let key = Hash::new("group")
        .int(allow_uncompressed as u64)
        .int(allow_bzip2 as u64)
        .int(forced.map_or(u64::MAX, |codec| codec as u64))
        .int(u64::from(version))
        .bytes(data)
        .key();
    let raw = incremental::memo(key, || {
        let mut raw = compress_best(data, allow_uncompressed, allow_bzip2, forced)?;
        raw.reserve_exact(2);
        raw.extend_from_slice(&version.to_be_bytes());
        Ok(raw)
    })?;
    let crc = compute_group_crc(&raw[..raw.len() - 2]);
    Ok((raw, crc))
}

#[cfg(test)]
mod tests {
    use super::{PreparedGroup, insert_archive};
    use crate::archive::{self, ArchiveVersions};
    use crate::cache::js5::PackedCache;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn absent_groups_enter_the_table_without_data() {
        let aid = archive::VORBIS;
        let versions = HashMap::from([(
            aid,
            ArchiveVersions {
                table_version: 26,
                group_versions: HashMap::from([(0, 1), (1, 1), (2, 3)]),
                group_file_ids: HashMap::new(),
                absent_groups: HashMap::from([(1, (0x1fe351ce, -1)), (2, (0xb96c7b96, -1))]),
            },
        )]);
        let setup = PreparedGroup::new(aid, 0, b"setup", &versions).unwrap();
        let restored = PreparedGroup::new(aid, 2, b"restored", &versions).unwrap();
        let restored_crc = restored.crc;
        let mut store = PackedCache::default();
        let format = archive::archive_format(aid).unwrap();
        insert_archive(
            &mut store,
            aid,
            format,
            &versions,
            vec![setup, restored],
            "",
        )
        .unwrap();
        let groups: Vec<_> = store.tables[&aid]
            .groups
            .iter()
            .map(|g| (g.id, g.crc, g.version, g.files.len()))
            .collect();
        assert_eq!(groups[1], (1, 0x1fe351ce, 1, 1));
        assert_eq!(groups[2], (2, restored_crc, 3, 1));
        assert_eq!(store.absent, HashSet::from([(aid, 1)]));
        assert!(!store.groups.contains_key(&(aid, 1)));
        assert_eq!(store.groups.len(), 3);
    }
}
