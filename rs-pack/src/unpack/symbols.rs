//! Symbol discovery — generate `.pack` registry files from a JS5 cache.
//!
//! Scans the cache once and emits per-type `id=name` files under `pack_dir`
//! (e.g. `content/pack/npc.pack`). Output format matches 2004scape's pack
//! files and is consumed by the RuneScript compiler / `PackRegistry`.
//!
//! ## Idempotency
//!
//! Existing pack files are **never clobbered**. For each type:
//! - The current `<type>.pack` is loaded into memory.
//! - Cache-derived names are merged in.
//! - For ids that already have a hand-maintained name (i.e. anything that
//!   isn't the auto-generated `<type>_<id>` placeholder), the existing
//!   name wins.
//! - Ids present in the existing pack but not in the cache are preserved
//!   too (e.g. hand-added entries beyond the cache's id range).
//!
//! Pass `force = true` to overwrite hand-maintained names with whatever
//! the cache currently produces. Default is preserve.
//!
//! This mirrors the prior `tools/symbol-generator` behavior: that tool
//! already preserved existing names via the same `is_auto_name` heuristic.
//! The `force` flag is a new addition — symbol-generator had no escape
//! hatch for regenerating from the cache after a rename.
use crate::archive;
use crate::cache::loader;
use anyhow::Result;
use rs_io::js5::Js5Store;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tracing::{info, warn};

/// Per-type counts produced by a single discover pass.
#[derive(Debug, Default, Clone)]
pub struct TypeReport {
    /// Total entries written to the pack file.
    pub total: u32,
    /// Newly added (cache-derived) entries.
    pub added: u32,
    /// Entries kept from the existing pack (hand-maintained names).
    pub preserved: u32,
}

/// Aggregate report for a `discover_symbols` run.
#[derive(Debug, Default, Clone)]
pub struct DiscoveryReport {
    pub types: BTreeMap<String, TypeReport>,
}

impl DiscoveryReport {
    fn record(&mut self, ty: &str, report: TypeReport) {
        self.types.insert(ty.to_string(), report);
    }
}

