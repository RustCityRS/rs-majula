use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::pack::other::map::encode_jm2;
use crate::pack::util::walk;
use crate::pack::versionlist::build_version_list;
use crate::types::OndemandBlobs;
use crate::unpack::model;
use crate::versionlist::VersionListMeta;
use rs_io::js5::Js5Store;
#[cfg(before_274)]
use rs_io::js5::js5zip;
use tracing::{debug, warn};

pub struct OndemandArtifacts {
    pub version_list: Vec<u8>,
    #[cfg(before_274)]
    pub zip: Vec<u8>,
    pub blobs: OndemandBlobs,
}

fn stem_paths(dir: &Path, ext: &str) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    for path in walk(dir) {
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            map.insert(stem.to_string(), path);
        }
    }
    map
}

fn warn_missing_referenced_models(
    model_names: &HashMap<u16, String>,
    ob2: &HashMap<String, PathBuf>,
) {
    let mut missing: Vec<(u16, &str)> = model_names
        .iter()
        .filter(|&(&id, name)| *name != format!("model_{id}") && !ob2.contains_key(name.as_str()))
        .map(|(&id, name)| (id, name.as_str()))
        .collect();
    missing.sort_unstable();
    for (_, name) in &missing {
        warn!(
            "packing: \"{name}\" is referenced by a config but has no .ob2 in content; cache will omit it"
        );
    }
}

fn stage(
    bulk: &mut Js5Store,
    index: usize,
    versions: &[u16],
    absent: &[i32],
    label: &str,
    warn_missing: bool,
    mut resolve: impl FnMut(usize) -> Option<Vec<u8>>,
) {
    for (id, &version) in versions.iter().enumerate() {
        if absent.get(id).copied().unwrap_or(0) != 0 {
            continue;
        }
        match resolve(id) {
            Some(data) => bulk.write_compressed(index, id, &data, version),
            None if warn_missing && version != 0 => warn!(
                "packing: {label} {id} is in the version list but missing from content; cache will omit it"
            ),
            None => {}
        }
    }
    bulk.ensure_file_count(index, versions.len());
    debug!("  idx{index}: {} {label} slots", versions.len());
}

fn stage_bulk(
    content_dir: &Path,
    pack_dir: &Path,
    meta: &VersionListMeta,
    count: usize,
) -> (Js5Store, Vec<bool>) {
    let mut bulk = Js5Store::create(count);

    // idx1: models (.ob2), resolved through their pack name.
    let model_names = model::load_existing_pack(pack_dir, "model");
    let ob2 = stem_paths(&content_dir.join("models"), "ob2");
    stage(
        &mut bulk,
        1,
        &meta.model_version,
        &meta.model_crc,
        "model",
        false,
        |id| {
            let name = model_names.get(&(id as u16))?;
            std::fs::read(ob2.get(name)?).ok()
        },
    );
    warn_missing_referenced_models(&model_names, &ob2);

    // idx2: anims (.anim), named directly by id.
    let anim_dir = content_dir.join("models").join("anim");
    stage(
        &mut bulk,
        2,
        &meta.anim_version,
        &meta.anim_crc,
        "anim",
        true,
        |id| std::fs::read(anim_dir.join(format!("anim_{id}.anim"))).ok(),
    );

    // idx3: midi (.mid under songs/ or jingles/); record which ids are jingles.
    let midi_names = model::load_existing_pack(pack_dir, "midi");
    let songs = stem_paths(&content_dir.join("songs"), "mid");
    let jingles = stem_paths(&content_dir.join("jingles"), "mid");
    let mut midi_jingles = vec![false; meta.midi_version.len()];
    stage(
        &mut bulk,
        3,
        &meta.midi_version,
        &meta.midi_crc,
        "midi",
        true,
        |id| {
            let name = midi_names.get(&(id as u16))?;
            let jingle = jingles.get(name);
            midi_jingles[id] = jingle.is_some();
            std::fs::read(songs.get(name).or(jingle)?).ok()
        },
    );

    // idx4: maps. Each .jm2 square encodes to a land + loc blob, placed at its
    // land_file / loc_file ids so the dat lands in ascending file-id order.
    let maps_dir = content_dir.join("maps");
    let mut maps: Vec<Option<Vec<u8>>> = vec![None; meta.map_version.len()];
    for entry in &meta.maps {
        let path = maps_dir.join(format!("m{}_{}.jm2", entry.map_x(), entry.map_z()));
        let Ok(jm2) = std::fs::read(&path) else {
            warn!(
                "packing: map m{}_{} is in the version list but missing from content; cache will omit it",
                entry.map_x(),
                entry.map_z()
            );
            continue;
        };
        let (land, loc) = encode_jm2(&jm2);
        if meta
            .map_crc
            .get(entry.land_file as usize)
            .copied()
            .unwrap_or(0)
            == 0
            && let Some(slot) = maps.get_mut(entry.land_file as usize)
        {
            *slot = Some(land);
        }
        if meta
            .map_crc
            .get(entry.loc_file as usize)
            .copied()
            .unwrap_or(0)
            == 0
            && let Some(slot) = maps.get_mut(entry.loc_file as usize)
        {
            *slot = Some(loc);
        }
    }
    for (id, blob) in maps.iter().enumerate() {
        if let Some(blob) = blob {
            bulk.write_compressed(4, id, blob, meta.map_version[id]);
        }
    }
    bulk.ensure_file_count(4, meta.map_version.len());
    debug!("  idx4: {} map squares", meta.maps.len());

    (bulk, midi_jingles)
}

pub fn build_ondemand_artifacts(
    content_dir: &Path,
    pack_dir: &Path,
    count: usize,
) -> OndemandArtifacts {
    let meta = VersionListMeta::read(&pack_dir.join("version_list"));
    let (bulk, midi_jingles) = stage_bulk(content_dir, pack_dir, &meta, count);

    let blobs = (1..count)
        .map(|index| {
            (0..bulk.count(index))
                .map(|id| {
                    bulk.read(index, id, false)
                        .unwrap_or_default()
                        .into_boxed_slice()
                })
                .collect()
        })
        .collect();

    OndemandArtifacts {
        version_list: build_version_list(&bulk, &meta, &midi_jingles),
        #[cfg(before_274)]
        zip: js5zip(&bulk, count),
        blobs,
    }
}
