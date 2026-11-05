#[cfg(since_274)]
use crate::pack::pack::{FileCache, parse_config_sections_cached};
#[cfg(since_274)]
use crate::pack::pack_registry::PackedFile;
#[cfg(since_274)]
use crate::pack::packed_data::PackedData;
#[cfg(since_274)]
use anyhow::Result;
#[cfg(since_274)]
use std::collections::HashMap;
#[cfg(since_274)]
use tracing::debug;

#[cfg(since_274)]
pub fn pack_mes(file_cache: &FileCache, constants: &HashMap<String, String>) -> Result<PackedFile> {
    let configs = parse_config_sections_cached(file_cache, "mesanim", constants);

    let count = configs
        .values()
        .flatten()
        .filter_map(|(key, _)| key.strip_prefix("len"))
        .filter_map(|rest| rest.parse::<u16>().ok())
        .max()
        .unwrap_or(0);
    debug!("  Deriving {count} mes entries from mesanim len tiers");

    let mut client = PackedData::new(count);
    for _ in 0..count {
        client.start_entry();
        client.finish_entry();
    }

    Ok(PackedFile {
        server: PackedData::new(0),
        client: Some(client),
    })
}
