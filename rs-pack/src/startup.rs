#![allow(clippy::needless_range_loop)]
// Ported (adapted) from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Rev-647 pack pipeline driver — the complete pack+build sequence behind
//! `rs_pack::pack_all` at 647. Ported verbatim from the rev-647 source repo's
//! `startup.rs` (rs_cache → rs_dat2, `crate::cache` → `crate::cache647`).
//!
//! Produces a ready-to-serve JS5 cache plus all loaded config TypeProviders.
//! Uses mtime-based change detection to skip steps whose outputs are
//! already up-to-date relative to their inputs.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Result, bail};
use tracing::{info, warn};

use crate::cache647::{
    CacheType, DbRowType, DbTableType, EnumType, FloType, FluType, HuntType, IdkType, InvType,
    LocType, MesAnimType, NpcType, ObjContext, ObjType, ParamType, SeqType, SpotAnimType,
    StructType, TypeProvider, VarBitType, VarnType, VarpType,
};
use crate::pack_registry::PackRegistry;
use crate::validate::{Severity, check_pack_content_sync};

// ── mtime utilities ──────────────────────────────────────────────────────────

pub(crate) fn newest_mtime(dir: &Path, extensions: &[&str]) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    walk_mtime(dir, extensions, &mut newest);
    newest
}

fn walk_mtime(dir: &Path, extensions: &[&str], newest: &mut Option<SystemTime>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    // A directory's own mtime moves when an entry is added, removed or
    // renamed. Without it a deleted input leaves every remaining file as old
    // as before, the stamp still reads current, and the stale output is
    // served -- hiding `wearpos.obj` once left `pack` reporting "up to date".
    if let Ok(mtime) = dir.metadata().and_then(|m| m.modified()) {
        *newest = Some(newest.map_or(mtime, |n| n.max(mtime)));
    }
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_mtime(&path, extensions, newest);
        } else if path.is_file() {
            let matches = extensions.is_empty()
                || path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| extensions.contains(&ext));
            if matches {
                if let Ok(mtime) = path.metadata().and_then(|m| m.modified()) {
                    *newest = Some(newest.map_or(mtime, |n| n.max(mtime)));
                }
            }
        }
    }
}

pub(crate) fn file_mtime(path: &Path) -> Option<SystemTime> {
    path.metadata().ok().and_then(|m| m.modified().ok())
}

/// Modification time of the running server executable. Folded into the
/// up-to-date inputs so a rebuilt binary (changed *packer logic*, not content)
/// forces a repack — content-only timestamp checks otherwise silently keep
/// stale .dat from an older packer (e.g. enum symbol resolution changing).
pub(crate) fn current_exe_mtime() -> Option<SystemTime> {
    std::env::current_exe().ok().and_then(|p| file_mtime(&p))
}

/// Marks a pack step done, now.
///
/// Writes bytes rather than an empty file. On Windows, truncating a file that
/// is already empty does not move its mtime, so `.anim_stamp`, written as `[]`,
/// kept the date it was first created: once any animation input was newer, all
/// 374,553 frames were repacked on every start and every reload.
pub fn write_stamp(path: &Path) -> std::io::Result<()> {
    std::fs::write(path, b"ok")
}

pub(crate) fn is_up_to_date(sentinel: &Path, newest_input: Option<SystemTime>) -> bool {
    match (file_mtime(sentinel), newest_input) {
        (Some(out), Some(inp)) => out >= inp,
        (Some(_), None) => true,
        _ => false,
    }
}

pub(crate) fn max_of(times: &[Option<SystemTime>]) -> Option<SystemTime> {
    times.iter().filter_map(|t| *t).max()
}

// ── pipeline ─────────────────────────────────────────────────────────────────

/// Result of `pack_all()` -- everything the server needs to start.
pub struct PackResult {
    /// Path to the built JS5 cache directory (for `Js5Cache::preload`).
    pub cache_dir: PathBuf,
    /// Loaded config TypeProviders from server .dat files.
    pub config: PackedConfig,
    /// Component index lookups derived from interface.pack.
    pub layout: InterfaceLayout,
    /// Pre-opened JS5 cache — avoids redundant re-opens during server init.
    pub js5_store: rs_dat2::CacheStore,
}

