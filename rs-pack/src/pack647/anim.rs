#![allow(clippy::map_identity)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Pack animation frames and skeleton bases from text source to binary.
//!
//! Reads `all.base` and `all.anim` keyed by name (from pack registries).
//! Resolves names → group IDs via `base.pack` and `anim.pack`.
//! Frame indices in the text file become file IDs in the group.

use std::collections::BTreeMap;
use std::path::Path;

use crate::cache647::anim_frame::{AnimFrame, FrameBone, SkeletonBase};
use anyhow::{Context, Result};
use tracing::info;

use crate::pack_registry::PackFile;

fn load_pack(pack_dir: &Path, name: &str) -> PackFile {
    let path = pack_dir.join(format!("{}.pack", name));
    if path.exists() {
        PackFile::load(&path).unwrap_or_else(|_| PackFile::empty())
    } else {
        PackFile::empty()
    }
}

/// Parse `all.base` → map of group_id → encoded binary bytes.
pub fn pack_bases(source_dir: &Path, pack_dir: &Path) -> Result<BTreeMap<u32, Vec<u8>>> {
    let path = source_dir.join("_unpack").join("all.base");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let pack = load_pack(pack_dir, "base");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;

    let mut result: BTreeMap<u32, Vec<u8>> = BTreeMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap_or("");
        let group_id = pack
            .get_by_name(name)
            .map(|id| id)
            .or_else(|| name.strip_prefix("base_").and_then(|s| s.parse().ok()))
            .unwrap_or(0);
        let bone_count: u8 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

        let mut transform_types = Vec::new();
        let mut parent_flags = Vec::new();
        let mut labels = Vec::new();
        let mut label_groups = Vec::new();

        for bone_str in parts {
            let fields: Vec<&str> = bone_str.split(':').collect();
            if fields.len() < 4 {
                continue;
            }
            transform_types.push(fields[0].parse().unwrap_or(0));
            parent_flags.push(fields[1] == "1");
            labels.push(fields[2].parse().unwrap_or(0));
            let grps = if fields[3] == "-" {
                Vec::new()
            } else {
                fields[3]
                    .split(';')
                    .filter_map(|s| s.parse().ok())
                    .collect()
            };
            label_groups.push(grps);
        }

        let base = SkeletonBase {
            bone_count,
            transform_types,
            parent_flags,
            labels,
            label_groups,
        };
        result.insert(group_id, base.encode());
    }

    info!("  {} skeleton bases packed", result.len());
    Ok(result)
}

/// Parse per-animation files from `_unpack/animations/` → map of (group_id, file_id) → encoded binary bytes.
/// Each file is named `{anim_name}.anim` with `[frame_N]` sections containing key=value properties.
pub fn pack_anims(source_dir: &Path, pack_dir: &Path) -> Result<BTreeMap<(u32, u32), Vec<u8>>> {
    let anim_dir = source_dir.join("_unpack").join("animations");
    if anim_dir.is_dir() {
        return pack_anims_from_dir(&anim_dir, pack_dir);
    }
    Ok(BTreeMap::new())
}

fn pack_anims_from_dir(anim_dir: &Path, pack_dir: &Path) -> Result<BTreeMap<(u32, u32), Vec<u8>>> {
    let pack = load_pack(pack_dir, "anim");
    let mut result: BTreeMap<(u32, u32), Vec<u8>> = BTreeMap::new();

    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(anim_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("anim"))
        .collect();
    files.sort();

    for path in &files {
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let group_id = pack
            .get_by_name(name)
            .map(|id| id)
            .or_else(|| name.strip_prefix("anim_").and_then(|s| s.parse().ok()))
            .unwrap_or(0);

        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

        let mut frame_index: u32 = 0;
        let mut type_marker: u8 = 0;
        let mut base_id: u16 = 0;
        let mut bone_count: u8 = 0;
        let mut bones: Vec<FrameBone> = Vec::new();
        let mut in_frame = false;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                if in_frame {
                    let frame = AnimFrame {
                        type_marker,
                        base_id,
                        bone_count,
                        bones: std::mem::take(&mut bones),
                    };
                    result.insert((group_id, frame_index), frame.encode());
                    in_frame = false;
                }
                continue;
            }
            if let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                if in_frame {
                    let frame = AnimFrame {
                        type_marker,
                        base_id,
                        bone_count,
                        bones: std::mem::take(&mut bones),
                    };
                    result.insert((group_id, frame_index), frame.encode());
                }
                frame_index = inner
                    .strip_prefix("frame_")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                type_marker = 0;
                base_id = 0;
                bone_count = 0;
                in_frame = true;
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "type" => type_marker = value.trim().parse().unwrap_or(0),
                    "base" => base_id = value.trim().parse().unwrap_or(0),
                    "bone_count" => bone_count = value.trim().parse().unwrap_or(0),
                    "bone" => {
                        let fields: Vec<&str> = value.trim().split(':').collect();
                        if fields.len() >= 5 {
                            bones.push(FrameBone {
                                bone_index: fields[0].parse().unwrap_or(0),
                                mask: fields[1].parse().unwrap_or(0),
                                dx: fields[2].parse().unwrap_or(0),
                                dy: fields[3].parse().unwrap_or(0),
                                dz: fields[4].parse().unwrap_or(0),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        if in_frame {
            let frame = AnimFrame {
                type_marker,
                base_id,
                bone_count,
                bones,
            };
            result.insert((group_id, frame_index), frame.encode());
        }
    }

    info!(
        "  {} animation frames packed from {} ({} files)",
        result.len(),
        anim_dir.display(),
        files.len()
    );
    Ok(result)
}

/// Pack `all.anim` text → 3-stripe encoded group blobs ready for JS5.
/// Returns map of group_id → encoded group blob (pre-compression).
pub fn pack_anim_groups(source_dir: &Path, pack_dir: &Path) -> Result<BTreeMap<u32, Vec<u8>>> {
    use crate::cache647::anim_frame::split_frame_into_stripes;

    let entries = pack_anims(source_dir, pack_dir)?;
    if entries.is_empty() {
        return Ok(BTreeMap::new());
    }

    // Group entries by group_id
    let mut groups: BTreeMap<u32, Vec<(u32, Vec<u8>)>> = BTreeMap::new();
    for ((gid, fid), data) in entries {
        groups.entry(gid).or_default().push((fid, data));
    }

    let mut result = BTreeMap::new();
    for (gid, mut frames) in groups {
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
        result.insert(gid, blob);
    }

    info!("  {} animation groups assembled", result.len());
    Ok(result)
}
