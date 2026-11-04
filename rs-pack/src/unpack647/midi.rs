//! Unpack Jagex MIDI archives to standard MIDI files.
//!
//! Decoder logic lives in `crate::repack::midi::decode_jagex_midi`.

use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use tracing::{info, warn};

use crate::asset_roundtrip::{AssetManifest, GroupEntry};
use crate::repack::midi::decode_jagex_midi;

const ARCHIVE_SONGS: u8 = 6;

pub fn unpack_songs(store: &mut CacheStore, pack_dir: &Path, content_dir: &Path) -> Result<u32> {
    use crate::pack_registry::PackFile;
    let song_pack = PackFile::load(&pack_dir.join("song.pack")).ok();
    unpack_midi_archive(
        store,
        ARCHIVE_SONGS,
        "songs",
        "mid",
        content_dir,
        song_pack.as_ref(),
    )
}

fn unpack_midi_archive(
    store: &mut CacheStore,
    archive_id: u8,
    dir_name: &str,
    extension: &str,
    content_dir: &Path,
    pack: Option<&crate::pack_registry::PackFile>,
) -> Result<u32> {
    let archive_dir = content_dir.join(dir_name).join("_unpack");
    std::fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(archive_id).unwrap_or(0);
    let mut manifest = AssetManifest {
        archive_id,
        archive_name: dir_name.to_string(),
        file_extension: extension.to_string(),
        groups: Vec::new(),
    };

    let mut written = 0u32;
    let mut failed = 0u32;

    for gid in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(archive_id, gid)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let comp_type = raw[0];
        let compressed_len = u32::from_be_bytes([raw[1], raw[2], raw[3], raw[4]]) as usize;
        let header_size = if comp_type == 0 { 5 } else { 9 };
        let decompressed_len_raw = if comp_type != 0 && raw.len() >= 9 {
            u32::from_be_bytes([raw[5], raw[6], raw[7], raw[8]])
        } else {
            compressed_len as u32
        };
        let container_end = header_size + compressed_len;
        let version = if raw.len() >= container_end + 2 {
            u16::from_be_bytes([raw[container_end], raw[container_end + 1]])
        } else {
            0
        };
        let crc = rs_io::crc::getcrc(&raw, 0, container_end.min(raw.len())) as u32;

        let decompressed = match rs_dat2::compression::decompress(&raw) {
            Ok(d) => d,
            Err(e) => {
                warn!(
                    "archive {} group {}: decompress failed ({})",
                    archive_id, gid, e
                );
                failed += 1;
                continue;
            }
        };

        let midi_bytes = match decode_jagex_midi(&decompressed) {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    "archive {} group {}: MIDI decode failed ({})",
                    archive_id, gid, e
                );
                failed += 1;
                continue;
            }
        };

        let name = pack
            .and_then(|p| p.get_by_id(gid).map(|s| s.to_string()))
            .unwrap_or_else(|| gid.to_string());
        let path = archive_dir.join(format!("{}.{}", name, extension));
        std::fs::write(&path, &midi_bytes)?;

        manifest.groups.push(GroupEntry {
            group_id: gid,
            file_name: Some(format!("{}.{}", name, extension)),
            file_extension: None,
            compression: comp_type,
            version,
            compressed_len: compressed_len as u32,
            decompressed_len: decompressed_len_raw,
            crc,
            name_hash: None,
            file_ids: None,
        });
        written += 1;
    }

    let manifest_path = archive_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    if failed > 0 {
        warn!("  {} groups failed to decode", failed);
    }
    info!("  {} {} decoded to standard MIDI", written, dir_name);
    Ok(written)
}
