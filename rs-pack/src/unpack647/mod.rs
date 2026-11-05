//! Unpack — rev-647 JS5 (dat2) cache → text config files + pack file
//! generation. Ported from the rev-647 source repo's `rs-pack/src/unpack/`
//! (which mirrors 2004scape's `engine/tools/unpack/` toolkit), rewired from
//! `rs_cache` to `rs_dat2`.
//!
//! Entry point [`unpack_all`] drives every per-domain unpacker against the
//! `expected/` cache and writes the full per-domain tree under the output
//! directory (`scripts/_unpack/all.*`, `maps/_unpack/`, `interfaces/_unpack/`,
//! `clientscript/{_unpack,_decompiled}/`, sprite PNGs, model `.ob2`s, …) plus
//! an `unpack_report.txt` summary.

pub mod anim;
pub mod billboards;
pub mod binary_assets;
pub mod common;
pub mod config_generic;
pub mod cs2;
pub mod defaults;
pub mod enums;
pub mod flo;
pub mod flu;
pub mod fontmetrics;
pub mod idk;
pub mod interfaces;
pub mod inv;
pub mod loc;
pub mod maps;
pub mod materials;
pub mod midi;
pub mod models;
pub mod npc;
pub mod obj;
pub mod param;
pub mod particles;
pub mod quickchat;
pub mod seq;
pub mod shaders;
pub mod spotanim;
pub mod sprites;
pub mod struct_pack;
pub mod textures_named;
pub mod varbit;
pub mod varp;

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rs_dat2::archive::CONFIG_UNKNOWN_GROUPS;
use tracing::{error, info};

use crate::pack::pack_registry::PackRegistry;

/// One line per unpack step for the run report.
struct StepResult {
    name: &'static str,
    outcome: String,
    ok: bool,
}

/// Run one unpack step, recording success/failure without aborting the run.
fn step<F>(results: &mut Vec<StepResult>, name: &'static str, f: F)
where
    F: FnOnce() -> Result<String>,
{
    info!("Unpacking {name}...");
    match f() {
        Ok(outcome) => {
            if !outcome.is_empty() {
                info!("  {name}: {outcome}");
            }
            results.push(StepResult {
                name,
                outcome,
                ok: true,
            });
        }
        Err(e) => {
            error!("  {name} FAILED: {e:#}");
            results.push(StepResult {
                name,
                outcome: format!("ERROR: {e:#}"),
                ok: false,
            });
        }
    }
}

/// Unpack the full rev-647 cache to a per-domain content tree.
///
/// Arguments follow the CLI's `(expected, output, pack)` order:
/// - `expected_dir` — the raw dat2/idx cache (`main_file_cache.dat2` + idx).
/// - `output_dir` — content root; each domain writes `<domain>/_unpack/`
///   below it (configs go to `scripts/_unpack/all.*`).
/// - `pack_dir` — `.pack` name registries (discovered idempotently).
pub fn unpack_all(expected_dir: &Path, output_dir: &Path, pack_dir: &Path) -> Result<()> {
    unpack(expected_dir, output_dir, pack_dir, Domains::ALL, output_dir)
}

/// Which parts of the cache an unpack writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Domains {
    /// Config text (`scripts/_unpack/all.*`) and its animations.
    pub configs: bool,
    /// Map squares (`maps/_unpack`).
    pub maps: bool,
    /// Everything else: clientscripts, interfaces, models, sprites, songs, ...
    pub assets: bool,
}

impl Domains {
    pub const ALL: Self = Self {
        configs: true,
        maps: true,
        assets: true,
    };

    /// Parse a comma-separated list of `configs`, `maps`, `assets`.
    pub fn parse(list: &str) -> Result<Self> {
        let mut domains = Self {
            configs: false,
            maps: false,
            assets: false,
        };
        for part in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            match part {
                "configs" => domains.configs = true,
                "maps" => domains.maps = true,
                "assets" => domains.assets = true,
                other => anyhow::bail!("unknown unpack domain `{other}` (configs, maps, assets)"),
            }
        }
        Ok(domains)
    }
}

