use crate::archive::{self, ArchiveFormat, ArchiveVersions};
use crate::cache::js5::PackedCache;
use crate::pack::model::source as model_source;
use crate::pack::names::{AssetNames, archive_name};
use crate::pack::other::asset;
use crate::pack::pack_registry::PackFile;
use crate::pack::sound::{vorbis, vorbis_setup};
use anyhow::{Context, Result, ensure};
use rayon::prelude::*;
use rs_crypto::whirlpool;
use rs_io::js5::compression::compress_best;
use rs_io::js5::java_string_hash;
use rs_io::js5::reference::{FileInfo, GroupInfo, ReferenceTable, encode_reference_table};
use rs_io::js5::{compute_group_crc, detect_file_count, encode_group_entries};
use runec::clientscript;
use std::collections::{HashMap, hash_map::Entry};
use std::fs;
use std::path::{Path, PathBuf};
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
                        e.insert(java_string_hash(name));
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
        23 => "worldmap_data",
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
        let whirlpool = format.whirlpooled.then(|| whirlpool(&raw[..raw.len() - 2]));
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

    group_infos.sort_by_key(|g| g.id);

    let table_ver = versions.get(&aid).map(|v| v.table_version).unwrap_or(0);
    let table = ReferenceTable {
        archive_id: aid,
        format: format.format,
        version: table_ver,
        named: format.named,
        whirlpooled: format.whirlpooled,
        groups: group_infos,
    };
    let raw = compress_best(
        &encode_reference_table(&table),
        true,
        true,
        archive::codec_override(255, aid as u32),
    )?;
    store.groups.insert((255, aid as u32), raw);
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
    archive::BASES,
    archive::MODELS,
    archive::BINARY,
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
    archive::LOADING_SCREENS,
    archive::WORLDMAP_LOCS,
];