/// Discover symbols from `cache_dir` and write/refresh pack files into
/// `pack_dir`. When `force` is false (default), hand-maintained names in
/// existing pack files are preserved.
pub fn discover_symbols(cache_dir: &Path, pack_dir: &Path, force: bool) -> Result<DiscoveryReport> {
    info!("Opening cache at {:?}", cache_dir);
    let mut cache = Js5Store::open(cache_dir)?;

    fs::create_dir_all(pack_dir)?;

    let mut report = DiscoveryReport::default();

    // Entity packs (named).
    report.record("obj", generate_obj_pack(&mut cache, pack_dir, force)?);
    report.record("npc", generate_npc_pack(&mut cache, pack_dir, force)?);
    report.record("loc", generate_loc_pack(&mut cache, pack_dir, force)?);

    // Animation frames + skeleton bases (one entry per group).
    report.record(
        "anim",
        generate_auto_pack_per_group(&mut cache, pack_dir, "anim", archive::ANIMS, force)?,
    );
    report.record(
        "base",
        generate_auto_pack_per_group(&mut cache, pack_dir, "base", archive::BASES, force)?,
    );

    // Multi-group archives — auto-named.
    report.record(
        "enum",
        generate_auto_pack_stride(
            &mut cache,
            pack_dir,
            "enum",
            archive::CONFIG_ENUM,
            256,
            force,
        )?,
    );
    report.record(
        "struct",
        generate_auto_pack_group(
            &mut cache,
            pack_dir,
            "struct",
            archive::CONFIG,
            archive::CONFIG_GROUP_STRUCT,
            force,
        )?,
    );
    report.record(
        "param",
        generate_auto_pack_group(
            &mut cache,
            pack_dir,
            "param",
            archive::CONFIG,
            archive::CONFIG_GROUP_PARAM,
            force,
        )?,
    );
    report.record(
        "inv",
        generate_auto_pack_group(
            &mut cache,
            pack_dir,
            "inv",
            archive::CONFIG,
            archive::CONFIG_GROUP_INV,
            force,
        )?,
    );
    report.record(
        "idk",
        generate_auto_pack_group(
            &mut cache,
            pack_dir,
            "idk",
            archive::CONFIG,
            archive::CONFIG_GROUP_IDK,
            force,
        )?,
    );
    report.record(
        "flo",
        generate_auto_pack_group(
            &mut cache,
            pack_dir,
            "flo",
            archive::CONFIG,
            archive::CONFIG_GROUP_FLO,
            force,
        )?,
    );
    report.record(
        "flu",
        generate_auto_pack_group(
            &mut cache,
            pack_dir,
            "flu",
            archive::CONFIG,
            archive::CONFIG_GROUP_FLU,
            force,
        )?,
    );
    report.record(
        "seq",
        generate_auto_pack_stride(&mut cache, pack_dir, "seq", archive::CONFIG_SEQ, 128, force)?,
    );
    report.record(
        "spotanim",
        generate_auto_pack_stride(
            &mut cache,
            pack_dir,
            "spotanim",
            archive::CONFIG_SPOTANIM,
            256,
            force,
        )?,
    );

    // Categories derived from item/loc category fields.
    report.record("category", generate_category_pack(&mut cache, pack_dir)?);

    // Game vars.
    report.record("varp", generate_varp_pack(&mut cache, pack_dir)?);
    report.record("varbit", generate_varbit_pack(&mut cache, pack_dir)?);

    // Hardcoded enums.
    report.record("stat", generate_stat_pack(pack_dir)?);
    report.record("npc_stat", generate_npc_stat_pack(pack_dir)?);
    report.record("locshape", generate_locshape_pack(pack_dir)?);
    report.record("npc_mode", generate_npc_mode_pack(pack_dir)?);
    report.record("hunt", generate_hunt_pack(pack_dir)?);

    // Synth (sound effects). Archive 4 holds one sound per group (file 0 only),
    // and the wire id sent by SOUND_SYNTH is the raw group id as a u16 — so the
    // pack id must equal the group id (stride 1), not a stride-256 encoding.
    report.record(
        "synth",
        generate_auto_pack_stride(&mut cache, pack_dir, "synth", archive::SYNTH, 1, force)?,
    );

    // Interface / component.
    let (iface_report, comp_report) = generate_interface_packs(&mut cache, pack_dir, force)?;
    report.record("interface", iface_report);
    report.record("component", comp_report);

    // Font metrics.
    report.record(
        "fontmetrics",
        generate_auto_pack_stride(
            &mut cache,
            pack_dir,
            "fontmetrics",
            archive::FONTMETRICS,
            256,
            force,
        )?,
    );

    // Server-side content configs — only create if missing.
    // These are hand-maintained by RuneScript content authors.
    for name in &[
        "mesanim",
        "varn",
        "vars",
        "writeinv",
        "dbrow",
        "dbtable",
        "dbcolumn",
        "overlayinterface",
    ] {
        let path = pack_dir.join(format!("{}.pack", name));
        if !path.exists() {
            generate_empty_pack(pack_dir, name)?;
            report.record(
                name,
                TypeReport {
                    total: 0,
                    ..Default::default()
                },
            );
        }
    }

    info!("Done. Pack files written to {:?}", pack_dir);
    Ok(report)
}

// ── helpers ─────────────────────────────────────────────────────────────

/// Sanitize a name for use as a RuneScript identifier.
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

/// Load existing pack file entries as id→name map.
fn load_existing_pack(path: &Path) -> BTreeMap<u32, String> {
    let mut map = BTreeMap::new();
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((id_str, name)) = line.split_once('=') {
                if let Ok(id) = id_str.parse::<u32>() {
                    map.insert(id, name.to_string());
                }
            }
        }
    }
    map
}

/// Load existing `iface_id:comp_id=name` component entries from a combined interface.pack.
fn load_existing_components(path: &Path) -> BTreeMap<(u32, u32), String> {
    let mut map = BTreeMap::new();
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Some((id_str, name)) = line.split_once('=') else {
                continue;
            };
            let Some((iface_str, comp_str)) = id_str.split_once(':') else {
                continue;
            };
            if let (Ok(iface), Ok(comp)) = (iface_str.parse::<u32>(), comp_str.parse::<u32>()) {
                map.insert((iface, comp), name.to_string());
            }
        }
    }
    map
}