/// Unpack `domains` of the cache into `output_dir`.
///
/// A partial unpack regenerates content in place: each selected domain's
/// `_unpack` directory is **deleted first**, so what it holds afterwards is a
/// function of the cache and the name registries alone -- never of what was
/// there before. That is what lets `_unpack` stay out of git. Authored data
/// lives beside it and is laid over it at pack time
/// (`pack::overlay_section`, `map_format::source_dirs`).
///
/// `work_dir` takes what a partial run needs but must not write into
/// `output_dir`: the decompiled clientscripts the varp unpacker reads, and the
/// run report.
pub fn unpack(
    expected_dir: &Path,
    output_dir: &Path,
    pack_dir: &Path,
    domains: Domains,
    work_dir: &Path,
) -> Result<()> {
    // In the 647 era `expected/` holds the raw JS5 dat2 cache.
    let cache_dir = expected_dir;

    std::fs::create_dir_all(output_dir)?;
    std::fs::create_dir_all(pack_dir)?;
    std::fs::create_dir_all(work_dir)?;
    if domains != Domains::ALL {
        for (selected, dir) in [
            (domains.configs, output_dir.join("scripts").join("_unpack")),
            (domains.maps, output_dir.join("maps").join("_unpack")),
        ] {
            if selected && dir.exists() {
                info!("Clearing {} for regeneration", dir.display());
                std::fs::remove_dir_all(&dir)?;
            }
        }
    }

    // Configs live under scripts/, matching the source repo's layout where
    // `_unpack/` sits inside the scripts dir.
    let scripts_dir = output_dir.join("scripts");
    if domains.configs {
        std::fs::create_dir_all(scripts_dir.join("_unpack"))?;
    }

    let mut results: Vec<StepResult> = Vec::new();

    // Step 1: Generate pack files from cache via the idempotent discover
    // path. Hand-maintained names in existing `<type>.pack` files are
    // preserved; only auto-generated / missing entries are (re)written.
    info!("Discovering pack registries from cache (idempotent)...");
    let report = crate::discover::discover_symbols(cache_dir, pack_dir, false)?;
    info!("  {} registry types processed", report.types.len());

    // Step 2: Load the freshly-discovered packs.
    let registry = PackRegistry::load(pack_dir)?;

    // Step 3: CS2 scripts first — the varp unpacker scans the decompiled
    // output for transmit analysis.
    // A partial run decompiles into `work_dir`, so the content tree's own
    // clientscript directory is never touched.
    let cs2_root = if domains.assets { output_dir } else { work_dir };
    let cs2_dir = cs2_root.join("clientscript").join("_decompiled");
    if domains.assets {
        step(&mut results, "cs2 disassembly", || {
            cs2::unpack_cs2_scripts(cache_dir, output_dir).map(|n| format!("{n} scripts"))
        });
    }
    if domains.assets || domains.configs {
        step(&mut results, "cs2 decompile", || {
            cs2::decompile_cs2_scripts(cache_dir, cs2_root, None).map(|n| format!("{n} scripts"))
        });
    }
    if domains.configs {
        unpack_configs(
            &mut results,
            cache_dir,
            &registry,
            pack_dir,
            &scripts_dir,
            &cs2_dir,
        );
    }
    if domains.maps {
        step(&mut results, "maps", || {
            let mut store = rs_dat2::CacheStore::open(cache_dir)?;
            maps::unpack_maps(&mut store, cache_dir, output_dir).map(|n| format!("{n} map groups"))
        });
    }
    if domains.assets {
        unpack_assets(&mut results, cache_dir, &registry, pack_dir, output_dir);
    }

    // Step 7: Write the run report.
    let report_dir = if domains.assets { output_dir } else { work_dir };
    write_report(&results, &report_dir.join("unpack_report.txt"))
}