/// Well-known component indices per toplevel, derived from interface.pack.
///
/// Allows the engine to resolve `if_openmain` etc. to the correct
/// `if_opensub` parent component without hardcoded constants.
#[derive(Debug, Clone, Default)]
pub struct InterfaceLayout {
    /// interface_id → component_id for the `com_main_screen` target.
    pub main_screen: std::collections::HashMap<u16, u16>,
    /// interface_id → component_id for the `com_wide_screen` target: the
    /// toplevel's full-size main slot (548:8 fixed, 746:6 resizable,
    /// `void-634`'s `wide_screen` with 746's +1 drift).
    pub wide_screen: std::collections::HashMap<u16, u16>,
    /// The main-screen interfaces that open in `wide_screen` instead: the
    /// content `wide_screen_enum` (2011's skill guide, whose own backdrop
    /// fills its slot).
    pub wide_screen_ifaces: std::collections::HashSet<u16>,
    /// interface_id → component_id for the `com_overlay_tab` target, the slot
    /// over the side panels that side modals mount on.
    pub overlay_tab: std::collections::HashMap<u16, u16>,
    /// interface_id → component_id for the `com_overlay` target, the
    /// full-size slot over the game view that `if_openoverlay` mounts on
    /// (548:7, 746:8: `void-634`'s `overlay` with 746's +1 drift).
    pub overlay: std::collections::HashMap<u16, u16>,
    /// interface_id → component_id for the `com_chatbox` target.
    pub chatbox: std::collections::HashMap<u16, u16>,
    /// component_id → interface_id (root_layer) for interface listener cleanup.
    pub component_roots: std::collections::HashMap<u16, i32>,
    /// Parent component hash (`chatbox_iface << 16 | dialogue_box_index`) where
    /// dialogue_box interfaces (npc/player chat, message box, multi-choice)
    /// mount. Rev-647: the chatbox interface (752) hosts the dialogue_box at
    /// child component 13 — matching void `dialogue_box parent="chat_box"
    /// index=13`. Dialogues nest *inside* the chatbox frame here, rather than
    /// replacing it at `toplevel:com_chatbox`. 0 if the chatbox interface
    /// wasn't found.
    pub dialogue_parent_hash: i32,
    /// Per-interface override of the chatbox dialogue slot: dialogue interface
    /// id → parent component hash it mounts on. Sourced from the content
    /// `dialogue_slot_enum` (e.g. objbox/doubleobjbox → chatbox child 12,
    /// `dialogue_box_small`). Any dialogue *not* in this map uses
    /// `dialogue_parent_hash` (the default `dialogue_box` slot, child 13).
    pub dialogue_slots: std::collections::HashMap<u16, i32>,
}

impl InterfaceLayout {
    /// Chatbox child component hash an interface mounts on when opened as a
    /// chat dialogue: its `dialogue_slots` override, else the default slot.
    pub fn dialogue_slot(&self, iface: u16) -> i32 {
        self.dialogue_slots
            .get(&iface)
            .copied()
            .unwrap_or(self.dialogue_parent_hash)
    }
}

/// All config TypeProviders loaded from server .dat files.
pub struct PackedConfig {
    pub npcs: TypeProvider<NpcType>,
    pub objs: TypeProvider<ObjType>,
    pub locs: TypeProvider<LocType>,
    pub seqs: TypeProvider<SeqType>,
    pub enums: TypeProvider<EnumType>,
    pub structs: TypeProvider<StructType>,
    pub params: TypeProvider<ParamType>,
    pub varbits: TypeProvider<VarBitType>,
    pub varps: TypeProvider<VarpType>,
    pub idks: TypeProvider<IdkType>,
    pub invs: TypeProvider<InvType>,
    pub hunts: TypeProvider<HuntType>,
    pub flos: TypeProvider<FloType>,
    pub flus: TypeProvider<FluType>,
    pub mesanims: TypeProvider<MesAnimType>,
    pub spotanims: TypeProvider<SpotAnimType>,
    pub varns: TypeProvider<VarnType>,
    pub dbtables: TypeProvider<DbTableType>,
    pub dbrows: TypeProvider<DbRowType>,
    /// Synth name → raw archive-4 sound id (= wire id), from synth.pack.
    /// Synths have no decoded type, so this is the only runtime name source
    /// (used by the cheat arg parser to resolve `synth fire_lit` like `give`).
    pub synth_names: std::collections::HashMap<String, u16>,
}

