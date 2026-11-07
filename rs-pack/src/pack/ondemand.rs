use crate::pack::other::map::encode_jm2;
use crate::pack::util::walk;
use crate::pack::versionlist::build_version_list;
use crate::types::OndemandBlobs;
use crate::unpack::model;
use crate::versionlist::VersionListMeta;
use rs_io::gz::gz_compress;
#[cfg(before_274)]
use rs_io::zip::fflate;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
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
    versions: &[u16],
    absent: &[i32],
    label: &str,
    warn_missing: bool,
    mut resolve: impl FnMut(usize) -> Option<Vec<u8>>,
) -> Vec<Box<[u8]>> {
    versions.iter().enumerate().map(|(id, &version)| {
        if absent.get(id).copied().unwrap_or(0) != 0 {
            return Box::default();
        }
        let Some(data) = resolve(id) else {
            if warn_missing && version != 0 {
                warn!("packing: {label} {id} is in the version list but missing from content; cache will omit it");
            }
            return Box::default();
        };
        let mut blob = gz_compress(&data);
        if version != 0 {
            blob.extend_from_slice(&version.to_be_bytes());
        }
        blob.into_boxed_slice()
    }).collect()
}

fn stage_bulk(
    content_dir: &Path,
    pack_dir: &Path,
    meta: &VersionListMeta,
) -> (OndemandBlobs, Vec<bool>) {
    // idx1: models (.ob2), resolved through their pack name.
    let model_names = model::load_existing_pack(pack_dir, "model");
    let ob2 = stem_paths(&content_dir.join("models"), "ob2");
    let models = stage(&meta.model_version, &meta.model_crc, "model", false, |id| {
        let name = model_names.get(&(id as u16))?;
        fs::read(ob2.get(name)?).ok()
    });
    warn_missing_referenced_models(&model_names, &ob2);

    // idx2: anims (.anim), named directly by id.
    let anim_dir = content_dir.join("models").join("anim");
    let anims = stage(&meta.anim_version, &meta.anim_crc, "anim", true, |id| {
        fs::read(anim_dir.join(format!("anim_{id}.anim"))).ok()
    });

    // idx3: midi (.mid under songs/ or jingles/); record which ids are jingles.
    let midi_names = model::load_existing_pack(pack_dir, "midi");
    let songs = stem_paths(&content_dir.join("songs"), "mid");
    let jingles = stem_paths(&content_dir.join("jingles"), "mid");
    let mut midi_jingles = vec![false; meta.midi_version.len()];
    let midi = stage(&meta.midi_version, &meta.midi_crc, "midi", true, |id| {
        let name = midi_names.get(&(id as u16))?;
        let jingle = jingles.get(name);
        midi_jingles[id] = jingle.is_some();
        fs::read(songs.get(name).or(jingle)?).ok()
    });

    // idx4: maps. Each .jm2 square encodes to a land + loc blob, placed at its
    // land_file / loc_file ids so the dat lands in ascending file-id order.
    let maps_dir = content_dir.join("maps");
    let mut maps: Vec<Option<Vec<u8>>> = vec![None; meta.map_version.len()];
    for entry in &meta.maps {
        let path = maps_dir.join(format!("m{}_{}.jm2", entry.map_x(), entry.map_z()));
        let Ok(jm2) = fs::read(&path) else {
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
    let maps = stage(&meta.map_version, &meta.map_crc, "map", false, |id| {
        maps[id].take()
    });
    debug!("  idx4: {} map squares", meta.maps.len());
    (vec![models, anims, midi, maps], midi_jingles)
}

pub fn build_ondemand_artifacts(content_dir: &Path, pack_dir: &Path) -> OndemandArtifacts {
    let meta = VersionListMeta::read(&pack_dir.join("version_list"));
    let (blobs, midi_jingles) = stage_bulk(content_dir, pack_dir, &meta);

    OndemandArtifacts {
        version_list: build_version_list(&blobs, &meta, &midi_jingles),
        #[cfg(before_274)]
        zip: build_zip(&blobs),
        blobs,
    }
}

#[cfg(before_274)]
fn build_zip(blobs: &OndemandBlobs) -> Vec<u8> {
    fflate(blobs.iter().enumerate().flat_map(|(archive, groups)| {
        groups
            .iter()
            .enumerate()
            .filter(|(_, data)| !data.is_empty())
            .map(move |(id, data)| (format!("{}.{id}", archive + 1), data.as_ref()))
    }))
}
