//! Unpack shaders (archive 31) to source files.
//!
//! Group 1 contains ARB vertex/fragment programs (plain text).
//! Group 3 contains compiled DirectX shaders (binary CTAB format).

use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use tracing::warn;

use crate::asset_roundtrip::{AssetManifest, GroupEntry};

const ARCHIVE_SHADERS: u8 = 31;

pub fn unpack_shaders(store: &mut CacheStore, content_dir: &Path) -> Result<u32> {
    let archive_dir = content_dir.join("shaders").join("_unpack");
    std::fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(ARCHIVE_SHADERS).unwrap_or(0);
    let mut manifest = AssetManifest {
        archive_id: ARCHIVE_SHADERS,
        archive_name: "shaders".to_string(),
        file_extension: "bin".to_string(),
        groups: Vec::new(),
    };
    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(ARCHIVE_SHADERS, gid)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let comp_type = raw[0];
        let compressed_len = u32::from_be_bytes([raw[1], raw[2], raw[3], raw[4]]) as usize;
        let header_size = if comp_type == 0 { 5 } else { 9 };
        let decompressed_len = if comp_type != 0 && raw.len() >= 9 {
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
                    ARCHIVE_SHADERS, gid, e
                );
                continue;
            }
        };

        // Detect if content is text (ARB programs start with "!!")
        let ext = if decompressed.starts_with(b"!!") {
            "glsl"
        } else if decompressed.starts_with(&[0x01, 0x01])
            || decompressed.len() > 4 && &decompressed[4..8] == b"CTAB"
        {
            "cso"
        } else {
            "bin"
        };

        let path = archive_dir.join(format!("{}.{}", gid, ext));
        std::fs::write(&path, &decompressed)?;

        manifest.groups.push(GroupEntry {
            group_id: gid,
            file_name: None,
            file_extension: Some(ext.to_string()),
            compression: comp_type,
            version,
            compressed_len: compressed_len as u32,
            decompressed_len,
            crc,
            name_hash: None,
            file_ids: None,
        });
        written += 1;
    }

    let manifest_path = archive_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    Ok(written)
}