/// Run the complete pack + build pipeline.
///
/// 1. Pack text configs to server .dat + client blobs
/// 2. Pack animations to .anim files
/// 3. Compile RuneScript sources
/// 4. Build JS5 cache from content + assets
/// 5. Load TypeProviders from the just-written server .dat files
///
/// Steps 1-4 are skipped when their outputs are already newer than all inputs.
///
/// # Arguments
/// * `content_dir` -- root content directory (e.g. `content/647`)
/// * `pack_dir`    -- `.pack` name registries (e.g. `content/647/pack`)
/// * `output_dir`  -- output root (e.g. `target/pack/647`), JS5 cache files land here
/// * `verify`      -- strict mode: panic when a built archive CRC diverges from
///   `crate::archive_crc::EXPECTED` (Jordan's `insert_jag` convention)
pub fn pack_all(
    content_dir: &Path,
    pack_dir: &Path,
    output_dir: &Path,
    verify: bool,
) -> Result<PackResult> {
    pack_all_inner(content_dir, pack_dir, output_dir, verify, false)
}

pub fn pack_all_no_lint(
    content_dir: &Path,
    pack_dir: &Path,
    output_dir: &Path,
    verify: bool,
) -> Result<PackResult> {
    pack_all_inner(content_dir, pack_dir, output_dir, verify, true)
}

/// The cache's own configs and map squares are not kept in git: they are
/// regenerated from `expected/` (`cargo regen-content`) and authored content
/// is laid over them. Without them a pack would silently build a world with
/// no locs, objs or terrain, so refuse instead.
pub fn require_derived_content(content_dir: &Path) -> Result<()> {
    let needed = [
        content_dir.join("scripts").join("_unpack").join("all.loc"),
        content_dir
            .join("maps")
            .join("_unpack")
            .join("manifest.json"),
    ];
    if let Some(missing) = needed.iter().find(|p| !p.exists()) {
        anyhow::bail!(
            "{} is missing. The cache-derived content is regenerated, not \
             committed; run `cargo fetch-expected && cargo regen-content` first.",
            missing.display()
        );
    }
    Ok(())
}

