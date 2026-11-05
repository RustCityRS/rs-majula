//! NPC discovery — cache → `content/pack/npc.pack` registry + `_unpack/all.npc` baseline.
//!
//! This is the **bootstrap** path for a fresh cache revision. It performs two
//! complementary jobs:
//!
//! 1. Generates `content/pack/npc.pack` (id→unique-name registry, ~13,410
//!    entries on rev 647). Idempotent: existing entries are preserved; only
//!    ids without a name are added. Pass `force = true` to rewrite all entries
//!    from cache (clobbering hand edits).
//! 2. Emits `_unpack/all.npc` — inert baseline `[name]` stubs for every NPC
//!    that has not yet been "adopted" into a hand-written `.npc` config under
//!    `content/scripts/`. Adopted names are detected by scanning every
//!    `**/*.npc` recursively (excluding the `_unpack/` tree itself), so once
//!    a stub is split out into e.g. `lumbridge.npc` it stops appearing in
//!    `all.npc`.
//!
//! The behaviour mirrors the legacy `tools/npc-pack-generator` `generate`
//! subcommand and the pack-pipeline `validate_config_packs()` invariant that
//! never drops user-edited registry entries.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::cache::loader;
use rs_dat2::CacheStore;

/// Outcome of a single `discover_npcs` run.
#[derive(Debug, Default, Clone)]
pub struct NpcDiscoveryReport {
    /// Total NPC ids found in the cache.
    pub cache_ids: usize,
    /// Number of `id=name` entries written to `npc.pack` after merging.
    pub pack_entries: usize,
    /// Entries newly added to `npc.pack` this run.
    pub pack_added: usize,
    /// Entries already present in `npc.pack` and preserved verbatim.
    pub pack_preserved: usize,
    /// `[name]` sections written to `_unpack/all.npc`.
    pub all_npc_unadopted: usize,
    /// Entries skipped because they were already adopted by hand-written
    /// `.npc` files outside `_unpack/`.
    pub all_npc_adopted_skipped: usize,
    /// Path of the regenerated `npc.pack`.
    pub pack_path: PathBuf,
    /// Path of the regenerated `_unpack/all.npc`.
    pub all_npc_path: PathBuf,
}

/// Discover NPCs from the cache and (re)generate `npc.pack` + `_unpack/all.npc`.
///
/// * `cache_dir` — path to the JS5 cache (must contain `main_file_cache.dat2`
///   and the configured idx files).
/// * `pack_dir`  — directory holding `npc.pack` (typically `content/pack`).
/// * `unpack_dir` — directory holding adopted `.npc` configs and the
///   `_unpack/` baseline tree (typically `content/scripts`).
/// * `force` — when `true`, names in `npc.pack` are overwritten from the cache
///   even if they already have a hand-edited entry. Default flow (`false`)
///   preserves all existing entries.
pub fn discover_npcs(
    cache_dir: &Path,
    pack_dir: &Path,
    unpack_dir: &Path,
    force: bool,
) -> Result<NpcDiscoveryReport> {
    let entries = build_unique_names(cache_dir)?;
    let cache_ids = entries.len();

    // ── 1. Merge into npc.pack (idempotent) ──────────────────────────────
    std::fs::create_dir_all(pack_dir).with_context(|| format!("create pack dir {:?}", pack_dir))?;
    let pack_path = pack_dir.join("npc.pack");

    let existing = if force {
        HashMap::new()
    } else {
        load_pack_file(&pack_path).unwrap_or_default()
    };

    let mut merged: HashMap<u16, String> = existing.clone();
    let mut pack_added = 0usize;
    for (id, unique_name, _, _) in &entries {
        match merged.get(id) {
            Some(existing_name) if !force => {
                if existing_name != unique_name {
                    // Preserve user-edited name; just note the divergence.
                    tracing::debug!(
                        id = id,
                        existing = %existing_name,
                        cache = %unique_name,
                        "npc.pack: preserving hand-edited name over cache name"
                    );
                }
            }
            _ => {
                merged.insert(*id, unique_name.clone());
                pack_added += 1;
            }
        }
    }

    let pack_preserved = merged.len().saturating_sub(pack_added);
    write_pack_file(&pack_path, &merged)?;
    info!(
        "npc.pack: {} total ({} added, {} preserved) → {:?}",
        merged.len(),
        pack_added,
        pack_preserved,
        pack_path
    );

    // ── 2. Emit _unpack/all.npc baseline (skipping adopted names) ────────
    let adopted = scan_adopted_names(unpack_dir)?;
    info!(
        "{} names already adopted in hand-written .npc configs",
        adopted.len()
    );

    let unpack_subdir = unpack_dir.join("_unpack");
    std::fs::create_dir_all(&unpack_subdir)
        .with_context(|| format!("create unpack dir {:?}", unpack_subdir))?;
    let all_npc_path = unpack_subdir.join("all.npc");
    let mut all_file = std::fs::File::create(&all_npc_path)
        .with_context(|| format!("create {:?}", all_npc_path))?;

    writeln!(all_file, "// Auto-generated NPC configs from cache.")?;
    writeln!(
        all_file,
        "// Remove entries from here as you adopt them into proper config files."
    )?;
    writeln!(
        all_file,
        "// Re-run via rs-pack discover (or `cargo run --bin npc-pack-generator -- generate`)."
    )?;
    writeln!(all_file)?;

    let mut unadopted = 0usize;
    let mut skipped = 0usize;
    for (id, unique_name, display, combat) in &entries {
        if adopted.contains(unique_name.as_str()) {
            skipped += 1;
            continue;
        }
        if *combat > 0 {
            writeln!(all_file, "// {} (id={}, lvl-{})", display, id, combat)?;
        } else {
            writeln!(all_file, "// {} (id={})", display, id)?;
        }
        writeln!(all_file, "[{}]", unique_name)?;
        writeln!(all_file)?;
        unadopted += 1;
    }
    info!(
        "_unpack/all.npc: {} unadopted entries written ({} adopted, skipped) → {:?}",
        unadopted, skipped, all_npc_path
    );

    Ok(NpcDiscoveryReport {
        cache_ids,
        pack_entries: merged.len(),
        pack_added,
        pack_preserved,
        all_npc_unadopted: unadopted,
        all_npc_adopted_skipped: skipped,
        pack_path,
        all_npc_path,
    })
}