/// Is `name` an auto-generated default like `synth_17` / `param_42`?
/// The placeholder discovery itself would write for `id`: `{type}_{id}`.
///
/// Only that exact name is disposable. A name that merely *looks*
/// generated but carries another id -- `208=category_22`, which
/// firemaking's `category=category_22` resolves through while id 22 is
/// `drawer` -- was assigned by someone, and dropping it would silently
/// re-point every reference to it.
fn is_auto_name(name: &str, type_name: &str, id: u32) -> bool {
    name.strip_prefix(type_name)
        .and_then(|rest| rest.strip_prefix('_'))
        .is_some_and(|n| n == id.to_string())
}

/// Should the existing hand-maintained name win over the cache name?
/// `force` overrides preservation; otherwise we keep anything that isn't
/// this id's auto-generated placeholder.
fn should_preserve(existing: &str, type_name: &str, id: u32, force: bool) -> bool {
    !force && !is_auto_name(existing, type_name, id)
}

// ── entity pack writers ─────────────────────────────────────────────────

fn write_entity_pack(
    path: &Path,
    entries: &[(u32, String)],
    type_name: &str,
    force: bool,
) -> Result<TypeReport> {
    let existing = load_existing_pack(path);
    let mut file = File::create(path)?;
    let mut report = TypeReport::default();
    let cache_ids: HashSet<u32> = entries.iter().map(|(id, _)| *id).collect();

    for (id, name) in entries {
        // Cache slot exists — always emit *some* name. Fall back to
        // `{type}_{id}` for null/empty/unsanitizable names so every cache
        // entry has a stable identifier for the source pipeline.
        let sanitized = if name == "null" || name.is_empty() {
            format!("{type_name}_{id}")
        } else {
            let s = sanitize_name(name);
            if s.is_empty() {
                format!("{type_name}_{id}")
            } else {
                s
            }
        };
        if let Some(existing_name) = existing.get(id) {
            if *existing_name != sanitized && should_preserve(existing_name, type_name, *id, force)
            {
                writeln!(file, "{}={}", id, existing_name)?;
                report.preserved += 1;
                report.total += 1;
                continue;
            }
        }
        writeln!(file, "{}={}", id, sanitized)?;
        report.added += 1;
        report.total += 1;
    }

    for (id, name) in &existing {
        if !cache_ids.contains(id) && should_preserve(name, type_name, *id, force) {
            writeln!(file, "{}={}", id, name)?;
            report.preserved += 1;
            report.total += 1;
        }
    }

    if report.preserved > 0 {
        info!(
            "  {}.pack: {} entries ({} preserved)",
            type_name, report.total, report.preserved
        );
    } else {
        info!("  {}.pack: {} entries", type_name, report.total);
    }
    Ok(report)
}

fn write_disambiguated_entity_pack(
    path: &Path,
    entries: &[(u32, String)],
    type_name: &str,
    force: bool,
) -> Result<TypeReport> {
    let existing = load_existing_pack(path);

    let mut name_counts: HashMap<String, Vec<u32>> = HashMap::new();
    for (id, name) in entries {
        let sanitized = if name == "null" || name.is_empty() {
            format!("{type_name}_{id}")
        } else {
            let s = sanitize_name(name);
            if s.is_empty() {
                format!("{type_name}_{id}")
            } else {
                s
            }
        };
        name_counts.entry(sanitized).or_default().push(*id);
    }

    let first_id: HashMap<String, u32> = name_counts
        .iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(name, ids)| (name.clone(), *ids.iter().min().unwrap()))
        .collect();

    let ambiguous: HashSet<String> = first_id.keys().cloned().collect();
    let cache_ids: HashSet<u32> = entries.iter().map(|(id, _)| *id).collect();

    let mut file = File::create(path)?;
    let mut report = TypeReport::default();
    for (id, name) in entries {
        let sanitized = if name == "null" || name.is_empty() {
            format!("{type_name}_{id}")
        } else {
            let s = sanitize_name(name);
            if s.is_empty() {
                format!("{type_name}_{id}")
            } else {
                s
            }
        };
        let generated_name = if ambiguous.contains(&sanitized) {
            if *id == first_id[&sanitized] {
                sanitized
            } else {
                format!("{}_{}", sanitized, id)
            }
        } else {
            sanitized
        };
        if let Some(existing_name) = existing.get(id) {
            if *existing_name != generated_name
                && should_preserve(existing_name, type_name, *id, force)
            {
                writeln!(file, "{}={}", id, existing_name)?;
                report.preserved += 1;
                report.total += 1;
                continue;
            }
        }
        writeln!(file, "{}={}", id, generated_name)?;
        report.added += 1;
        report.total += 1;
    }

    for (id, name) in &existing {
        if !cache_ids.contains(id) && should_preserve(name, type_name, *id, force) {
            writeln!(file, "{}={}", id, name)?;
            report.preserved += 1;
            report.total += 1;
        }
    }
    info!(
        "  {}.pack: {} entries ({} disambiguated, {} preserved)",
        type_name,
        report.total,
        ambiguous.len(),
        report.preserved
    );
    Ok(report)
}