pub fn pack_asset_archives(
    store: &mut PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    let names = AssetNames::load(pack_dir)?;
    let cache = SIMPLE_ARCHIVES
        .par_iter()
        .map(|&aid| -> Result<_> {
            let start = Instant::now();
            let mut cache = PackedCache::default();
            let format = archive::archive_format(aid).unwrap();

            if aid == archive::CLIENTSCRIPT {
                let compiled = clientscript::compile_directory(
                    &content_dir.join(format.content_dir),
                    pack_dir,
                )?;
                let hashes =
                    load_name_hashes(pack_dir, aid, &compiled.keys().copied().collect::<Vec<_>>());
                let prepared = compiled
                    .into_iter()
                    .map(|(id, data)| {
                        let mut group = PreparedGroup::new(aid, id, &data, versions)?;
                        group.name_hash = hashes.get(&id).copied().unwrap_or(-1);
                        Ok(group)
                    })
                    .collect::<Result<Vec<_>>>()?;
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

            let group_files = if let Some(extension) = asset::extension(aid) {
                names.files(
                    &content_dir.join(format.content_dir),
                    archive_name(aid),
                    &[extension],
                )?
            } else if aid == archive::MODELS {
                names.files(
                    &content_dir.join(format.content_dir),
                    archive_name(aid),
                    &["model"],
                )?
            } else if aid == archive::VORBIS {
                names.files(
                    &content_dir.join(format.content_dir),
                    "vorbis",
                    &["vorbis", "sample"],
                )?
            } else if aid == archive::DLLS {
                names.files(
                    &content_dir.join(format.content_dir),
                    "dlls",
                    &["dll", "dylib", "so"],
                )?
            } else {
                Vec::new()
            };

            if group_files.is_empty() {
                insert_empty_archive(&mut cache, aid, format, versions)?;
                return Ok(cache);
            }

            if aid == archive::BILLBOARDS {
                let group = pack_billboards(&group_files, &names, versions)?;
                insert_archive(
                    &mut cache,
                    aid,
                    format,
                    versions,
                    vec![group],
                    "from billboard sources",
                )?;
                return Ok(cache);
            }

            let vorbis = if aid == archive::VORBIS {
                let path = &group_files
                    .iter()
                    .find(|(id, _)| *id == 0)
                    .context("Missing Vorbis setup source")?
                    .1;
                let data = vorbis_setup::pack(&fs::read_to_string(path)?, &names)?;
                Some((vorbis::Setup::new(&data, &names)?, data))
            } else {
                None
            };

            let name_hashes: HashMap<u32, i32> = if format.named {
                let gids: Vec<u32> = group_files.iter().map(|(gid, _)| *gid).collect();
                load_name_hashes(pack_dir, aid, &gids)
            } else {
                HashMap::new()
            };

            let prepared: Vec<PreparedGroup> = group_files
                .par_iter()
                .map(|(gid, path)| {
                    let mut file_ids = None;
                    let data = if asset::extension(aid).is_some() {
                        let entries = asset::pack_file(aid, *gid, path, &names)?;
                        file_ids = Some(entries.iter().map(|(id, _)| *id).collect::<Vec<_>>());
                        encode_group_entries(
                            &entries
                                .iter()
                                .map(|(id, data)| (*id, data.as_slice()))
                                .collect::<Vec<_>>(),
                        )
                    } else if aid == archive::MODELS {
                        model_source::pack(&fs::read_to_string(path)?, &names)?
                    } else if aid == archive::VORBIS {
                        let (setup, data) = vorbis.as_ref().unwrap();
                        if *gid == 0 {
                            data.clone()
                        } else {
                            vorbis::pack_sample(
                                &fs::read_to_string(path)?,
                                &fs::read(path.with_extension("ogg"))?,
                                setup,
                                &names,
                            )?
                        }
                    } else {
                        fs::read(path)?
                    };
                    let mut group = PreparedGroup::new(aid, *gid, &data, versions)?;
                    if let Some(ids) = file_ids {
                        group.set_files(ids);
                    }
                    group.name_hash = name_hashes.get(gid).copied().unwrap_or(-1);
                    Ok(group)
                })
                .collect::<Result<Vec<_>>>()?;

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

fn pack_billboards(
    files: &[(u32, PathBuf)],
    names: &AssetNames,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<PreparedGroup> {
    let mut entries = Vec::with_capacity(files.len());
    for (id, path) in files {
        let source = asset::pack_file(archive::BILLBOARDS, 0, path, names)?;
        ensure!(
            source.len() == 1 && source[0].0 == *id,
            "{} must contain only [{}]",
            path.display(),
            names.name("billboard", *id)
        );
        entries.extend(source);
    }
    let data = encode_group_entries(
        &entries
            .iter()
            .map(|(id, data)| (*id, data.as_slice()))
            .collect::<Vec<_>>(),
    );
    let mut group = PreparedGroup::new(archive::BILLBOARDS, 0, &data, versions)?;
    group.set_files(entries.iter().map(|(id, _)| *id));
    Ok(group)
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
    let compressed = compress_best(data, allow_uncompressed, allow_bzip2, forced)?;
    let crc = compute_group_crc(&compressed);
    let mut with_version = compressed;
    with_version.reserve_exact(2);
    with_version.extend_from_slice(&version.to_be_bytes());
    Ok((with_version, crc))
}

#[cfg(test)]
mod tests {
    use super::{pack_asset_archives, pack_billboards};
    use crate::archive::{self, ArchiveVersions};
    use crate::cache::js5::PackedCache;
    use crate::pack::names::AssetNames;
    use crate::unpack;
    use rs_io::js5::compression::decompress;
    use rs_io::js5::{Js5Store, encode_group_entries};
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use std::process;

    #[test]
    fn billboard_files_preserve_names_and_sparse_ids() {
        let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target");
        let dir = target.join(format!("billboard-files-{}", process::id()));
        let pack = dir.join("pack");
        let source = dir.join("billboards");
        fs::create_dir_all(&pack).unwrap();
        fs::create_dir_all(&source).unwrap();
        let scripts = dir.join("scripts/clientscript");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(scripts.join("engine.cs2"), "").unwrap();
        fs::write(pack.join("clientscript.pack"), "").unwrap();
        fs::write(pack.join("cs2_command.pack"), "").unwrap();
        fs::write(pack.join("billboard.pack"), "2=small\n9=large\n").unwrap();
        let small = "[small]\nopcode=material\nmaterial=texture_7\nopcode=end\n\n";
        let large = "[large]\nopcode=size\nsize_minus_one=100,200\nopcode=end\n\n";
        fs::write(source.join("small.billboard"), small).unwrap();
        fs::write(source.join("large.billboard"), large).unwrap();
        let names = AssetNames::load(&pack).unwrap();
        let files = names.files(&source, "billboard", &["billboard"]).unwrap();
        let aid = archive::BILLBOARDS;
        let versions = HashMap::from([(
            aid,
            ArchiveVersions {
                table_version: 57,
                group_versions: HashMap::from([(0, 57)]),
                group_file_ids: HashMap::new(),
            },
        )]);
        let group = pack_billboards(&files, &names, &versions).unwrap();
        assert_eq!(group.id, 0);
        assert_eq!(group.version, 57);
        assert_eq!(group.files.iter().map(|f| f.id).collect::<Vec<_>>(), [2, 9]);
        assert_eq!(
            decompress(&group.raw).unwrap().as_ref(),
            encode_group_entries(&[(2, &[1, 0, 7, 0]), (9, &[2, 0, 100, 0, 200, 0])])
        );

        let mut cache = PackedCache::default();
        pack_asset_archives(&mut cache, &dir, &pack, &versions).unwrap();
        assert_eq!(cache.tables[&aid].groups.len(), 1);
        assert_eq!(cache.tables[&aid].groups[0].id, 0);
        assert_eq!(cache.groups[&(aid, 0)], group.raw);
        let mut store = Js5Store::create(dir.join("cache"), &[aid]).unwrap();
        store
            .write_raw_group(aid, 0, &cache.groups[&(aid, 0)])
            .unwrap();
        store
            .write_reference_table(aid, &cache.tables[&aid])
            .unwrap();
        let output = dir.join("unpacked");
        unpack::asset::unpack_archive(&mut store, aid, &pack, &output).unwrap();
        let output = output.join("billboards");
        assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
        assert_eq!(
            fs::read_to_string(output.join("small.billboard")).unwrap(),
            small
        );
        assert_eq!(
            fs::read_to_string(output.join("large.billboard")).unwrap(),
            large
        );
        let files = names.files(&output, "billboard", &["billboard"]).unwrap();
        let rebuilt = pack_billboards(&files, &names, &versions).unwrap();
        assert_eq!(rebuilt.raw, cache.groups[&(aid, 0)]);

        fs::write(output.join("small.billboard"), format!("{small}{large}")).unwrap();
        assert!(pack_billboards(&files, &names, &versions).is_err());
        fs::write(output.join("small.billboard"), large).unwrap();
        assert!(pack_billboards(&files, &names, &versions).is_err());

        drop(store);
        assert!(
            dir.canonicalize()
                .unwrap()
                .starts_with(target.canonicalize().unwrap())
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
