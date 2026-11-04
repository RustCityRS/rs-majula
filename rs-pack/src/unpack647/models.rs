//! Unpack model archive (archive 7) into categorized `.ob2` files.
//!
//! Models are extracted as raw decompressed binary blobs and categorized
//! into subdirectories based on their suffix in `model.pack`:
//!
//!   content/models/_unpack/
//!     loc/            — _loc suffix (scenery/location models)
//!     npc/            — _npc suffix (NPC body models)
//!     npc_head/       — _npc_head suffix (NPC chathead models)
//!     obj/            — _obj suffix (item ground models)
//!     obj_wear/       — _obj_wear suffix (item equipped models)
//!     obj_head/       — _obj_head suffix (item chathead models)
//!     spotanim/       — _spotanim suffix (spot animation models)
//!     idk/            — _idk suffix (identity kit body models)
//!     idk_head/       — _idk_head suffix (identity kit chathead models)
//!     uncategorized/  — no recognized suffix
//!     manifest.json   — per-group metadata for repack
//!
//! Mirrors 2004scape's `content/models/` layout. File names use the
//! registry name + `.ob2` extension (matching 2004scape convention).

use std::path::Path;

use anyhow::{Context, Result};
use rs_dat2::CacheStore;
use rs_dat2::compression;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::pack_registry::PackRegistry;

const ARCHIVE_MODELS: u8 = 7;

const CATEGORY_SUFFIXES: &[(&str, &str)] = &[
    ("_npc_head", "npc_head"),
    ("_obj_wear", "obj_wear"),
    ("_obj_head", "obj_head"),
    ("_idk_head", "idk_head"),
    ("_spotanim", "spotanim"),
    ("_npc", "npc"),
    ("_obj", "obj"),
    ("_loc", "loc"),
    ("_idk", "idk"),
];

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelManifest {
    pub archive_id: u8,
    pub groups: Vec<ModelEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelEntry {
    pub group_id: u32,
    pub name: String,
    pub category: String,
    pub compression: u8,
    pub version: u16,
    pub decompressed_len: u32,
}

fn categorize(name: &str) -> &'static str {
    for &(suffix, category) in CATEGORY_SUFFIXES {
        if name.ends_with(suffix) {
            return category;
        }
    }
    "uncategorized"
}

pub fn unpack_models(
    cache_dir: &Path,
    registry: &PackRegistry,
    output_dir: &Path,
) -> Result<UnpackModelStats> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_MODELS).unwrap_or(0);

    // Create category directories
    let categories = [
        "loc",
        "npc",
        "npc_head",
        "obj",
        "obj_wear",
        "obj_head",
        "spotanim",
        "idk",
        "idk_head",
        "uncategorized",
    ];
    for cat in &categories {
        std::fs::create_dir_all(output_dir.join(cat))?;
    }

    let mut manifest = ModelManifest {
        archive_id: ARCHIVE_MODELS,
        groups: Vec::with_capacity(group_count as usize),
    };
    let mut stats = UnpackModelStats::default();

    for gid in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(ARCHIVE_MODELS, gid)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let comp_type = raw[0];
        let compressed_len = u32::from_be_bytes([raw[1], raw[2], raw[3], raw[4]]) as usize;
        let header_size = if comp_type == 0 { 5 } else { 9 };
        let container_end = header_size + compressed_len;
        let version = if raw.len() >= container_end + 2 {
            u16::from_be_bytes([raw[container_end], raw[container_end + 1]])
        } else {
            0
        };

        let decompressed = match compression::decompress(&raw) {
            Ok(d) => d,
            Err(_) => {
                stats.failed += 1;
                continue;
            }
        };

        let name = registry
            .model
            .get_by_id(gid)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("model_{}", gid));

        let category = categorize(&name);
        let file_path = output_dir.join(category).join(format!("{name}.ob2"));
        std::fs::write(&file_path, &decompressed)?;

        manifest.groups.push(ModelEntry {
            group_id: gid,
            name: name.clone(),
            category: category.to_string(),
            compression: comp_type,
            version,
            decompressed_len: decompressed.len() as u32,
        });

        match category {
            "loc" => stats.loc += 1,
            "npc" => stats.npc += 1,
            "npc_head" => stats.npc_head += 1,
            "obj" => stats.obj += 1,
            "obj_wear" => stats.obj_wear += 1,
            "obj_head" => stats.obj_head += 1,
            "spotanim" => stats.spotanim += 1,
            "idk" => stats.idk += 1,
            "idk_head" => stats.idk_head += 1,
            _ => stats.uncategorized += 1,
        }

        if (stats.total() % 10000) == 0 && stats.total() > 0 {
            info!("  {} models unpacked...", stats.total());
        }
    }

    let manifest_path = output_dir.join("manifest.json");
    let json = serde_json::to_string_pretty(&manifest).context("serialize manifest")?;
    std::fs::write(&manifest_path, json)?;

    Ok(stats)
}

#[derive(Debug, Default)]
pub struct UnpackModelStats {
    pub loc: u32,
    pub npc: u32,
    pub npc_head: u32,
    pub obj: u32,
    pub obj_wear: u32,
    pub obj_head: u32,
    pub spotanim: u32,
    pub idk: u32,
    pub idk_head: u32,
    pub uncategorized: u32,
    pub failed: u32,
}

impl UnpackModelStats {
    pub fn total(&self) -> u32 {
        self.loc
            + self.npc
            + self.npc_head
            + self.obj
            + self.obj_wear
            + self.obj_head
            + self.spotanim
            + self.idk
            + self.idk_head
            + self.uncategorized
    }
}