fn pack_all_inner(
    content_dir: &Path,
    pack_dir: &Path,
    output_dir: &Path,
    verify: bool,
    skip_lint: bool,
) -> Result<PackResult> {
    let scripts_dir = content_dir.join("scripts");
    let pack_dir = pack_dir.to_path_buf();
    let server_dir = output_dir.join("server");
    let client_dir = output_dir.join("client");

    require_derived_content(content_dir)?;

    {
        let registry = PackRegistry::load(&pack_dir)?;
        let diags = check_pack_content_sync(content_dir, &registry);
        let errors = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        if !errors.is_empty() {
            bail!(
                "pack registries and content are out of sync:\n{errors}\n\
                 After registry renames, regenerate cache-derived content with \
                 `REV=647 cargo regen-content`."
            );
        }
        if !diags.is_empty() {
            info!(
                "pack_all: {} unregistered content section(s) won't be packed",
                diags.len()
            );
        }
    }

    std::fs::create_dir_all(&server_dir)?;
    std::fs::create_dir_all(&client_dir)?;

    // ── Step 1: Pack text configs → server .dat/.idx + client blobs ───────
    let unpack_dir = scripts_dir.join("_unpack");
    let config_exts: &[&str] = &[
        "npc", "obj", "loc", "seq", "enum", "struct", "param", "varp", "varbit", "inv", "hunt",
        "idk", "flo", "flu", "spotanim", "dbtable", "dbrow", "mesanim", "varn", "vars",
    ];
    let config_input = max_of(&[
        newest_mtime(&unpack_dir, config_exts),
        newest_mtime(&scripts_dir, config_exts),
        newest_mtime(&pack_dir, &["pack"]),
        // A rebuilt server can carry new packer logic with unchanged content;
        // treat the binary itself as an input so configs repack after a rebuild.
        current_exe_mtime(),
    ]);
    let config_sentinel = server_dir.join("npc.dat");

    if is_up_to_date(&config_sentinel, config_input) {
        info!("pack_all: text configs up to date, skipping");
    } else {
        info!("pack_all: packing text configs...");
        crate::pack::pack_all(
            &scripts_dir,
            &pack_dir,
            &server_dir,
            &client_dir,
            false,
            false,
        )?;
    }

    // ── Step 1b: Pack NPC spawns → server map.dat/map.idx ──────────────
    let map_dirs = crate::map_format::source_dirs(content_dir);
    let map_output = server_dir.join("map.dat");
    let map_input = max_of(
        &map_dirs
            .iter()
            .map(|dir| newest_mtime(dir, &["jm2"]))
            .collect::<Vec<_>>(),
    );
    if !is_up_to_date(&map_output, map_input) {
        info!("pack_all: packing NPC spawns...");
        crate::pack::map_spawns::pack(&map_dirs, &server_dir)?;
    } else {
        info!("pack_all: NPC spawns up to date, skipping");
    }

    // ── Step 1c: Pack global obj (ground-item) spawns → objspawn.dat/idx ──
    let objspawn_output = server_dir.join("objspawn.dat");
    if !is_up_to_date(&objspawn_output, map_input) {
        info!("pack_all: packing obj spawns...");
        crate::pack::map_spawns::pack_objs(&map_dirs, &server_dir)?;
    } else {
        info!("pack_all: obj spawns up to date, skipping");
    }

    // ── Step 2: Pack animations → .anim files ────────────────────────────
    // Also captures raw frame entries for passing to build-cache (avoids disk round-trip).
    let anim_input = max_of(&[
        newest_mtime(&unpack_dir.join("animations"), &["anim"]),
        newest_mtime(&unpack_dir, &["base"]),
    ]);
    let anim_stamp = output_dir.join(".anim_stamp");

    let prebuilt_anims = if is_up_to_date(&anim_stamp, anim_input) {
        info!("pack_all: animations up to date, skipping");
        None
    } else {
        info!("pack_all: packing animations...");
        let packed_entries = crate::pack::anim::pack_anims(&scripts_dir, &pack_dir)?;
        info!("pack_all: {} animation frames packed", packed_entries.len());
        write_stamp(&anim_stamp)?;
        Some(packed_entries)
    };

    // ── Step 3: Compile RuneScript → script.dat/script.idx ───────────────
    let script_input = max_of(&[
        newest_mtime(&scripts_dir, &["rs2", "constant"]),
        newest_mtime(&pack_dir, &["pack"]),
    ]);
    let script_sentinel = server_dir.join("script.dat");

    if is_up_to_date(&script_sentinel, script_input) {
        info!("pack_all: RuneScript up to date, skipping");
    } else {
        info!("pack_all: compiling RuneScript...");

        match runec::compile(&scripts_dir, Some(&pack_dir), &server_dir, false) {
            Ok(()) => {
                info!("pack_all: RuneScript compilation succeeded");
                if !skip_lint {
                    info!("pack_all: linting RuneScript...");
                    match runec::lint(&scripts_dir, Some(&pack_dir)) {
                        Ok(()) => info!("pack_all: RuneScript linting succeeded"),
                        Err(e) => warn!("pack_all: RuneScript linting failed: {}", e),
                    }
                }
            }
            Err(e) => warn!("pack_all: RuneScript compilation failed: {}", e),
        }
    }

    // ── Step 4: Build JS5 cache from content + assets ────────────────────
    // Per-archive stamps inside run_build_cache handle incremental skipping.
    // In this repo the raw OpenRS2 786 cache lives in `expected/` (the same
    // dir the unpack side reads); it is only used for group-level diffing
    // when archives diverge.
    let source_cache = Path::new("expected");
    info!("pack_all: building JS5 cache (incremental)...");
    crate::build::run_build_cache(
        source_cache,
        content_dir,
        output_dir,
        &client_dir,
        false,
        prebuilt_anims,
    )?;

    // ── Step 4b: CRC gate against the OpenRS2 786 master index ──────────
    verify_archive_crcs(output_dir, verify)?;

    // ── Step 5: Load TypeProviders (always runs) ─────────────────────────
    info!("pack_all: loading TypeProviders...");
    let mut js5_store = rs_dat2::CacheStore::open(output_dir)?;
    let mut config = load_packed_config(&server_dir, &mut js5_store)?;
    config.synth_names = load_synth_names(&pack_dir);

    info!(
        "pack_all: complete -- {} npcs, {} objs, {} locs, {} seqs, {} enums",
        config.npcs.count(),
        config.objs.count(),
        config.locs.count(),
        config.seqs.count(),
        config.enums.count(),
    );

    let mut layout = load_interface_layout(&pack_dir);
    load_dialogue_slots(&pack_dir, &config.enums, &mut layout);
    load_wide_screen_ifaces(&pack_dir, &config.enums, &mut layout);

    Ok(PackResult {
        cache_dir: output_dir.to_path_buf(),
        config,
        layout,
        js5_store,
    })
}