// ── entity pack generators ──────────────────────────────────────────────

fn generate_obj_pack(cache: &mut Js5Store, pack_dir: &Path, force: bool) -> Result<TypeReport> {
    let items = loader::load_all_objs(cache, true)?;
    let mut entries: Vec<(u32, String)> = items
        .types
        .iter()
        .map(|def| {
            (
                def.id as u32,
                def.name.as_deref().unwrap_or("null").to_string(),
            )
        })
        .collect();
    entries.sort_by_key(|(id, _)| *id);
    write_disambiguated_entity_pack(&pack_dir.join("obj.pack"), &entries, "obj", force)
}

fn generate_npc_pack(cache: &mut Js5Store, pack_dir: &Path, force: bool) -> Result<TypeReport> {
    let npcs = loader::load_all_npcs(cache)?;
    let mut entries: Vec<(u32, String)> = npcs
        .types
        .iter()
        .map(|def| {
            (
                def.id as u32,
                def.name.as_deref().unwrap_or("null").to_string(),
            )
        })
        .collect();
    entries.sort_by_key(|(id, _)| *id);
    write_disambiguated_entity_pack(&pack_dir.join("npc.pack"), &entries, "npc", force)
}

fn generate_loc_pack(cache: &mut Js5Store, pack_dir: &Path, force: bool) -> Result<TypeReport> {
    let locs = loader::load_all_locs(cache)?;
    let mut entries: Vec<(u32, String)> = locs
        .types
        .iter()
        .map(|def| {
            (
                def.id as u32,
                def.name.as_deref().unwrap_or("null").to_string(),
            )
        })
        .collect();
    entries.sort_by_key(|(id, _)| *id);
    write_disambiguated_entity_pack(&pack_dir.join("loc.pack"), &entries, "loc", force)
}

/// Generate a pack file with one entry per group (for archives where each group = one entity).
fn generate_auto_pack_per_group(
    cache: &mut Js5Store,
    pack_dir: &Path,
    type_name: &str,
    archive_id: u8,
    force: bool,
) -> Result<TypeReport> {
    let pack_path = pack_dir.join(format!("{}.pack", type_name));
    let existing = load_existing_pack(&pack_path);
    let mut file = File::create(&pack_path)?;
    let group_count = cache.group_count(archive_id).unwrap_or(0);
    let mut report = TypeReport::default();

    for gid in 0..group_count {
        if cache
            .read_raw_group_bytes(archive_id, gid)
            .ok()
            .flatten()
            .is_none()
        {
            continue;
        }
        if let Some(name) = existing.get(&gid) {
            if should_preserve(name, type_name, gid, force) {
                writeln!(file, "{}={}", gid, name)?;
                report.preserved += 1;
                report.total += 1;
                continue;
            }
        }
        writeln!(file, "{}={}_{}", gid, type_name, gid)?;
        report.added += 1;
        report.total += 1;
    }

    // Preserve out-of-cache hand-added entries, as the other writers do.
    for (id, name) in &existing {
        let in_cache = *id < group_count
            && cache
                .read_raw_group_bytes(archive_id, *id)
                .ok()
                .flatten()
                .is_some();
        if !in_cache && should_preserve(name, type_name, *id, force) {
            writeln!(file, "{}={}", id, name)?;
            report.preserved += 1;
            report.total += 1;
        }
    }

    if report.total > 0 {
        info!(
            "  {}.pack: {} entries (auto-named)",
            type_name, report.total
        );
    }
    Ok(report)
}