/// Steps 4-5 and the animations: every config type, to `scripts/_unpack`.
fn unpack_configs(
    results: &mut Vec<StepResult>,
    cache_dir: &Path,
    registry: &PackRegistry,
    pack_dir: &Path,
    scripts_dir: &Path,
    cs2_dir: &Path,
) {
    let config_out = scripts_dir.join("_unpack");

    // Step 4: Unpack every cache-resident config type to text. Each unpacker
    // walks every cache entry and writes _unpack/all.<type> in the unified
    // 2004scape-style text format.
    step(results, "enum configs", || {
        enums::unpack_enums(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "inv configs", || {
        inv::unpack_invs(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "flu configs", || {
        flu::unpack_flus(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "flo configs", || {
        flo::unpack_flos(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "idk configs", || {
        idk::unpack_idks(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "varbit configs", || {
        varbit::unpack_varbits(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "varp configs", || {
        varp::unpack_varps(cache_dir, registry, &config_out, cs2_dir)?;
        Ok(String::new())
    });
    step(results, "param configs", || {
        param::unpack_params(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "struct configs", || {
        struct_pack::unpack_structs(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "spotanim configs", || {
        spotanim::unpack_spotanims(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "seq configs", || {
        seq::unpack_seqs(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "npc configs", || {
        npc::unpack_npcs(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "obj configs", || {
        obj::unpack_objs(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });
    step(results, "loc configs", || {
        loc::unpack_locs(cache_dir, registry, &config_out)?;
        Ok(String::new())
    });

    // Step 5: Generic config unpackers for archive 2 groups without dedicated
    // CacheType structs. Known types get named opcodes; unknown groups get
    // raw hex dumps.
    step(results, "generic config groups", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        for &(group_id, type_name) in config_generic::KNOWN_CONFIG_GROUPS {
            info!("  Unpacking {type_name} (group {group_id})...");
            config_generic::unpack_config_group(&mut store, group_id, type_name, &config_out)?;
        }
        for &group_id in CONFIG_UNKNOWN_GROUPS {
            info!("  Unpacking config{group_id} (raw)...");
            config_generic::unpack_raw_config_group(&mut store, group_id, &config_out)?;
        }
        Ok(String::new())
    });

    step(results, "animations", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        anim::unpack_anims(&mut store, pack_dir, scripts_dir).map(|n| format!("{n} anim frames"))
    });
}

/// Step 6: the asset domains, each to its own `<domain>/_unpack/`.
fn unpack_assets(
    results: &mut Vec<StepResult>,
    cache_dir: &Path,
    registry: &PackRegistry,
    pack_dir: &Path,
    output_dir: &Path,
) {
    // Step 6: Asset domains. Each writes its own `<domain>/_unpack/` tree
    // under the output dir.
    step(results, "interfaces", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        interfaces::unpack_interfaces(&mut store, output_dir).map(|n| format!("{n} groups"))
    });
    step(results, "models", || {
        let stats = models::unpack_models(
            cache_dir,
            registry,
            &output_dir.join("models").join("_unpack"),
        )?;
        Ok(format!(
            "{} models ({} failed)",
            stats.total(),
            stats.failed
        ))
    });
    step(results, "sprites", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        let stats = sprites::unpack_all_sprites(&mut store, output_dir)?;
        Ok(format!(
            "{} groups, {} sprites ({} failed)",
            stats.groups, stats.sprites, stats.failed
        ))
    });
    step(results, "textures", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        textures_named::unpack_texture_text(&mut store, output_dir).map(|n| format!("{n} groups"))
    });
    step(results, "songs (midi)", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        midi::unpack_songs(&mut store, pack_dir, output_dir).map(|n| format!("{n} songs"))
    });
    step(results, "fontmetrics", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        fontmetrics::unpack_fontmetrics(&mut store, pack_dir, output_dir)
            .map(|n| format!("{n} fonts"))
    });
    step(results, "quickchat", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        quickchat::unpack_quickchat(&mut store, output_dir).map(|n| format!("{n} groups"))
    });
    step(results, "materials", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        materials::unpack_materials(&mut store, output_dir).map(|n| format!("{n} groups"))
    });
    step(results, "particles", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        particles::unpack_particles(&mut store, output_dir).map(|n| format!("{n} groups"))
    });
    step(results, "defaults", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        defaults::unpack_defaults(&mut store, output_dir).map(|n| format!("{n} groups"))
    });
    step(results, "billboards", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        billboards::unpack_billboards(&mut store, output_dir).map(|n| format!("{n} groups"))
    });
    step(results, "shaders", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        shaders::unpack_shaders(&mut store, output_dir).map(|n| format!("{n} groups"))
    });
    step(results, "binary assets", || {
        let mut store = rs_dat2::CacheStore::open(cache_dir)?;
        binary_assets::unpack_all_binary_assets(&mut store, output_dir)
            .map(|n| format!("{n} groups"))
    });
}

fn write_report(results: &[StepResult], report_path: &Path) -> Result<()> {
    let mut report_text = String::from("rev-647 unpack report\n=====================\n");
    let mut failures = 0usize;
    for r in results {
        if !r.ok {
            failures += 1;
        }
        report_text.push_str(&format!(
            "{} {:<24} {}\n",
            if r.ok { "OK  " } else { "FAIL" },
            r.name,
            r.outcome
        ));
    }
    report_text.push_str(&format!("\n{} steps, {} failed\n", results.len(), failures));
    std::fs::write(report_path, &report_text)?;
    info!("Unpack complete. Report: {}", report_path.display());
    if failures > 0 {
        error!("{failures} unpack steps failed — see report");
    }
    Ok(())
}

/// Reorder emitted properties to match the original opcode decode order.
///
/// `opcode_order` is the sequence of opcodes as they appeared in the binary.
/// `props` is a list of (opcode, key, value) triples built by the emitter.
/// Returns just (key, value) pairs in the correct order.
///
/// If `opcode_order` is empty (hand-written config), preserves the original
/// insertion order (which is the fixed emitter order).
pub fn reorder_by_opcode(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
) -> Vec<(String, String)> {
    reorder_by_opcode_with_raw(opcode_order, props, &[])
}

/// Reorder emitted properties to match the original opcode decode order,
/// using raw_opcodes to reconstruct earlier values for duplicate opcodes.
///
/// When an opcode appears N times in `opcode_order` but the emitter only
/// produced M entries (because the struct field stores only the last value),
/// the first (N-M) occurrences are reconstructed from `raw_opcodes`.
/// The emitter's entries are used for the LAST occurrence.
///
/// Uses the generic `decode_raw_to_text` decoder. For type-specific decoding,
/// use `reorder_by_opcode_with_raw_fn`.
pub fn reorder_by_opcode_with_raw(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
    raw_opcodes: &[(u8, Vec<u8>)],
) -> Vec<(String, String)> {
    reorder_by_opcode_with_raw_fn(opcode_order, props, raw_opcodes, decode_raw_to_text)
}

/// Reorder emitted properties using a custom raw-to-text decoder function.
pub fn reorder_by_opcode_with_raw_fn(
    opcode_order: &[u8],
    props: Vec<(u8, String, String)>,
    raw_opcodes: &[(u8, Vec<u8>)],
    decoder: impl Fn(u8, &[u8]) -> Option<(String, String)>,
) -> Vec<(String, String)> {
    if opcode_order.is_empty() {
        return props.into_iter().map(|(_, k, v)| (k, v)).collect();
    }

    // Group emitted props by opcode, preserving insertion order within each group.
    let mut by_op: HashMap<u8, Vec<(String, String)>> = HashMap::new();
    for (op, k, v) in props {
        by_op.entry(op).or_default().push((k, v));
    }

    // Count how many times each opcode appears in the original order.
    let mut op_count_in_order: HashMap<u8, usize> = HashMap::new();
    for &op in opcode_order {
        *op_count_in_order.entry(op).or_default() += 1;
    }

    // For opcodes that appear more times in opcode_order than the emitter
    // produced, build raw-decoded text entries from raw_opcodes.
    // The emitter's entries correspond to the LAST occurrence (since struct
    // fields store the last decoded value). We need entries for the earlier
    // occurrences from raw_opcodes.
    let mut raw_extras: HashMap<u8, Vec<(String, String)>> = HashMap::new();
    if !raw_opcodes.is_empty() {
        for (&op, &needed) in &op_count_in_order {
            let have = by_op.get(&op).map(|v| v.len()).unwrap_or(0);
            if needed > have {
                // Collect all raw entries for this opcode
                let raw_entries: Vec<&Vec<u8>> = raw_opcodes
                    .iter()
                    .filter(|(o, _)| *o == op)
                    .map(|(_, data)| data)
                    .collect();
                // The emitter produced `have` entries using the struct's LAST
                // value. The raw_entries list has ALL occurrences in order.
                // The emitter's entries correspond to raw_entries[needed-have..].
                // We need raw_entries[0..needed-have] as extras.
                let extras_needed = needed - have;
                let mut extras = Vec::new();
                for raw in raw_entries.iter().take(extras_needed) {
                    if let Some(text) = decoder(op, raw) {
                        extras.push(text);
                    }
                }
                if !extras.is_empty() {
                    raw_extras.insert(op, extras);
                }
            }
        }
    }

    // Build queues: raw extras first (for earlier occurrences), then emitter entries.
    let mut by_op_queues: HashMap<u8, std::collections::VecDeque<(String, String)>> =
        HashMap::new();
    for (op, entries) in &by_op {
        by_op_queues.insert(*op, entries.iter().cloned().collect());
    }
    for (op, extras) in &raw_extras {
        let queue = by_op_queues.entry(*op).or_default();
        for (i, entry) in extras.iter().enumerate() {
            queue.insert(i, entry.clone());
        }
    }

    // For each opcode, decide how many text entries to drain per occurrence.
    // - If queue.len() == occurrences in opcode_order: 1 entry per occurrence
    //   (duplicate opcode or multi-instance like seq op19/op20).
    // - If queue.len() > occurrences: batch opcode (e.g. recol produces
    //   recol1s, recol1d, recol2s, recol2d for a single op40 emission).
    //   Compute entries_per = queue.len() / occurrences (integer division).
    //   Drain `entries_per` entries per occurrence, remainder on last.
    let mut result = Vec::new();
    let mut op_occurrence: HashMap<u8, usize> = HashMap::new();

    for &op in opcode_order {
        let queue = match by_op_queues.get_mut(&op) {
            Some(q) if !q.is_empty() => q,
            _ => continue,
        };

        let occ_idx = op_occurrence.entry(op).or_insert(0);
        let total_occ = *op_count_in_order.get(&op).unwrap_or(&1);
        let remaining_occ = total_occ - *occ_idx;

        if remaining_occ <= 1 {
            // Last (or only) occurrence: drain everything remaining
            result.extend(queue.drain(..));
        } else if queue.len() == remaining_occ {
            // 1:1: one entry per remaining occurrence
            if let Some(entry) = queue.pop_front() {
                result.push(entry);
            }
        } else if queue.len() > remaining_occ {
            // Batch: multiple entries per occurrence.
            // Drain a proportional share, rounding down (last gets remainder).
            let per = queue.len() / remaining_occ;
            for _ in 0..per {
                if let Some(entry) = queue.pop_front() {
                    result.push(entry);
                }
            }
        } else {
            // Fewer entries than occurrences (shouldn't happen with raw extras)
            if let Some(entry) = queue.pop_front() {
                result.push(entry);
            }
        }

        *occ_idx += 1;
    }

    result
}

/// Decode raw opcode bytes back to a (key, value) text pair.
/// This handles the common single-value opcodes that appear as duplicates.
/// Returns None for complex/batch opcodes that can't be trivially decoded.
pub fn decode_raw_to_text(opcode: u8, raw: &[u8]) -> Option<(String, String)> {
    // Helper to read big-endian u16
    let r_u16 = |data: &[u8], pos: usize| -> u16 {
        if pos + 1 < data.len() {
            u16::from_be_bytes([data[pos], data[pos + 1]])
        } else {
            0
        }
    };
    let r_i8 =
        |data: &[u8], pos: usize| -> i8 { if pos < data.len() { data[pos] as i8 } else { 0 } };
    let r_u8 = |data: &[u8], pos: usize| -> u8 { if pos < data.len() { data[pos] } else { 0 } };
    let _r_i32 = |data: &[u8], pos: usize| -> i32 {
        if pos + 3 < data.len() {
            i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
        } else {
            0
        }
    };
    let r_i16 = |data: &[u8], pos: usize| -> i16 {
        if pos + 1 < data.len() {
            i16::from_be_bytes([data[pos], data[pos + 1]])
        } else {
            0
        }
    };
    // Read NUL-terminated string
    let r_str = |data: &[u8], pos: usize| -> String {
        let mut end = pos;
        while end < data.len() && data[end] != 0 {
            end += 1;
        }
        String::from_utf8_lossy(&data[pos..end]).to_string()
    };

    match opcode {
        // ── NPC opcodes ──
        // op2: name (string)
        2 => Some(("name".into(), r_str(raw, 0))),
        // op12: size (u8)
        12 => Some(("size".into(), r_u8(raw, 0).to_string())),
        // op30..34: op1..op5 (string) — slot encoded in opcode
        30..=34 => Some((format!("op{}", opcode - 29), r_str(raw, 0))),
        // op93: minimap=no (flag, no payload)
        93 => Some(("minimap".into(), "no".into())),
        // op95: vislevel (u16)
        95 => {
            let v = r_u16(raw, 0);
            if v == 0 {
                Some(("vislevel".into(), "hide".into()))
            } else {
                Some(("vislevel".into(), v.to_string()))
            }
        }
        // op97: resizeh (u16)
        97 => Some(("resizeh".into(), r_u16(raw, 0).to_string())),
        // op98: resizev (u16)
        98 => Some(("resizev".into(), r_u16(raw, 0).to_string())),
        // op99: alwaysontop (flag)
        99 => Some(("alwaysontop".into(), "yes".into())),
        // op100: ambient (i8)
        100 => Some(("ambient".into(), r_i8(raw, 0).to_string())),
        // op101: contrast (i8, raw byte not * 5)
        101 => Some(("contrast".into(), r_i8(raw, 0).to_string())),
        // op102: headicon (u16)
        102 => Some(("headicon".into(), r_u16(raw, 0).to_string())),
        // op103: turnspeed (u16)
        103 => Some(("turnspeed".into(), r_u16(raw, 0).to_string())),

        // ── LOC opcodes ──
        // op14: width (u8)
        14 => Some(("width".into(), r_u8(raw, 0).to_string())),
        // op15: length (u8)
        15 => Some(("length".into(), r_u8(raw, 0).to_string())),
        // op17: blockwalk=no (flag)
        17 => Some(("blockwalk".into(), "no".into())),
        // op18: blockrange=no (flag)
        18 => Some(("blockrange".into(), "no".into())),
        // op19: active (u8) — Note: for LOC this is a single u8, for SEQ it's multi-instance
        19 => Some(("active".into(), r_u8(raw, 0).to_string())),
        // op21: hillskew=yes (flag)
        21 => Some(("hillskew".into(), "yes".into())),
        // op24: anim (u16, 0xFFFF = null)
        24 => {
            let v = r_u16(raw, 0);
            if v == 0xFFFF {
                Some(("anim".into(), "null".into()))
            } else {
                Some(("anim".into(), v.to_string()))
            }
        }
        // op29: ambient (i8) — LOC version
        29 => Some(("ambient".into(), r_i8(raw, 0).to_string())),
        // op39: contrast (i8) — LOC version
        39 => Some(("contrast".into(), r_i8(raw, 0).to_string())),
        // op62: mirror (flag)
        62 => Some(("mirror".into(), "yes".into())),
        // op64: shadow=no (flag)
        64 => Some(("shadow".into(), "no".into())),
        // op65: resizex (u16)
        65 => Some(("resizex".into(), r_u16(raw, 0).to_string())),
        // op66: resizey (u16)
        66 => Some(("resizey".into(), r_u16(raw, 0).to_string())),
        // op67: resizez (u16)
        67 => Some(("resizez".into(), r_u16(raw, 0).to_string())),
        // op69: forceapproach (u8)
        69 => Some(("forceapproach".into(), r_u8(raw, 0).to_string())),
        // op70: offsetx (u16)
        70 => Some(("offsetx".into(), r_u16(raw, 0).to_string())),
        // op71: offsety (u16)
        71 => Some(("offsety".into(), r_u16(raw, 0).to_string())),
        // op72: offsetz (u16)
        72 => Some(("offsetz".into(), r_u16(raw, 0).to_string())),
        // op73: forcedecor (flag)
        73 => Some(("forcedecor".into(), "yes".into())),
        // op74: breakroutefinding (flag)
        74 => Some(("breakroutefinding".into(), "yes".into())),
        // op75: raiseobject (u8)
        75 => Some(("raiseobject".into(), r_u8(raw, 0).to_string())),

        // ── OBJ opcodes ──
        // op1: model (u16) — for OBJ
        1 => Some(("model".into(), r_u16(raw, 0).to_string())),
        // op4: 2dzoom (u16)
        4 => Some(("2dzoom".into(), r_u16(raw, 0).to_string())),
        // op5: 2dxan (u16)
        5 => Some(("2dxan".into(), r_u16(raw, 0).to_string())),
        // op6: 2dyan (u16)
        6 => Some(("2dyan".into(), r_u16(raw, 0).to_string())),
        // op7: 2dxof (i16)
        7 => Some(("2dxof".into(), r_i16(raw, 0).to_string())),
        // op8: 2dyof (i16)
        8 => Some(("2dyof".into(), r_i16(raw, 0).to_string())),
        // op11: stackable (flag)
        11 => Some(("stackable".into(), "yes".into())),
        // op16: members (flag)
        16 => Some(("members".into(), "yes".into())),
        // op23: manwear (u16)
        23 => Some(("manwear".into(), r_u16(raw, 0).to_string())),
        // op25: womanwear (u16)
        25 => Some(("womanwear".into(), r_u16(raw, 0).to_string())),

        // ── Shared opcodes with i32 payload ──
        // op12 for OBJ: cost (i32) — conflicts with NPC op12 (size u8)
        // We can't distinguish here; the caller knows the type.
        // For safety, these ambiguous opcodes should be handled by
        // the type-specific emitter, not here.

        // ── LOC cursor opcodes ──
        // op99 (LOC): cursor1 (u8, u16) — conflicts with NPC op99 (alwaysontop flag)
        // op100 (LOC): cursor2 (u8, u16) — conflicts with NPC op100 (ambient i8)

        // ── Op with u16 payload ──
        // op101 (LOC): mapfunction (u8) — shadowed by NPC op101 (contrast)
        // above; the type-specific emitter handles LOC mapfunction.
        // op122 (NPC): hitbar (u16)
        122 => Some(("hitbar".into(), r_u16(raw, 0).to_string())),
        // op123 (NPC): height (u16)
        123 => Some(("height".into(), r_u16(raw, 0).to_string())),
        // op125 (NPC): respawndir (i8)
        125 => Some(("respawndir".into(), r_i8(raw, 0).to_string())),
        // op127 (NPC): readyanim (u16)
        127 => Some(("readyanim".into(), r_u16(raw, 0).to_string())),
        // op128 (NPC): unknown128 (u8)
        128 => Some(("unknown128".into(), r_u8(raw, 0).to_string())),

        // ── SEQ opcodes (multi-instance, already handled correctly) ──
        // op19 (SEQ): idx:val pair — already pushed to Vec, handled by emitter
        // op20 (SEQ): idx:a:b triple — already pushed to Vec, handled by emitter

        // ── LOC misc ──
        // op82: hidden (flag)
        82 => Some(("hidden".into(), "yes".into())),
        // op88: nominimap_visible (flag)
        88 => Some(("nominimap_visible".into(), "yes".into())),
        // op89: noblocks_npcs (flag)
        89 => Some(("noblocks_npcs".into(), "yes".into())),
        // op90: op90 (flag)
        90 => Some(("op90".into(), "yes".into())),
        // op91: members_only (flag)
        91 => Some(("members_only".into(), "yes".into())),
        // op104: interaction_sides (u8)
        104 => Some(("interaction_sides".into(), r_u8(raw, 0).to_string())),
        // op105: op105 (flag)
        105 => Some(("op105".into(), "yes".into())),
        // op107: mapelement (u16)
        107 => Some(("mapelement".into(), r_u16(raw, 0).to_string())),

        // op164..167: i16/u16 values
        164 => Some(("op164".into(), r_i16(raw, 0).to_string())),
        165 => Some(("op165".into(), r_i16(raw, 0).to_string())),
        166 => Some(("op166".into(), r_i16(raw, 0).to_string())),
        167 => Some(("op167".into(), r_u16(raw, 0).to_string())),
        // op168..169: flags
        168 => Some(("op168".into(), "yes".into())),
        169 => Some(("op169".into(), "yes".into())),
        // op177: flag
        177 => Some(("op177".into(), "yes".into())),
        // op178: u8
        178 => Some(("op178".into(), r_u8(raw, 0).to_string())),

        // ── Complex/batch opcodes: can't trivially decode ──
        _ => None,
    }
}