/// Master-index order of the 35 rev-647 archives: idx255 positions 0..=34 map
/// to archive ids 0-33 then 36 (OpenRS2 786 ships no idx34/idx35). This is the
/// same order the client sends login CRCs in (`Class130.method834`).
pub const MASTER_INDEX_ORDER: [u8; 35] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 36,
];

/// Verify each built archive's reference-table container CRC against
/// [`crate::archive_crc::EXPECTED`].
///
/// The CRC is crc32 over the raw (255, archive_id) container bytes — exactly
/// what the client hashes for its login CRCs. `EXPECTED` is indexed by
/// master-index POSITION (not archive id): entry 34 is archive id 36.
/// `None` entries are known divergences and are skipped. With `strict`,
/// any mismatch panics (Jordan's `insert_jag` convention).
pub fn verify_archive_crcs(output_dir: &Path, strict: bool) -> Result<()> {
    use crate::archive_crc::EXPECTED;

    let mut store = rs_dat2::CacheStore::open(output_dir)?;
    let mut mismatches: Vec<String> = Vec::new();
    info!("CRC gate: verifying 35 archives against OpenRS2 786...");
    for (pos, &aid) in MASTER_INDEX_ORDER.iter().enumerate() {
        let actual = match store.read_raw_group_bytes(255, aid as u32) {
            Ok(Some(raw)) => rs_io::crc::getcrc(&raw, 0, raw.len()),
            _ => 0,
        };
        match EXPECTED[pos] {
            Some(expected) if expected == actual => {
                info!("  archive {aid:>2}: OK ({actual:#010x})");
            }
            Some(expected) => {
                warn!("  archive {aid:>2}: MISMATCH got {actual:#010x}, expected {expected:#010x}");
                mismatches.push(format!(
                    "archive {aid}: got {actual:#010x}, expected {expected:#010x}"
                ));
            }
            None => {
                info!("  archive {aid:>2}: skipped, known divergence ({actual:#010x})");
            }
        }
    }
    if mismatches.is_empty() {
        info!("CRC gate: all 31 verified archives match OpenRS2 786 (4 known divergences skipped)");
    } else if strict {
        panic!(
            "CRC gate: {} archive(s) diverge from OpenRS2 786:\n  {}",
            mismatches.len(),
            mismatches.join("\n  "),
        );
    } else {
        warn!(
            "CRC gate: {} archive(s) diverge from OpenRS2 786",
            mismatches.len()
        );
    }
    Ok(())
}