fn generate_auto_pack_stride(
    cache: &mut Js5Store,
    pack_dir: &Path,
    type_name: &str,
    archive_id: u8,
    stride: u32,
    force: bool,
) -> Result<TypeReport> {
    let pack_path = pack_dir.join(format!("{}.pack", type_name));
    let existing = load_existing_pack(&pack_path);
    let mut file = File::create(&pack_path)?;
    let group_count = cache.group_count(archive_id).unwrap_or(0);
    let mut report = TypeReport::default();

    for gid in 0..group_count {
        if let Ok(Some(group)) = cache.read_group(archive_id, gid) {
            for entry in &group.entries {
                let id = gid * stride + entry.id;
                if let Some(name) = existing.get(&id) {
                    if should_preserve(name, type_name, id, force) {
                        writeln!(file, "{}={}", id, name)?;
                        report.preserved += 1;
                        report.total += 1;
                        continue;
                    }
                }
                writeln!(file, "{}={}_{}", id, type_name, id)?;
                report.added += 1;
                report.total += 1;
            }
        }
    }

    // Preserve out-of-cache hand-added entries.
    for (id, name) in &existing {
        if should_preserve(name, type_name, *id, force) {
            let already_written = (0..group_count).any(|gid| {
                cache
                    .read_group(archive_id, gid)
                    .ok()
                    .flatten()
                    .is_some_and(|g| g.entries.iter().any(|e| gid * stride + e.id == *id))
            });
            if !already_written {
                writeln!(file, "{}={}", id, name)?;
                report.preserved += 1;
                report.total += 1;
            }
        }
    }
    if report.preserved > 0 {
        info!(
            "  {}.pack: {} entries ({} preserved)",
            type_name, report.total, report.preserved
        );
    } else {
        info!(
            "  {}.pack: {} entries (auto-named)",
            type_name, report.total
        );
    }
    Ok(report)
}

fn generate_auto_pack_group(
    cache: &mut Js5Store,
    pack_dir: &Path,
    type_name: &str,
    archive_id: u8,
    group_id: u32,
    force: bool,
) -> Result<TypeReport> {
    let pack_path = pack_dir.join(format!("{}.pack", type_name));
    let existing = load_existing_pack(&pack_path);
    let mut file = File::create(&pack_path)?;
    let mut report = TypeReport::default();
    let mut written_ids = HashSet::new();

    if let Ok(Some(group)) = cache.read_group(archive_id, group_id) {
        for entry in &group.entries {
            written_ids.insert(entry.id);
            if let Some(name) = existing.get(&entry.id) {
                if should_preserve(name, type_name, entry.id, force) {
                    writeln!(file, "{}={}", entry.id, name)?;
                    report.preserved += 1;
                    report.total += 1;
                    continue;
                }
            }
            writeln!(file, "{}={}_{}", entry.id, type_name, entry.id)?;
            report.added += 1;
            report.total += 1;
        }
    }
    for (id, name) in &existing {
        if !written_ids.contains(id) && should_preserve(name, type_name, *id, force) {
            writeln!(file, "{}={}", id, name)?;
            report.preserved += 1;
            report.total += 1;
        }
    }
    if report.preserved > 0 {
        info!(
            "  {}.pack: {} entries ({} preserved)",
            type_name, report.total, report.preserved
        );
    } else {
        info!(
            "  {}.pack: {} entries (auto-named)",
            type_name, report.total
        );
    }
    Ok(report)
}

fn generate_category_pack(cache: &mut Js5Store, pack_dir: &Path) -> Result<TypeReport> {
    let mut categories = BTreeSet::new();

    let items = loader::load_all_objs(cache, true)?;
    for item in items.types.iter() {
        if let Some(category) = item.category.filter(|&category| category > 0) {
            categories.insert(category as u32);
        }
    }

    let locs = loader::load_all_locs(cache)?;
    for loc in locs.types.iter() {
        if let Some(category) = loc.category.filter(|&category| category > 0) {
            categories.insert(category as u32);
        }
    }

    let entries: Vec<(u32, String)> = categories
        .iter()
        .map(|&id| (id, format!("category_{}", id)))
        .collect();
    write_entity_pack(&pack_dir.join("category.pack"), &entries, "category", false)
}

fn generate_hunt_pack(pack_dir: &Path) -> Result<TypeReport> {
    let path = pack_dir.join("hunt.pack");
    if path.exists() {
        info!("  hunt.pack: already exists, preserving");
        return Ok(TypeReport::default());
    }
    File::create(&path)?;
    info!("  hunt.pack: created empty (server-only, populated by validate)");
    Ok(TypeReport::default())
}

