//! Unpack Enum configs from JS5 cache to text config format.
//!
//! Produces `_unpack/all.enum` such that `pack::enum_pack::pack_enums`
//! reading the file produces byte-identical cache bytes for every enum.

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache647::{CacheType, EnumType};
use crate::pack::enum_pack::emit_enum_props;
use crate::pack_registry::PackRegistry;

const ARCHIVE_ENUM: u8 = 17;
const STRIDE_ENUM: u32 = 256;

pub fn unpack_enums(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_ENUM).unwrap_or(0);

    let out_path = output_dir.join("all.enum");
    let mut file = std::fs::File::create(&out_path)?;

    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_ENUM, gid)? else {
            continue;
        };

        let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
        sorted.sort_by_key(|e| e.id);

        for entry in sorted {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = gid * STRIDE_ENUM + entry.id;
            let mut def = EnumType::new(global_id as u16);
            let mut buf = Packet::from(entry.data.to_vec());
            def.decode(&mut buf);

            let fallback = format!("enum_{global_id}");
            let name = registry.r#enum.get_by_id(global_id).unwrap_or(&fallback);

            writeln!(file, "[{name}]")?;
            for (k, v) in emit_enum_props(&def) {
                writeln!(file, "{k}={v}")?;
            }
            writeln!(file)?;
            written += 1;
        }
    }

    info!("  Wrote {written} enum configs to {}", out_path.display());
    Ok(())
}
