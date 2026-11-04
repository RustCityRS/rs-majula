//! CS2 script name loading from cracked name hash files.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

/// Load script names from an extracted TSV (id\tname format).
pub fn load_script_names(path: &Path) -> Result<HashMap<u32, String>> {
    let text = std::fs::read_to_string(path)?;
    let mut names = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((id_str, name)) = line.split_once('\t') else {
            continue;
        };
        let Ok(id) = id_str.trim().parse::<u32>() else {
            continue;
        };
        if !name.is_empty() {
            names.insert(id, name.to_string());
        }
    }
    info!("  Loaded {} script names", names.len());
    Ok(names)
}

/// Load script names from the full cache-names TSV, filtering for archive 12.
pub fn load_script_names_from_cache_names(path: &Path) -> Result<HashMap<u32, String>> {
    let text = std::fs::read_to_string(path)?;
    let mut names = HashMap::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }
        if parts[0].trim() != "12" {
            continue;
        }
        let Ok(id) = parts[1].trim().parse::<u32>() else {
            continue;
        };
        let name = parts[4].trim();
        if !name.is_empty() {
            names.insert(id, name.to_string());
        }
    }
    info!("  Loaded {} script names from cache names", names.len());
    Ok(names)
}
