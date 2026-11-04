//! Interface archive unpacker — cache archive 3 → text `.if` files.

use std::path::Path;

use anyhow::{Context, Result};
use rs_dat2::CacheStore;
use tracing::info;

use crate::interface_format::{self, INTERFACE_ARCHIVE, InterfaceGroupEntry, InterfaceManifest};

pub fn unpack_interfaces(store: &mut CacheStore, content_dir: &Path) -> Result<u32> {
    let if_dir = content_dir.join("interfaces").join("_unpack");
    std::fs::create_dir_all(&if_dir)?;

    let group_count = store.group_count(INTERFACE_ARCHIVE).unwrap_or(0);
    let mut manifest = InterfaceManifest {
        archive_id: INTERFACE_ARCHIVE,
        groups: Vec::new(),
    };
    let mut written = 0u32;

    for group_id in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(INTERFACE_ARCHIVE, group_id)? else {
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

        let Some(group) = store
            .read_group(INTERFACE_ARCHIVE, group_id)
            .with_context(|| format!("read interface group {}", group_id))?
        else {
            continue;
        };

        let mut text = String::new();
        let mut comp_count = 0u32;

        for entry in &group.entries {
            if entry.data.is_empty() {
                continue;
            }
            let comp = interface_format::parse_component(&entry.data, group_id);
            text.push_str(&interface_format::emit_component_text(&comp, entry.id));
            text.push('\n');
            comp_count += 1;
        }

        let file_name = format!("{}.if", group_id);
        std::fs::write(if_dir.join(&file_name), &text)?;

        manifest.groups.push(InterfaceGroupEntry {
            group_id,
            file_name,
            compression: comp_type,
            version,
            compressed_len: compressed_len as u32,
            decompressed_len,
            crc,
            component_count: comp_count,
        });
        written += 1;
    }

    let manifest_path = if_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    info!(
        "Interfaces unpacked: {} groups, {} total components",
        written,
        manifest
            .groups
            .iter()
            .map(|g| g.component_count)
            .sum::<u32>()
    );

    Ok(written)
}