/// Load all config TypeProviders from server .dat files + JS5 cache.
///
/// Types whose server packer writes the same opcode format as the CacheType
/// decoder are loaded directly from server .dat. Types where the packer uses
/// per-entry opcodes (vs batched) are loaded from the just-built JS5 cache.
pub fn load_packed_config(
    server_dir: &Path,
    cache: &mut rs_dat2::CacheStore,
) -> Result<PackedConfig> {
    // Types loaded from server .dat (opcode format matches decoder)
    let npcs = load_from_server_dat(server_dir, "npc", ())?;
    let objs = load_from_server_dat(server_dir, "obj", ObjContext { members: true })?;
    let seqs = load_from_server_dat(server_dir, "seq", ())?;
    let params = load_from_server_dat(server_dir, "param", ())?;
    let varbits = load_from_server_dat(server_dir, "varbit", ())?;
    let varps = load_from_server_dat(server_dir, "varp", ())?;
    let idks = load_from_server_dat(server_dir, "idk", ())?;
    let hunts = load_from_server_dat(server_dir, "hunt", ())?;
    let flos = load_from_server_dat(server_dir, "flo", ())?;
    let flus = load_from_server_dat(server_dir, "flu", ())?;
    let spotanims = load_from_server_dat(server_dir, "spotanim", ())?;
    let invs = load_from_server_dat(server_dir, "inv", ())?;
    let mesanims = load_from_server_dat(server_dir, "mesanim", ())?;
    let varns = load_from_server_dat(server_dir, "varn", ())?;
    let dbtables = load_from_server_dat(server_dir, "dbtable", ())?;
    let dbrows = load_from_server_dat(server_dir, "dbrow", ())?;

    // Types loaded from JS5 cache — either because the server packer uses
    // an incompatible format (enum/struct: unbatched opcodes) or because
    // the server .dat only has a subset of fields (locs: missing collision
    // fields like blockwalk/width/length/active which are client opcodes).
    let mut locs = crate::cache647::loader::load_all_locs(cache)?;
    let mut enums = crate::cache647::loader::load_all_enums(cache)?;
    let mut structs = crate::cache647::loader::load_all_structs(cache)?;
    info!("  locs: {} entries (from JS5 cache)", locs.count());

    // Server-only enums (e.g. [stats]) are packed into server enum.dat but
    // deliberately kept out of client archive 17 (crc-gate). Append the ids
    // beyond the JS5 range so scripts can read them at runtime.
    let enum_overlay: TypeProvider<EnumType> = load_from_server_dat(server_dir, "enum", ())?;
    if enum_overlay.count() > enums.count() {
        let mut types = std::mem::take(&mut enums.types).into_vec();
        let mut overlay_types = enum_overlay.types.into_vec();
        let appended = overlay_types.split_off(types.len());
        let added = appended.len();
        types.extend(appended);
        enums.types = types.into_boxed_slice();
        info!("  enums: {} server-only entries appended", added);
    }

    // Server-only structs likewise (289's fishing data, `fishing_struct_*`):
    // in server struct.dat, out of the client archive, appended past the JS5
    // range.
    let struct_overlay: TypeProvider<StructType> = load_from_server_dat(server_dir, "struct", ())?;
    if struct_overlay.count() > structs.count() {
        let mut types = std::mem::take(&mut structs.types).into_vec();
        let mut overlay_types = struct_overlay.types.into_vec();
        let appended = overlay_types.split_off(types.len());
        let added = appended.len();
        types.extend(appended);
        structs.types = types.into_boxed_slice();
        info!("  structs: {} server-only entries appended", added);
    }

    // Merge server loc overlay (category, ops, params, desc) into JS5 locs
    let loc_overlay: TypeProvider<LocType> = load_from_server_dat(server_dir, "loc", ())?;
    let mut loc_overlays_merged = 0u32;
    for id in 0..loc_overlay.count() as u16 {
        if let Some(overlay) = loc_overlay.get_by_id(id) {
            if let Some(base) = locs.get_by_id_mut(id) {
                let mut merged = false;
                if overlay.category != -1 {
                    base.category = overlay.category;
                    merged = true;
                }
                if overlay.desc.is_some() {
                    base.desc = overlay.desc.clone();
                    merged = true;
                }
                for (i, op) in overlay.op.iter().enumerate() {
                    if op.is_some() {
                        base.op[i] = op.clone();
                        merged = true;
                    }
                }
                if let Some(ref overlay_params) = overlay.params {
                    let params = base
                        .params
                        .get_or_insert_with(std::collections::HashMap::new);
                    for (k, v) in overlay_params.iter() {
                        params.insert(*k, v.clone());
                    }
                    merged = true;
                }
                if merged {
                    loc_overlays_merged += 1;
                }
            }
        }
    }
    info!(
        "  locs: {} server overlays merged (category, ops, params, desc)",
        loc_overlays_merged
    );
    info!("  enums: {} entries (from JS5 cache)", enums.count());
    info!("  structs: {} entries (from JS5 cache)", structs.count());

    Ok(PackedConfig {
        npcs,
        objs,
        locs,
        seqs,
        enums,
        structs,
        params,
        varbits,
        varps,
        idks,
        invs,
        hunts,
        flos,
        flus,
        mesanims,
        spotanims,
        varns,
        dbtables,
        dbrows,
        synth_names: Default::default(),
    })
}