/// Generate interface.pack from the interface archive.
fn generate_interface_packs(
    cache: &mut Js5Store,
    pack_dir: &Path,
    force: bool,
) -> Result<(TypeReport, TypeReport)> {
    let iface_path = pack_dir.join("interface.pack");
    let existing_iface = load_existing_pack(&iface_path);
    let existing_comps = load_existing_components(&iface_path);

    let group_count = cache.group_count(archive::INTERFACES).unwrap_or(0);

    let file_counts: HashMap<u32, u32> = cache
        .reference_table(archive::INTERFACES)
        .map(|rt| rt.groups.iter().map(|g| (g.id, g.file_count)).collect())
        .unwrap_or_default();

    let mut iface_file = File::create(&iface_path)?;
    let mut iface_report = TypeReport::default();
    let mut comp_report = TypeReport::default();

    for gid in 0..group_count {
        let auto_iface_name = format!("interface_{}", gid);
        if let Some(name) = existing_iface.get(&gid) {
            if *name != auto_iface_name && !force {
                writeln!(iface_file, "{}={}", gid, name)?;
                iface_report.preserved += 1;
            } else {
                writeln!(iface_file, "{}={}", gid, auto_iface_name)?;
                iface_report.added += 1;
            }
        } else {
            writeln!(iface_file, "{}={}", gid, auto_iface_name)?;
            iface_report.added += 1;
        }
        iface_report.total += 1;

        let fc = file_counts.get(&gid).copied().unwrap_or(0);
        for cid in 0..fc {
            let auto_comp_name = format!("com{}", cid);
            if let Some(name) = existing_comps.get(&(gid, cid)) {
                if *name != auto_comp_name && !force {
                    writeln!(iface_file, "{}:{}={}", gid, cid, name)?;
                    comp_report.preserved += 1;
                } else {
                    writeln!(iface_file, "{}:{}={}", gid, cid, auto_comp_name)?;
                    comp_report.added += 1;
                }
            } else {
                writeln!(iface_file, "{}:{}={}", gid, cid, auto_comp_name)?;
                comp_report.added += 1;
            }
            comp_report.total += 1;
        }
    }

    if iface_report.preserved > 0 {
        info!(
            "  interface.pack: {} interfaces ({} preserved), {} components ({} preserved)",
            iface_report.total, iface_report.preserved, comp_report.total, comp_report.preserved,
        );
    } else {
        info!(
            "  interface.pack: {} interfaces, {} components",
            iface_report.total, comp_report.total,
        );
    }
    Ok((iface_report, comp_report))
}

fn generate_empty_pack(pack_dir: &Path, type_name: &str) -> Result<()> {
    File::create(pack_dir.join(format!("{}.pack", type_name)))?;
    info!("  {}.pack: empty (placeholder)", type_name);
    Ok(())
}

// ── game var pack generators ────────────────────────────────────────────

fn generate_varp_pack(cache: &mut Js5Store, pack_dir: &Path) -> Result<TypeReport> {
    let varps = loader::load_all_varps(cache)?;
    let path = pack_dir.join("varp.pack");
    let entries: Vec<(u32, String)> = (0..varps.count() as u32)
        .map(|id| (id, format!("varp_{}", id)))
        .collect();
    if entries.is_empty() {
        warn!("  varp.pack: cache returned 0 varps — check cache version");
    }
    write_entity_pack(&path, &entries, "varp", false)
}

fn generate_varbit_pack(cache: &mut Js5Store, pack_dir: &Path) -> Result<TypeReport> {
    let varbits = loader::load_all_varbits(cache)?;
    let path = pack_dir.join("varbit.pack");
    let entries: Vec<(u32, String)> = (0..varbits.count() as u32)
        .map(|id| (id, format!("varbit_{}", id)))
        .collect();
    write_entity_pack(&path, &entries, "varbit", false)
}

// ── hardcoded pack generators ──────────────────────────────────────────

fn write_hardcoded(
    pack_dir: &Path,
    type_name: &str,
    entries: &[(u32, &str)],
) -> Result<TypeReport> {
    let owned: Vec<(u32, String)> = entries
        .iter()
        .map(|(id, name)| (*id, name.to_string()))
        .collect();
    write_entity_pack(
        &pack_dir.join(format!("{}.pack", type_name)),
        &owned,
        type_name,
        false,
    )
}

