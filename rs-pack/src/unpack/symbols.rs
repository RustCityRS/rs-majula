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
#[cfg(since_530)]
use crate::pack::util::placeholder_prefix;
use anyhow::{Context, Result, ensure};
use rs_io::js5::Js5LocalCache;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tracing::{info, warn};

#[cfg(before_530)]
fn placeholder_prefix(kind: &str) -> &str {
    kind
}

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
    let mut cache = Js5LocalCache::open(cache_dir)?;

    fs::create_dir_all(pack_dir)?;
    // String variables merge back in after the ints, wherever those now end.
    for (kind, strings) in [("varp", "varpstr"), ("vars", "varsstr")] {
        strip_string_vars(pack_dir, kind, strings)?;
    }

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
        "chatcat",
        generate_quickchat_pack(&mut cache, pack_dir, "chatcat", 0, force)?,
    );
    report.record(
        "chatphrase",
        generate_quickchat_pack(&mut cache, pack_dir, "chatphrase", 1, force)?,
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
    report.record("map", generate_map_pack(&mut cache, pack_dir)?);

    // Game vars.
    report.record("varp", generate_varp_pack(&mut cache, pack_dir)?);
    report.record("varpbit", generate_varpbit_pack(&mut cache, pack_dir)?);

    // Hardcoded enums.
    report.record("stat", generate_stat_pack(pack_dir)?);
    report.record("npc_stat", generate_npc_stat_pack(pack_dir)?);
    report.record("locshape", generate_locshape_pack(pack_dir)?);
    report.record("npc_mode", generate_npc_mode_pack(pack_dir)?);

    // Tables the cache holds empty, for the server's own bodies, and the
    // tables of client variables.
    for (name, section, group) in [
        ("hunt", "hunt", archive::CONFIG_GROUP_HUNT),
        ("mesanim", "mesanim", archive::CONFIG_GROUP_MESANIM),
        ("vars", "varworld", archive::CONFIG_GROUP_VARS),
        ("varn", "varnpc", archive::CONFIG_GROUP_VARN),
        #[cfg(since_647)]
        ("dbtable", "dbtable", archive::CONFIG_GROUP_DBTABLE),
        #[cfg(since_647)]
        ("dbrow", "dbrow", archive::CONFIG_GROUP_DBROW),
        ("bas", "bas", archive::CONFIG_GROUP_BAS),
        #[cfg(since_647)]
        ("cursor", "cursor", archive::CONFIG_GROUP_CURSOR),
        #[cfg(since_647)]
        ("hitmark", "hitmark", archive::CONFIG_GROUP_HITMARK),
        #[cfg(since_742)]
        ("skybox", "skybox", archive::CONFIG_GROUP_SKYBOX),
        #[cfg(since_742)]
        ("headbar", "headbar", archive::CONFIG_GROUP_HEADBAR),
        #[cfg(since_647)]
        ("light", "light", archive::CONFIG_GROUP_LIGHT),
        #[cfg(since_647)]
        ("mel", placeholder_prefix("mel"), archive::CONFIG_GROUP_MEL),
        ("msi", "msi", archive::CONFIG_GROUP_MSI),
        #[cfg(since_647)]
        ("quest", "quest", archive::CONFIG_GROUP_QUEST),
        (
            "varc",
            placeholder_prefix("varc"),
            archive::CONFIG_GROUP_VARC,
        ),
        ("area", "area", archive::CONFIG_GROUP_AREA),
        ("varobj", "varobject", archive::CONFIG_GROUP_VAROBJ),
        ("varnbit", "varnpcbit", archive::CONFIG_GROUP_VARNBIT),
        #[cfg(since_647)]
        ("config37", "config37", 37),
        #[cfg(since_647)]
        ("config38", "config38", 38),
        #[cfg(since_647)]
        ("config39", "config39", 39),
        #[cfg(since_647)]
        ("controller", "controller", archive::CONFIG_GROUP_CONTROLLER),
        #[cfg(since_647)]
        ("varcon", "config43", archive::CONFIG_GROUP_VARCON),
        #[cfg(since_647)]
        ("varconbit", "varconbit", archive::CONFIG_GROUP_VARCONBIT),
        #[cfg(since_647)]
        ("config45", "config45", 45),
        #[cfg(since_647)]
        ("varclan", "varclan", archive::CONFIG_GROUP_VARCLAN),
        #[cfg(since_647)]
        ("itemcode", "itemcode", archive::CONFIG_GROUP_ITEMCODE),
        #[cfg(since_742)]
        ("config49", "config49", 49),
        #[cfg(since_742)]
        ("config50", "config50", 50),
        #[cfg(since_742)]
        ("config51", "config51", 51),
        #[cfg(since_742)]
        ("config53", "config53", 53),
        #[cfg(since_742)]
        ("gamelogevent", "gamelogevent", 70),
        #[cfg(since_742)]
        ("config74", "config74", 74),
        #[cfg(since_647)]
        (
            "varclansetting",
            "varclansetting",
            archive::CONFIG_GROUP_VARCLANSETTING,
        ),
    ] {
        report.record(
            name,
            generate_auto_pack_named(
                &mut cache,
                pack_dir,
                name,
                section,
                0,
                archive::CONFIG,
                group,
                force,
            )?,
        );
    }

    #[cfg(since_647)]
    report.record(
        "billboard",
        generate_auto_pack_named(
            &mut cache,
            pack_dir,
            "billboard",
            "billboard",
            0,
            archive::BILLBOARDS,
            0,
            force,
        )?,
    );
    #[cfg(since_647)]
    for (name, group) in [("particleemitter", 0), ("particleeffector", 1)] {
        report.record(
            name,
            generate_auto_pack_named(
                &mut cache,
                pack_dir,
                name,
                placeholder_prefix(name),
                0,
                archive::PARTICLES,
                group,
                force,
            )?,
        );
    }

    for (kind, strings, group, string_group) in [
        (
            "varc",
            "varcstr",
            archive::CONFIG_GROUP_VARC,
            archive::CONFIG_GROUP_VARCSTR,
        ),
        (
            "varp",
            "varpstr",
            archive::CONFIG_GROUP_VARP,
            archive::CONFIG_GROUP_VARPSTR,
        ),
        (
            "vars",
            "varsstr",
            archive::CONFIG_GROUP_VARS,
            archive::CONFIG_GROUP_VARSSTR,
        ),
    ] {
        let first = cache
            .read_group(archive::CONFIG, group)?
            .and_then(|group| group.entries.iter().map(|entry| entry.id + 1).max())
            .unwrap_or(0);
        report.record(
            strings,
            generate_auto_pack_named(
                &mut cache,
                pack_dir,
                strings,
                placeholder_prefix(kind),
                first,
                archive::CONFIG,
                string_group,
                force,
            )?,
        );
        if kind != "varc" {
            merge_string_vars(pack_dir, kind, strings, first)?;
        }
    }

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
    for name in &["writeinv", "dbcolumn", "overlayinterface"] {
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

    let generated: Vec<(u32, String)> = entries
        .iter()
        .map(|(id, name)| {
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
            let name = if ambiguous.contains(&sanitized) && *id != first_id[&sanitized] {
                format!("{sanitized}_{id}")
            } else {
                sanitized
            };
            (*id, name)
        })
        .collect();
    let preserved: HashMap<u32, &String> = generated
        .iter()
        .filter_map(|(id, name)| {
            existing
                .get(id)
                .filter(|kept| *kept != name && should_preserve(kept, type_name, *id, force))
                .map(|kept| (*id, kept))
        })
        .collect();
    let taken: HashSet<&String> = preserved
        .values()
        .copied()
        .chain(
            existing
                .iter()
                .filter(|(id, name)| {
                    !cache_ids.contains(id) && should_preserve(name, type_name, **id, force)
                })
                .map(|(_, name)| name),
        )
        .collect();

    let mut file = File::create(path)?;
    let mut report = TypeReport::default();
    for (id, generated_name) in &generated {
        if let Some(existing_name) = preserved.get(id) {
            writeln!(file, "{}={}", id, existing_name)?;
            report.preserved += 1;
            report.total += 1;
            continue;
        }
        if taken.contains(generated_name) {
            writeln!(file, "{}={}_{}", id, generated_name, id)?;
        } else {
            writeln!(file, "{}={}", id, generated_name)?;
        }
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

fn generate_obj_pack(
    cache: &mut Js5LocalCache,
    pack_dir: &Path,
    force: bool,
) -> Result<TypeReport> {
    let items = loader::load_all_objs(cache, true)?;
    let mut entries: Vec<(u32, String)> = items
        .types
        .iter()
        .map(|def| {
            (
                def.id.raw(),
                def.name.as_deref().unwrap_or("null").to_string(),
            )
        })
        .collect();
    entries.sort_by_key(|(id, _)| *id);
    write_disambiguated_entity_pack(&pack_dir.join("obj.pack"), &entries, "obj", force)
}

fn generate_npc_pack(
    cache: &mut Js5LocalCache,
    pack_dir: &Path,
    force: bool,
) -> Result<TypeReport> {
    let npcs = loader::load_all_npcs(cache)?;
    let mut entries: Vec<(u32, String)> = npcs
        .types
        .iter()
        .map(|def| {
            (
                def.id.raw(),
                def.name.as_deref().unwrap_or("null").to_string(),
            )
        })
        .collect();
    entries.sort_by_key(|(id, _)| *id);
    write_disambiguated_entity_pack(&pack_dir.join("npc.pack"), &entries, "npc", force)
}

fn generate_loc_pack(
    cache: &mut Js5LocalCache,
    pack_dir: &Path,
    force: bool,
) -> Result<TypeReport> {
    let locs = loader::load_all_locs(cache)?;
    let mut entries: Vec<(u32, String)> = locs
        .types
        .iter()
        .map(|def| {
            (
                def.id.raw(),
                def.name.as_deref().unwrap_or("null").to_string(),
            )
        })
        .collect();
    entries.sort_by_key(|(id, _)| *id);
    write_disambiguated_entity_pack(&pack_dir.join("loc.pack"), &entries, "loc", force)
}

/// Generate a pack file with one entry per group (for archives where each group = one entity).
fn generate_auto_pack_per_group(
    cache: &mut Js5LocalCache,
    pack_dir: &Path,
    type_name: &str,
    archive_id: u8,
    force: bool,
) -> Result<TypeReport> {
    let pack_path = pack_dir.join(format!("{}.pack", type_name));
    let existing = load_existing_pack(&pack_path);
    let mut file = File::create(&pack_path)?;
    let group_count = cache.group_id_limit(archive_id).unwrap_or(0);
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
    cache: &mut Js5LocalCache,
    pack_dir: &Path,
    type_name: &str,
    archive_id: u8,
    stride: u32,
    force: bool,
) -> Result<TypeReport> {
    let pack_path = pack_dir.join(format!("{}.pack", type_name));
    let existing = load_existing_pack(&pack_path);
    let mut file = File::create(&pack_path)?;
    let group_count = cache.group_id_limit(archive_id).unwrap_or(0);
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

/// Quickchat ids span both archives; the global ones start at `QUICKCHAT_GLOBAL_FIRST`.
fn generate_quickchat_pack(
    cache: &mut Js5LocalCache,
    pack_dir: &Path,
    type_name: &str,
    group_id: u32,
    force: bool,
) -> Result<TypeReport> {
    let pack_path = pack_dir.join(format!("{type_name}.pack"));
    let existing = load_existing_pack(&pack_path);
    let mut ids = Vec::new();
    for (archive_id, first) in [
        (archive::QUICKCHAT, 0),
        (
            archive::QUICKCHAT_GLOBAL,
            u32::from(archive::QUICKCHAT_GLOBAL_FIRST),
        ),
    ] {
        if let Ok(Some(group)) = cache.read_group(archive_id, group_id) {
            ids.extend(group.entries.iter().map(|entry| first + entry.id));
        }
    }
    ids.sort_unstable();
    let mut file = File::create(&pack_path)?;
    let mut report = TypeReport::default();
    for id in ids {
        match existing.get(&id) {
            Some(name) if should_preserve(name, type_name, id, force) => {
                writeln!(file, "{id}={name}")?;
                report.preserved += 1;
            }
            _ => {
                writeln!(file, "{id}={type_name}_{id}")?;
                report.added += 1;
            }
        }
        report.total += 1;
    }
    info!(
        "  {}.pack: {} entries ({} preserved)",
        type_name, report.total, report.preserved
    );
    Ok(report)
}

fn generate_auto_pack_group(
    cache: &mut Js5LocalCache,
    pack_dir: &Path,
    type_name: &str,
    archive_id: u8,
    group_id: u32,
    force: bool,
) -> Result<TypeReport> {
    generate_auto_pack_named(
        cache,
        pack_dir,
        type_name,
        placeholder_prefix(type_name),
        0,
        archive_id,
        group_id,
        force,
    )
}

fn generate_auto_pack_named(
    cache: &mut Js5LocalCache,
    pack_dir: &Path,
    type_name: &str,
    section: &str,
    first_section: u32,
    archive_id: u8,
    group_id: u32,
    force: bool,
) -> Result<TypeReport> {
    let pack_path = pack_dir.join(format!("{}.pack", type_name));
    // A placeholder numbered from another revision's offset is still a placeholder.
    let mut existing = load_existing_pack(&pack_path);
    existing.retain(|_, name| {
        !name
            .strip_prefix(section)
            .and_then(|rest| rest.strip_prefix('_'))
            .is_some_and(|number| number.parse::<u32>().is_ok())
    });
    let mut file = File::create(&pack_path)?;
    let mut report = TypeReport::default();
    let mut written_ids = HashSet::new();

    if let Ok(Some(group)) = cache.read_group(archive_id, group_id) {
        for entry in &group.entries {
            written_ids.insert(entry.id);
            if let Some(name) = existing.get(&entry.id) {
                if should_preserve(name, section, first_section + entry.id, force) {
                    writeln!(file, "{}={}", entry.id, name)?;
                    report.preserved += 1;
                    report.total += 1;
                    continue;
                }
            }
            writeln!(
                file,
                "{}={}_{}",
                entry.id,
                section,
                first_section + entry.id
            )?;
            report.added += 1;
            report.total += 1;
        }
    }
    for (id, name) in &existing {
        if !written_ids.contains(id) && should_preserve(name, section, first_section + id, force) {
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

/// Map squares by group id, named from their hashes: ids differ between revisions.
fn generate_map_pack(cache: &mut Js5LocalCache, pack_dir: &Path) -> Result<TypeReport> {
    let lookup = super::map::build_name_lookup();
    let mut groups: Vec<_> = cache
        .index(archive::MAPS)
        .context("Missing map table")?
        .groups
        .iter()
        .map(|group| {
            let name = lookup
                .get(&group.name_hash)
                .cloned()
                .unwrap_or_else(|| format!("map_{}", group.id));
            (group.id, name)
        })
        .collect();
    groups.sort_unstable();
    let mut file = File::create(pack_dir.join("map.pack"))?;
    for (id, name) in &groups {
        writeln!(file, "{id}={name}")?;
    }
    let report = TypeReport {
        total: groups.len() as u32,
        added: groups.len() as u32,
        preserved: 0,
    };
    info!("  map.pack: {} entries", report.total);
    Ok(report)
}

fn generate_category_pack(cache: &mut Js5LocalCache, pack_dir: &Path) -> Result<TypeReport> {
    let mut categories = BTreeSet::new();

    let items = loader::load_all_objs(cache, true)?;
    for item in items.types.iter() {
        if let Some(category) = item.category.filter(|category| category.raw() > 0) {
            categories.insert(category.raw());
        }
    }

    let locs = loader::load_all_locs(cache)?;
    for loc in locs.types.iter() {
        if let Some(category) = loc.category.filter(|category| category.raw() > 0) {
            categories.insert(category.raw());
        }
    }

    // The server's category table packs every id up to the last one.
    let last = load_existing_pack(&pack_dir.join("category.pack"))
        .into_keys()
        .chain(categories.iter().copied())
        .max();
    let entries: Vec<(u32, String)> = last.map_or(Vec::new(), |last| {
        (0..=last)
            .map(|id| (id, format!("category_{id}")))
            .collect()
    });
    write_entity_pack(&pack_dir.join("category.pack"), &entries, "category", false)
}

/// Generate interface.pack from the interface archive.
fn generate_interface_packs(
    cache: &mut Js5LocalCache,
    pack_dir: &Path,
    force: bool,
) -> Result<(TypeReport, TypeReport)> {
    let iface_path = pack_dir.join("interface.pack");
    let existing_iface = load_existing_pack(&iface_path);
    let existing_comps = load_existing_components(&iface_path);

    let group_count = cache.group_id_limit(archive::INTERFACES).unwrap_or(0);

    let file_counts: HashMap<u32, u32> = cache
        .index(archive::INTERFACES)
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
            #[cfg(before_530)]
            let auto_comp_name = format!("com{}", cid);
            #[cfg(since_530)]
            let auto_comp_name = format!("com_{}", cid);
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

fn strip_string_vars(pack_dir: &Path, kind: &str, strings: &str) -> Result<()> {
    let path = pack_dir.join(format!("{kind}.pack"));
    let string_names: HashSet<String> =
        load_existing_pack(&pack_dir.join(format!("{strings}.pack")))
            .into_values()
            .collect();
    if !path.exists() || string_names.is_empty() {
        return Ok(());
    }
    let names = load_existing_pack(&path);
    let mut file = File::create(&path)?;
    for (id, name) in names {
        if !string_names.contains(&name) {
            writeln!(file, "{id}={name}")?;
        }
    }
    Ok(())
}

fn merge_string_vars(pack_dir: &Path, kind: &str, strings: &str, first: u32) -> Result<()> {
    let path = pack_dir.join(format!("{kind}.pack"));
    let mut names = load_existing_pack(&path);
    for (id, name) in load_existing_pack(&pack_dir.join(format!("{strings}.pack"))) {
        let id = first + id;
        ensure!(
            names.get(&id).is_none_or(|existing| existing == &name),
            "{kind}.pack: string variable [{name}] conflicts with id {id}"
        );
        names.insert(id, name);
    }
    let mut file = File::create(path)?;
    for (id, name) in names {
        writeln!(file, "{id}={name}")?;
    }
    Ok(())
}

fn generate_varp_pack(cache: &mut Js5LocalCache, pack_dir: &Path) -> Result<TypeReport> {
    let varps = loader::load_all_varps(cache)?;
    let path = pack_dir.join("varp.pack");
    let prefix = placeholder_prefix("varp");
    let entries: Vec<(u32, String)> = (0..varps.count() as u32)
        .map(|id| (id, format!("{prefix}_{id}")))
        .collect();
    if entries.is_empty() {
        warn!("  varp.pack: cache returned 0 varps — check cache version");
    }
    write_entity_pack(&path, &entries, prefix, false)
}

fn generate_varpbit_pack(cache: &mut Js5LocalCache, pack_dir: &Path) -> Result<TypeReport> {
    let varpbits = loader::load_all_varpbits(cache)?;
    let path = pack_dir.join("varpbit.pack");
    let prefix = placeholder_prefix("varpbit");
    let entries: Vec<(u32, String)> = (0..varpbits.count() as u32)
        .map(|id| (id, format!("{prefix}_{id}")))
        .collect();
    write_entity_pack(&path, &entries, prefix, false)
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
        (20, "runecraft"),
        (21, "hunter"),
        (22, "construction"),
        (23, "animallore"),
        (24, "rand"),
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
    fn string_vars_follow_the_ints_when_more_ints_arrive() {
        let dir = scratch("string_vars");
        fs::write(dir.join("vars.pack"), "0=wins\n1=duel1\n").unwrap();
        fs::write(dir.join("varsstr.pack"), "0=duel1\n").unwrap();

        strip_string_vars(&dir, "vars", "varsstr").unwrap();
        fs::write(
            dir.join("vars.pack"),
            fs::read_to_string(dir.join("vars.pack")).unwrap() + "1=varworld_1\n",
        )
        .unwrap();
        merge_string_vars(&dir, "vars", "varsstr", 2).unwrap();

        assert_eq!(
            fs::read_to_string(dir.join("vars.pack"))
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["0=wins", "1=varworld_1", "2=duel1"]
        );
    }

    #[test]
    fn a_new_entity_never_takes_a_kept_name() {
        let dir = scratch("disambiguated_pack");
        let path = dir.join("obj.pack");
        fs::write(&path, "317=raw_shrimp\n").unwrap();
        let cache = [
            (317, "Raw shrimps".to_owned()),
            (20893, "Raw shrimp".to_owned()),
        ];

        write_disambiguated_entity_pack(&path, &cache, "obj", false).unwrap();

        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(
            written.lines().collect::<Vec<_>>(),
            ["317=raw_shrimp", "20893=raw_shrimp_20893"]
        );
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