/// Load synth name → id from `synth.pack` (`<id>=<name>` lines). Since the
/// stride-1 fix the pack id IS the wire id, so the map can be used directly.
pub fn load_synth_names(pack_dir: &Path) -> std::collections::HashMap<String, u16> {
    let mut map = std::collections::HashMap::new();
    if let Ok(contents) = std::fs::read_to_string(pack_dir.join("synth.pack")) {
        for line in contents.lines() {
            if let Some((id, name)) = line.trim().split_once('=')
                && let Ok(id) = id.parse::<u16>()
            {
                map.insert(name.to_string(), id);
            }
        }
    }
    map
}

/// Rev-647 chatbox child component that hosts dialogue_box interfaces
/// (npc/player chat, message box, multi-choice). Matches void's
/// `dialogue_box parent="chat_box" index=13`.
const DIALOGUE_BOX_INDEX: i32 = 13;

/// Scan interface.pack for well-known component names and build an [`InterfaceLayout`].
fn load_interface_layout(pack_dir: &Path) -> InterfaceLayout {
    let mut layout = InterfaceLayout::default();
    let path = pack_dir.join("interface.pack");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return layout;
    };
    for line in contents.lines() {
        let line = line.trim();
        let Some((id_str, name)) = line.split_once('=') else {
            continue;
        };
        let Some((iface_str, comp_str)) = id_str.split_once(':') else {
            // Top-level interface line `<id>=<name>` (no component). Capture the
            // chatbox interface id so dialogues can mount on its dialogue_box
            // child instead of clobbering the chatbox at `toplevel:com_chatbox`.
            if name == "chatbox"
                && let Ok(iface) = id_str.parse::<u16>()
            {
                layout.dialogue_parent_hash = ((iface as i32) << 16) | DIALOGUE_BOX_INDEX;
            }
            continue;
        };
        let (Ok(iface), Ok(comp)) = (iface_str.parse::<u16>(), comp_str.parse::<u16>()) else {
            continue;
        };
        if name == "com_main_screen" {
            layout.main_screen.insert(iface, comp);
        } else if name == "com_wide_screen" {
            layout.wide_screen.insert(iface, comp);
        } else if name == "com_overlay_tab" {
            layout.overlay_tab.insert(iface, comp);
        } else if name == "com_overlay" {
            layout.overlay.insert(iface, comp);
        } else if name == "com_chatbox" {
            layout.chatbox.insert(iface, comp);
        }
        layout.component_roots.insert(comp, iface as i32);
    }
    layout
}