fn generate_stat_pack(pack_dir: &Path) -> Result<TypeReport> {
    let stats: &[(u32, &str)] = &[
        (0, "attack"),
        (1, "defence"),
        (2, "strength"),
        (3, "hitpoints"),
        (4, "ranged"),
        (5, "prayer"),
        (6, "magic"),
        (7, "cooking"),
        (8, "woodcutting"),
        (9, "fletching"),
        (10, "fishing"),
        (11, "firemaking"),
        (12, "crafting"),
        (13, "smithing"),
        (14, "mining"),
        (15, "herblore"),
        (16, "agility"),
        (17, "thieving"),
        (18, "slayer"),
        (19, "farming"),
        (20, "runecrafting"),
        (21, "hunter"),
        (22, "construction"),
        (23, "summoning"),
        (24, "dungeoneering"),
    ];
    write_hardcoded(pack_dir, "stat", stats)
}

fn generate_npc_stat_pack(pack_dir: &Path) -> Result<TypeReport> {
    let stats: &[(u32, &str)] = &[
        (0, "attack"),
        (1, "defence"),
        (2, "strength"),
        (3, "hitpoints"),
        (4, "ranged"),
        (5, "magic"),
    ];
    write_hardcoded(pack_dir, "npc_stat", stats)
}

fn generate_locshape_pack(pack_dir: &Path) -> Result<TypeReport> {
    let shapes: &[(u32, &str)] = &[
        (0, "wall_straight"),
        (1, "wall_diagonalcorner"),
        (2, "wall_l"),
        (3, "wall_squarecorner"),
        (4, "walldecor_straight_nooffset"),
        (5, "walldecor_straight_offset"),
        (6, "walldecor_diagonal_offset"),
        (7, "walldecor_diagonal_nooffset"),
        (8, "walldecor_diagonal_both"),
        (9, "wall_diagonal"),
        (10, "centrepiece_straight"),
        (11, "centrepiece_diagonal"),
        (12, "roof_straight"),
        (13, "roof_diagonal_with_roofedge"),
        (14, "roof_diagonal"),
        (15, "roof_l_concave"),
        (16, "roof_l_convex"),
        (17, "roof_flat"),
        (18, "roofedge_straight"),
        (19, "roofedge_diagonalcorner"),
        (20, "roofedge_l"),
        (21, "roofedge_squarecorner"),
        (22, "grounddecor"),
    ];
    write_hardcoded(pack_dir, "locshape", shapes)
}

fn generate_npc_mode_pack(pack_dir: &Path) -> Result<TypeReport> {
    let modes: &[(u32, &str)] = &[
        (0, "none"),
        (1, "wander"),
        (2, "patrol"),
        (3, "playerescape"),
        (4, "playerfollow"),
        (5, "playerface"),
        (6, "playerfaceclose"),
    ];
    write_hardcoded(pack_dir, "npc_mode", modes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Scratch under the workspace `target/`, never the system temp dir.
    fn scratch(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/unit-scratch/discover")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn only_a_names_own_placeholder_is_auto() {
        assert!(is_auto_name("category_208", "category", 208));
        assert!(!is_auto_name("category_22", "category", 208));
        assert!(!is_auto_name("drawer", "category", 22));
        assert!(!is_auto_name("category_", "category", 0));
        assert!(!is_auto_name("categoryx_208", "category", 208));
        assert!(should_preserve("category_22", "category", 208, false));
        assert!(!should_preserve("category_208", "category", 208, false));
        assert!(!should_preserve("drawer", "category", 22, true));
    }

    #[test]
    fn a_hand_assigned_name_survives_rediscovery() {
        let dir = scratch("entity_pack");
        let path = dir.join("category.pack");
        // 208 carries another id's placeholder-shaped name (as in the real
        // registry); 300 is out of the cache's range; 5 is a stale placeholder.
        fs::write(
            &path,
            "5=category_5\n22=drawer\n208=category_22\n300=category_7\n",
        )
        .unwrap();
        let cache = [
            (5, String::new()),
            (22, String::new()),
            (208, String::new()),
        ];

        write_entity_pack(&path, &cache, "category", false).unwrap();

        let written = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = written.lines().collect();
        assert_eq!(
            lines,
            [
                "5=category_5",
                "22=drawer",
                "208=category_22",
                "300=category_7"
            ]
        );
    }
}
