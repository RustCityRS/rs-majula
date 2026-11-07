//! Discover dbtable / dbrow registry entries by scanning RuneScript source.
//!
//! Scans every `.dbtable` / `.dbrow` file under `scripts_dir` for `[name]`
//! section headers and emits / refreshes `pack_dir/dbtable.pack` and
//! `pack_dir/dbrow.pack` (`id=name` text format, one entry per line).
//!
//! Revision-agnostic: the registry is derived from text source files in
//! `content/scripts/runescript/`, never from the cache.
//!
//! Idempotency: when a pack file already exists, existing `id=name`
//! assignments are preserved for names that still appear in the scan. New
//! names take the next available id (max(existing_id) + 1, then incrementing).
//! Removed names leave their ids unused (gaps), so other content referencing
//! ids stays valid across runs.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Summary of one discovery pass.
#[derive(Debug, Default, Clone)]
pub struct DbDiscoveryReport {
    pub dbtable_total: usize,
    pub dbtable_added: usize,
    pub dbtable_removed: usize,
    pub dbrow_total: usize,
    pub dbrow_added: usize,
    pub dbrow_removed: usize,
    pub dbtable_pack: PathBuf,
    pub dbrow_pack: PathBuf,
}

/// Scan `scripts_dir` for `.dbtable` / `.dbrow` source files and regenerate
/// the matching `.pack` registries under `pack_dir`.
///
/// `force` is accepted for parity with sibling discover modules; it has no
/// behavioral effect today (this scanner always rewrites the pack file when
/// the discovered name set differs from the on-disk one — and is a no-op
/// when they match).
pub fn discover_dbrows(
    scripts_dir: &Path,
    pack_dir: &Path,
    force: bool,
) -> Result<DbDiscoveryReport> {
    if !scripts_dir.exists() {
        anyhow::bail!("scripts dir not found: {}", scripts_dir.display());
    }
    fs::create_dir_all(pack_dir)
        .with_context(|| format!("creating pack dir {}", pack_dir.display()))?;

    let mut tables: BTreeSet<String> = BTreeSet::new();
    let mut rows: BTreeSet<String> = BTreeSet::new();

    for path in walk(scripts_dir, "dbtable")? {
        scan_names(&path, &mut tables)?;
    }
    for path in walk(scripts_dir, "dbrow")? {
        scan_names(&path, &mut rows)?;
    }

    let dbtable_pack = pack_dir.join("dbtable.pack");
    let dbrow_pack = pack_dir.join("dbrow.pack");

    let (t_added, t_removed) = write_pack(&dbtable_pack, &tables, force)?;
    let (r_added, r_removed) = write_pack(&dbrow_pack, &rows, force)?;

    let report = DbDiscoveryReport {
        dbtable_total: tables.len(),
        dbtable_added: t_added,
        dbtable_removed: t_removed,
        dbrow_total: rows.len(),
        dbrow_added: r_added,
        dbrow_removed: r_removed,
        dbtable_pack,
        dbrow_pack,
    };

    info!(
        "discover_dbrows: dbtable {} (+{} -{}), dbrow {} (+{} -{})",
        report.dbtable_total,
        report.dbtable_added,
        report.dbtable_removed,
        report.dbrow_total,
        report.dbrow_added,
        report.dbrow_removed,
    );
    Ok(report)
}

fn walk(dir: &Path, ext: &str) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_inner(dir, ext, &mut out)?;
    Ok(out)
}

fn walk_inner(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            walk_inner(&p, ext, out)?;
        } else if p.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(p);
        }
    }
    Ok(())
}

fn scan_names(path: &Path, out: &mut BTreeSet<String>) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if let Some(rest) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            out.insert(rest.to_string());
        }
    }
    Ok(())
}

/// Read an existing `id=name` pack file. Returns `name -> id`.
fn read_existing(path: &Path) -> Result<BTreeMap<String, u32>> {
    let mut out = BTreeMap::new();
    if !path.exists() {
        return Ok(out);
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading existing pack {}", path.display()))?;
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let Some((id_s, name)) = line.split_once('=') else {
            warn!(
                "{}:{} malformed pack line (no '='): {:?}",
                path.display(),
                lineno + 1,
                line
            );
            continue;
        };
        let Ok(id) = id_s.trim().parse::<u32>() else {
            warn!(
                "{}:{} malformed pack line (bad id): {:?}",
                path.display(),
                lineno + 1,
                line
            );
            continue;
        };
        out.insert(name.trim().to_string(), id);
    }
    Ok(out)
}

/// Write a pack file, preserving existing id assignments for names still
/// present. Returns `(added, removed)` counts vs. the previous file.
fn write_pack(path: &Path, names: &BTreeSet<String>, _force: bool) -> Result<(usize, usize)> {
    let existing = read_existing(path)?;

    // Preserve existing id for names still present.
    let mut id_to_debugname: BTreeMap<u32, String> = BTreeMap::new();
    let mut assigned: BTreeSet<String> = BTreeSet::new();
    let mut removed = 0usize;
    for (name, id) in &existing {
        if names.contains(name) {
            id_to_debugname.insert(*id, name.clone());
            assigned.insert(name.clone());
        } else {
            removed += 1;
        }
    }

    // Allocate fresh ids for new names. Start at max(existing_id)+1 so we
    // never reuse a freed slot — keeps ids monotonic across renames.
    let mut next_id: u32 = existing.values().copied().max().map(|m| m + 1).unwrap_or(0);
    let mut added = 0usize;
    for name in names {
        if assigned.contains(name) {
            continue;
        }
        // Skip ids that somehow collide (shouldn't happen, but be safe).
        while id_to_debugname.contains_key(&next_id) {
            next_id += 1;
        }
        id_to_debugname.insert(next_id, name.clone());
        next_id += 1;
        added += 1;
    }

    debug!(
        "write_pack {}: {} entries ({} added, {} removed)",
        path.display(),
        id_to_debugname.len(),
        added,
        removed
    );

    let mut f = File::create(path).with_context(|| format!("creating {}", path.display()))?;
    for (id, name) in &id_to_debugname {
        writeln!(f, "{}={}", id, name)?;
    }
    Ok((added, removed))
}
