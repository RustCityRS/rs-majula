use crate::archive;
use anyhow::Result;
use rs_io::js5::Js5Store;
use rs_io::js5::compression::decompress;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Unpack a single binary archive to `content/{dir}/_unpack/{gid}.{ext}`.
pub fn unpack_binary_archive(
    store: &mut Js5Store,
    archive_id: u8,
    content_dir: &Path,
) -> Result<u32> {
    let format = archive::archive_format(archive_id)
        .ok_or_else(|| anyhow::anyhow!("unknown archive {}", archive_id))?;

    let ext = archive_extension(archive_id);
    let archive_dir = content_dir.join(format.content_dir).join("_unpack");
    fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(archive_id).unwrap_or(0);
    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(archive_id, gid)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let decompressed = match decompress(&raw) {
            Ok(d) => d,
            Err(e) => {
                warn!(
                    "archive {} group {}: decompress failed ({}), skipping",
                    archive_id, gid, e
                );
                continue;
            }
        };

        let path = archive_dir.join(format!("{}.{}", gid, ext));
        fs::write(&path, &decompressed)?;
        written += 1;
    }

    Ok(written)
}

/// File extension for each archive's unpacked groups.
fn archive_extension(archive_id: u8) -> &'static str {
    match archive_id {
        archive::ANIMS => "anim",
        archive::BASES => "skel",
        archive::SYNTH => "synth",
        archive::BINARY => "huffman",
        archive::JINGLES => "synth",
        archive::VORBIS => "vorbis",
        archive::MUSIC_SAMPLES => "sample",
        archive::DLLS => "dll",
        _ => "bin",
    }
}

pub fn unpack_all_binary_assets(store: &mut Js5Store, content_dir: &Path) -> Result<u32> {
    let mut total = 0u32;
    for &aid in &[
        archive::ANIMS,
        archive::BASES,
        // Models and clientscripts pack from their raw groups; the decoded
        // `.ob2` models and `.cs2` disassembly beside them are for reading.
        archive::MODELS,
        archive::CLIENTSCRIPT,
        archive::SYNTH,
        archive::BINARY,
        archive::JINGLES,
        archive::FONTMETRICS,
        archive::VORBIS,
        archive::MUSIC_SAMPLES,
        archive::WORLDMAP_DATA,
        archive::MATERIALS,
        archive::PARTICLES,
        archive::DEFAULTS,
        archive::BILLBOARDS,
        archive::DLLS,
        archive::LOADING_SCREENS,
        archive::WORLDMAP_LOCS,
    ] {
        if !store.has_archive(aid) {
            continue;
        }
        let count = store.group_count(aid).unwrap_or(0);
        if count == 0 {
            let format = archive::archive_format(aid);
            let name = format.map(|f| f.content_dir).unwrap_or("?");
            info!("  {} (archive {}): empty, skipping", name, aid);
            continue;
        }
        let format = archive::archive_format(aid);
        let name = format.map(|f| f.content_dir).unwrap_or("?");
        info!("  Unpacking {} (archive {})...", name, aid);
        let n = unpack_binary_archive(store, aid, content_dir)?;
        info!("    {} groups", n);
        total += n;
    }
    Ok(total)
}