// ── Internals ───────────────────────────────────────────────────────────

/// Sanitize a cache display name into a valid pack identifier.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Build the unique disambiguated name list from cache configs.
/// Returns sorted Vec<(id, unique_name, name, combat_level)>.
fn build_unique_names(cache_dir: &Path) -> Result<Vec<(u16, String, String, i32)>> {
    let mut cache = CacheStore::open(cache_dir)?;
    let npc_defs = loader::load_all_npcs(&mut cache)?;
    info!("Loaded {} NPC configs from cache", npc_defs.count());

    let mut name_counts: HashMap<String, Vec<u16>> = HashMap::new();
    let mut npc_info: Vec<(u16, String, String, i32)> = Vec::new();

    for id in 0..npc_defs.count() as u16 {
        let def = match npc_defs.get_by_id(id) {
            Some(d) => d,
            None => continue,
        };
        let display = def.name.as_deref().unwrap_or("null").to_string();
        let sanitized = sanitize_name(&display);
        let combat = def.vislevel.map_or(-1, i32::from);

        npc_info.push((id, sanitized.clone(), display, combat));

        let key = if sanitized.is_empty() || sanitized == "null" {
            String::new()
        } else {
            sanitized
        };
        name_counts.entry(key).or_default().push(id);
    }

    let ambiguous: HashSet<String> = name_counts
        .iter()
        .filter(|(k, v)| !k.is_empty() && v.len() > 1)
        .map(|(k, _)| k.clone())
        .collect();

    let mut result: Vec<(u16, String, String, i32)> = Vec::new();
    for (id, sanitized, display, combat) in npc_info {
        let unique_name = if sanitized.is_empty() || sanitized == "null" {
            format!("npc_{}", id)
        } else if ambiguous.contains(&sanitized) {
            format!("{}_{}", sanitized, id)
        } else {
            sanitized
        };
        result.push((id, unique_name, display, combat));
    }

    let total_affected: usize = name_counts
        .iter()
        .filter(|(k, v)| !k.is_empty() && v.len() > 1)
        .map(|(_, v)| v.len())
        .sum();
    info!(
        "{} unique names, {} ambiguous groups ({} NPCs needed disambiguation)",
        result.len(),
        ambiguous.len(),
        total_affected
    );

    Ok(result)
}

/// Scan all `*.npc` files under `configs_dir`, **excluding** the `_unpack/`
/// subtree, and collect `[name]` sections. These are the names already
/// "adopted" by hand-written configs and must NOT be regenerated into
/// `_unpack/all.npc`.
fn scan_adopted_names(configs_dir: &Path) -> Result<HashSet<String>> {
    let mut names = HashSet::new();
    if !configs_dir.exists() {
        return Ok(names);
    }
    scan_adopted_recursive(configs_dir, &mut names)?;
    Ok(names)
}

fn scan_adopted_recursive(dir: &Path, out: &mut HashSet<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Skip the _unpack baseline tree — those are *unadopted* stubs.
            if path.file_name().and_then(|s| s.to_str()) == Some("_unpack") {
                continue;
            }
            scan_adopted_recursive(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "npc") {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    warn!("failed to read {:?}: {}", path, e);
                    continue;
                }
            };
            for line in content.lines() {
                let line = line.split("//").next().unwrap_or("").trim();
                if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    out.insert(name.to_string());
                }
            }
        }
    }
    Ok(())
}

/// Load `npc.pack` (id=name format). Returns id→name map.
fn load_pack_file(path: &Path) -> Result<HashMap<u16, String>> {
    let content = std::fs::read_to_string(path).with_context(|| format!("read {:?}", path))?;
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        if let Some((id_str, name)) = line.split_once('=') {
            if let Ok(id) = id_str.trim().parse::<u16>() {
                map.insert(id, name.trim().to_string());
            }
        }
    }
    Ok(map)
}

/// Write `id=name` lines sorted by id.
fn write_pack_file(path: &Path, map: &HashMap<u16, String>) -> Result<()> {
    let mut entries: Vec<(u16, &str)> = map.iter().map(|(&id, name)| (id, name.as_str())).collect();
    entries.sort_by_key(|(id, _)| *id);
    let mut file = std::fs::File::create(path).with_context(|| format!("create {:?}", path))?;
    for (id, name) in entries {
        writeln!(file, "{}={}", id, name)?;
    }
    Ok(())
}
