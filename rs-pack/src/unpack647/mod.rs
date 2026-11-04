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
pub mod hunt;
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

use std::path::Path;

use anyhow::Result;
use tracing::{error, info};

use crate::pack_registry::PackRegistry;

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
    step(results, "hunt configs", || {
        hunt::unpack_hunts(cache_dir, registry, &config_out)?;
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
        for &group_id in config_generic::UNKNOWN_CONFIG_GROUPS {
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
