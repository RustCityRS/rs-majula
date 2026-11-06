use super::{AssetManifest, GroupEntry};
use crate::archive;
use anyhow::Result;
use rs_io::js5::Js5Store;
use rs_io::js5::compression::{Container, decompress};
use std::fs;
use std::path::Path;
use tracing::warn;

pub fn unpack_shaders(store: &mut Js5Store, content_dir: &Path) -> Result<u32> {
    let archive_dir = content_dir.join("shaders").join("_unpack");
    fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(archive::SHADERS).unwrap_or(0);
    let mut manifest = AssetManifest {
        archive_id: archive::SHADERS,
        archive_name: "shaders".to_string(),
        file_extension: "bin".to_string(),
        groups: Vec::new(),
    };
    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(archive::SHADERS, gid)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let container = Container::parse(&raw)?;

        let decompressed = match decompress(&raw) {
            Ok(d) => d,
            Err(e) => {
                warn!(
                    "archive {} group {}: decompress failed ({})",
                    archive::SHADERS,
                    gid,
                    e
                );
                continue;
            }
        };

        // Detect if content is text (ARB programs start with "!!")
        let ext = if decompressed.starts_with(b"!!") {
            "glsl"
        } else if decompressed.starts_with(&[0x01, 0x01]) || decompressed.get(4..8) == Some(b"CTAB")
        {
            "cso"
        } else {
            "bin"
        };

        let path = archive_dir.join(format!("{}.{}", gid, ext));
        fs::write(&path, &decompressed)?;

        manifest.groups.push(GroupEntry {
            group_id: gid,
            file_name: None,
            file_extension: Some(ext.to_string()),
            compression: container.compression as u8,
            version: container.version.unwrap_or(0),
            compressed_len: container.compressed_len,
            decompressed_len: container.uncompressed_len,
            crc: container.crc(),
        });
        written += 1;
    }

    let manifest_path = archive_dir.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    Ok(written)
}