/// Populate `layout.dialogue_slots` from the content `dialogue_slot_enum`.
///
/// The enum maps a dialogue interface id (key) → the chatbox child component
/// hash it mounts on (value), e.g. objbox/doubleobjbox →
/// `chatbox:com_dialogue_box_small` (752:12). The value is already a full
/// component hash (resolved symbolically at pack time), so it is the parent the
/// engine opens against — no slot-index math. The default `dialogue_box` slot
/// (752:13) is implicit; only exceptions appear in the enum. It's a server-only
/// enum, so its id is resolved from `enum.pack` by name and read out of the
/// already-loaded enum TypeProvider. Absent/empty enum → no overrides.
fn load_dialogue_slots(
    pack_dir: &Path,
    enums: &TypeProvider<EnumType>,
    layout: &mut InterfaceLayout,
) {
    const ENUM_NAME: &str = "dialogue_slot_enum";
    let Ok(contents) = std::fs::read_to_string(pack_dir.join("enum.pack")) else {
        return;
    };
    let enum_id = contents.lines().find_map(|line| {
        let (id, name) = line.trim().split_once('=')?;
        (name == ENUM_NAME)
            .then(|| id.parse::<u16>().ok())
            .flatten()
    });
    let Some(enum_id) = enum_id else { return };
    let Some(def) = enums.get_by_id(enum_id) else {
        return;
    };
    for (&iface, value) in &def.entries {
        if let crate::cache647::enum_type::EnumValue::Int(parent_hash) = value {
            layout.dialogue_slots.insert(iface as u16, *parent_hash);
        }
    }
    info!(
        "  dialogue slots: {} override(s) from {ENUM_NAME}",
        layout.dialogue_slots.len()
    );
}

/// Populate `layout.wide_screen_ifaces` from the content `wide_screen_enum`:
/// interface id (key) → true for each main-screen interface 2011 opens in the
/// toplevel's full-size slot. Absent/empty enum → none.
fn load_wide_screen_ifaces(
    pack_dir: &Path,
    enums: &TypeProvider<EnumType>,
    layout: &mut InterfaceLayout,
) {
    const ENUM_NAME: &str = "wide_screen_enum";
    let Ok(contents) = std::fs::read_to_string(pack_dir.join("enum.pack")) else {
        return;
    };
    let Some(def) = contents
        .lines()
        .find_map(|line| {
            let (id, name) = line.trim().split_once('=')?;
            (name == ENUM_NAME)
                .then(|| id.parse::<u16>().ok())
                .flatten()
        })
        .and_then(|id| enums.get_by_id(id))
    else {
        return;
    };
    for (&iface, value) in &def.entries {
        if let crate::cache647::enum_type::EnumValue::Int(1) = value {
            layout.wide_screen_ifaces.insert(iface as u16);
        }
    }
}

/// Load a single config type from server .dat/.idx files into a TypeProvider.
///
/// PackedData format: dat has `p2(count)` header then concatenated entry bytes.
/// idx has `p2(count)` header then `p2(len)` per entry (0 = absent entry).
/// Each entry's bytes are opcode-terminated (ending with `p1(0)`).
pub fn load_from_server_dat<T: CacheType>(
    server_dir: &Path,
    name: &str,
    ctx: T::Context,
) -> Result<TypeProvider<T>> {
    let dat_path = server_dir.join(format!("{name}.dat"));
    let idx_path = server_dir.join(format!("{name}.idx"));
    if !dat_path.exists() {
        info!("  {name}.dat not found, returning empty TypeProvider");
        return Ok(TypeProvider::default());
    }
    let dat = std::fs::read(&dat_path)?;
    let idx = std::fs::read(&idx_path)?;

    if dat.len() < 2 || idx.len() < 2 {
        return Ok(TypeProvider::default());
    }

    let count = u16::from_be_bytes([dat[0], dat[1]]) as usize;
    let mut types: Vec<T> = (0..count).map(|i| T::new(i as u16)).collect();
    let mut names = std::collections::HashMap::new();
    let mut dat_offset = 2usize;

    for id in 0..count {
        let idx_off = 2 + id * 2;
        if idx_off + 1 >= idx.len() {
            break;
        }
        let entry_len = u16::from_be_bytes([idx[idx_off], idx[idx_off + 1]]) as usize;

        if entry_len > 0 && dat_offset + entry_len <= dat.len() {
            let mut buf = rs_io::Packet::from(dat[dat_offset..dat_offset + entry_len].to_vec());
            types[id].decode(&mut buf);
            if let Some(name) = types[id].debugname() {
                names.insert(name.into(), id as u16);
            }
        }
        dat_offset += entry_len;
    }

    T::post_decode(&mut types, &ctx);

    let provider = TypeProvider {
        names,
        types: types.into_boxed_slice(),
    };
    info!("  {name}: {} entries loaded", provider.count());
    Ok(provider)
}
