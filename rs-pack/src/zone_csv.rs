//! The zone lists in `content/<rev>/maps` (`free2play.csv`, `multiway.csv`,
//! `ignore.csv`): one zone per line, `level_mx_mz_lx_lz`, the zone's origin
//! tile in its map square.

use crate::types::MapSquareCsv;
use std::collections::HashSet;
use std::path::Path;
use tracing::warn;

/// The key a zone (8x8 tiles) has in a `MapSquareCsv`.
pub fn zone_key(y: u8, x: u16, z: u16) -> u32 {
    ((x >> 3) & 0x7FF) as u32 | ((((z >> 3) & 0x7FF) as u32) << 11) | (((y & 0x3) as u32) << 22)
}

pub fn load_csv_zones(path: &Path) -> MapSquareCsv {
    let mut set = HashSet::default();
    let Ok(content) = std::fs::read_to_string(path) else {
        return set;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let parts: Vec<&str> = line.split('_').collect();
        if parts.len() != 5 {
            continue;
        }
        let Ok(y) = parts[0].parse::<u8>() else {
            continue;
        };
        let Ok(mx) = parts[1].parse::<u16>() else {
            continue;
        };
        let Ok(mz) = parts[2].parse::<u16>() else {
            continue;
        };
        let Ok(lx) = parts[3].parse::<u16>() else {
            continue;
        };
        let Ok(lz) = parts[4].parse::<u16>() else {
            continue;
        };
        if lx % 8 != 0 || lz % 8 != 0 {
            warn!("CSV map line not aligned to zone: {}", line);
        }
        set.insert(zone_key(y, (mx << 6) + lx, (mz << 6) + lz));
    }
    set
}
