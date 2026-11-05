//! Unpack animation frames (archive 0) and skeleton bases (archive 1)
//! from the JS5 cache to content source files.
//!
//! Produces:
//! - `content/scripts/_unpack/all.base` — one line per skeleton base, keyed by name from base.pack
//! - `content/scripts/_unpack/all.anim` — one line per frame, keyed by anim name + frame index
//!
//! Names come from pack registries. Group/file IDs never appear in content files.

use std::io::Write;
use std::path::Path;

use crate::cache::seq_frame::{AnimFrame, SkeletonBase};
use crate::pack_registry::PackFile;
use anyhow::Result;
use rs_dat2::CacheStore;
use rs_dat2::archive;
use tracing::info;

fn load_or_create_pack(pack_dir: &Path, name: &str) -> PackFile {
    let path = pack_dir.join(format!("{}.pack", name));
    if path.exists() {
        PackFile::load(&path).unwrap_or_else(|_| PackFile::empty())
    } else {
        PackFile::empty()
    }
}

fn name_or_fallback(pack: &PackFile, id: u32, prefix: &str) -> String {
    pack.get_by_id(id)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}_{}", prefix, id))
}

pub fn unpack_bases(store: &mut CacheStore, pack_dir: &Path, output_dir: &Path) -> Result<u32> {
    let group_count = store.group_count(archive::BASES).unwrap_or(0);
    if group_count == 0 {
        return Ok(0);
    }

    let pack = load_or_create_pack(pack_dir, "base");
    std::fs::create_dir_all(output_dir)?;
    let mut file = std::fs::File::create(output_dir.join("all.base"))?;
    writeln!(file, "# name bone_count [type:parent:label:groups ...]")?;
    let mut total = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(archive::BASES, gid)? else {
            continue;
        };
        for entry in &group.entries {
            if entry.data.is_empty() {
                continue;
            }
            let base = SkeletonBase::decode(&entry.data);
            let name = name_or_fallback(&pack, gid, "base");
            let mut line = format!("{} {}", name, base.bone_count);
            for i in 0..base.bone_count as usize {
                let p = if base.parent_flags[i] { 1 } else { 0 };
                let grps: Vec<String> =
                    base.label_groups[i].iter().map(|g| g.to_string()).collect();
                let grp_str = if grps.is_empty() {
                    "-".to_string()
                } else {
                    grps.join(";")
                };
                line.push_str(&format!(
                    " {}:{}:{}:{}",
                    base.transform_types[i], p, base.labels[i], grp_str
                ));
            }
            writeln!(file, "{}", line)?;
            total += 1;
        }
    }

    info!("  {} skeleton bases unpacked", total);
    Ok(total)
}

pub fn unpack_anims(store: &mut CacheStore, pack_dir: &Path, output_dir: &Path) -> Result<u32> {
    let group_count = store.group_count(archive::ANIMS).unwrap_or(0);
    if group_count == 0 {
        return Ok(0);
    }

    let pack = load_or_create_pack(pack_dir, "anim");
    let anim_dir = output_dir.join("_unpack").join("animations");
    std::fs::create_dir_all(&anim_dir)?;
    let mut total = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(archive::ANIMS, gid)? else {
            continue;
        };
        let anim_name = name_or_fallback(&pack, gid, "anim");
        let file_path = anim_dir.join(format!("{anim_name}.anim"));
        let mut file = std::fs::File::create(&file_path)?;

        for entry in &group.entries {
            if entry.data.is_empty() {
                continue;
            }
            let frame = AnimFrame::decode(&entry.data);
            writeln!(file, "[frame_{}]", entry.id)?;
            writeln!(file, "type={}", frame.type_marker)?;
            writeln!(file, "base={}", frame.base_id)?;
            writeln!(file, "bone_count={}", frame.bone_count)?;
            for bone in &frame.bones {
                writeln!(
                    file,
                    "bone={}:{}:{}:{}:{}",
                    bone.bone_index, bone.mask, bone.dx, bone.dy, bone.dz
                )?;
            }
            writeln!(file)?;
            total += 1;
        }
    }

    info!(
        "  {} animation frames unpacked to {}",
        total,
        anim_dir.display()
    );
    Ok(total)
}
