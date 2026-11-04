//! The cache's own content is derived, not committed.
//!
//! `scripts/_unpack` and `maps/_unpack` are regenerated from `expected/` by
//! `cargo regen-content`; authored content lives beside them and is laid over
//! them at pack time (`pack::overlay_section`, `map_format::source_dirs`).
//! These tests pin both halves: a regeneration is a function of the cache and
//! the registries alone, and every kind of authored overlay still reaches the
//! packed definitions.
//!
//! The ignored cases need `expected/` (`cargo fetch-expected`) and the
//! regenerated content:
//! `REV=647 cargo test -p rs-pack --release --test derived_content -- --ignored`
#![cfg(rev = "647")]

use std::fs;
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("derived_content")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn pack_refuses_to_run_without_the_derived_content() {
    let content = scratch("missing");
    let err = rs_pack::startup::require_derived_content(&content).unwrap_err();
    assert!(err.to_string().contains("cargo regen-content"), "{err}");

    fs::create_dir_all(content.join("scripts/_unpack")).unwrap();
    fs::write(content.join("scripts/_unpack/all.loc"), "").unwrap();
    let err = rs_pack::startup::require_derived_content(&content).unwrap_err();
    assert!(err.to_string().contains("manifest.json"), "{err}");

    fs::create_dir_all(content.join("maps/_unpack")).unwrap();
    fs::write(content.join("maps/_unpack/manifest.json"), "{}").unwrap();
    rs_pack::startup::require_derived_content(&content).unwrap();
}

#[test]
fn a_registry_rename_stops_packing_before_outputs_change() {
    let content = scratch("renamed");
    let pack = content.join("pack");
    let output = content.join("output");
    fs::create_dir_all(&pack).unwrap();
    fs::create_dir_all(content.join("scripts/_unpack")).unwrap();
    fs::create_dir_all(content.join("maps/_unpack")).unwrap();
    fs::create_dir_all(output.join("server")).unwrap();
    fs::write(pack.join("seq.pack"), "893=human_shearing\n").unwrap();
    fs::write(
        content.join("scripts/_unpack/all.seq"),
        "[seq_893]\npriority=5\n",
    )
    .unwrap();
    fs::write(content.join("scripts/_unpack/all.loc"), "").unwrap();
    fs::write(content.join("maps/_unpack/manifest.json"), "{}").unwrap();
    fs::write(output.join("server/seq.dat"), b"previous output").unwrap();

    for verify in [false, true] {
        let err = rs_pack::startup::pack_all(&content, &pack, &output, verify)
            .err()
            .expect("stale generated names must stop packing")
            .to_string();
        for expected in [
            "all.seq",
            "seq_893",
            "human_shearing",
            "cargo regen-content",
        ] {
            assert!(err.contains(expected), "{err}");
        }
        assert_eq!(
            fs::read(output.join("server/seq.dat")).unwrap(),
            b"previous output"
        );
        assert!(!output.join("client").exists());
    }
}

/// One authored overlay of every kind, read back out of the packed configs.
#[test]
#[ignore = "runs the full pack pipeline against content/647 (minutes)"]
fn every_kind_of_authored_overlay_reaches_the_packed_configs() {
    let root = repo_root();
    std::env::set_current_dir(&root).unwrap();
    let registry =
        rs_pack::pack_registry::PackRegistry::load(Path::new(rs_pack::PACK_DIR)).unwrap();
    // A fresh output directory: the incremental stamps must not decide what
    // this test sees.
    let output = scratch("pack");
    let result = rs_pack::startup::pack_all(
        Path::new(rs_pack::CONTENT_DIR),
        Path::new(rs_pack::PACK_DIR),
        &output,
        false,
    )
    .unwrap();
    let config = &result.config;
    let id = |pack: &rs_pack::pack_registry::PackFile, name: &str| -> u16 {
        pack.get_by_name(name)
            .unwrap_or_else(|| panic!("{name} is not registered")) as u16
    };

    // player/configs/wearpos.obj -- the cache has no wearpos opcode.
    let platebody = config
        .objs
        .get_by_id(id(&registry.obj, "bronze_platebody"))
        .unwrap();
    assert_eq!(platebody.wearpos1, 4, "torso");

    // general_use/configs/barriers.loc -- a category on a cache loc.
    let gate = config
        .locs
        .get_by_id(id(&registry.loc, "fishinggateclosedl"))
        .unwrap();
    assert_eq!(
        gate.category,
        registry.category.get_by_name("gate_main_closed").unwrap() as i32
    );

    // general/configs/npc_behaviour.npc -- movement on a cache npc.
    let hans = config.npcs.get_by_id(id(&registry.npc, "hans")).unwrap();
    assert_eq!(hans.max_range, 50);

    // general/configs/shops.inv -- stock on a cache inv.
    let shop = config.invs.get_by_id(id(&registry.inv, "inv_486")).unwrap();
    assert_eq!(
        shop.stock_obj.first().copied(),
        Some(id(&registry.obj, "junk_19713"))
    );

    // player/configs/perm.varp -- persistence on a cache varp.
    let varp = config
        .varps
        .get_by_id(id(&registry.varp, "learning_the_ropes"))
        .unwrap();
    assert_eq!(
        varp.scope,
        rs_pack::cache647::varp_type::VarPlayerScope::Perm
    );

    // donor/configs/donor.loc -- a whole definition 786 does not have.
    assert!(config.locs.get_by_id(864).is_some(), "donor loc_864");

    // ... and a cache definition that is no longer dropped (config_loc 179).
    assert!(
        config
            .locs
            .get_by_id(id(&registry.loc, "border_guard"))
            .is_some(),
        "border_guard"
    );
}

/// A regeneration writes the same `_unpack` trees the content holds, and
/// leaves the name registries byte-for-byte as committed. Either failing means
/// a hand edit crept into `_unpack`, where the next regeneration would lose it.
#[test]
#[ignore = "unpacks expected/ (minutes)"]
fn regeneration_is_pure_and_keeps_the_registries() {
    let root = repo_root();
    let content = root.join(rs_pack::CONTENT_DIR);
    let out = scratch("regen");
    let pack = out.join("pack");
    fs::create_dir_all(&pack).unwrap();
    for entry in fs::read_dir(content.join("pack")).unwrap() {
        let path = entry.unwrap().path();
        fs::copy(&path, pack.join(path.file_name().unwrap())).unwrap();
    }

    rs_pack::unpack::unpack(
        &root.join("expected"),
        &out,
        &pack,
        rs_pack::unpack::Domains::parse("configs,maps").unwrap(),
        &out.join("work"),
    )
    .unwrap();

    for entry in fs::read_dir(content.join("pack")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap();
        assert!(
            fs::read(&path).unwrap() == fs::read(pack.join(name)).unwrap(),
            "unpack rewrote registry {name:?}"
        );
    }
    for dir in ["scripts/_unpack", "maps/_unpack"] {
        assert_same_tree(&content.join(dir), &out.join(dir));
    }
}

fn assert_same_tree(want: &Path, got: &Path) {
    let mut want_files = Vec::new();
    walk(want, want, &mut want_files);
    let mut got_files = Vec::new();
    walk(got, got, &mut got_files);
    want_files.sort();
    got_files.sort();
    assert_eq!(
        want_files,
        got_files,
        "file sets differ under {}",
        want.display()
    );
    for rel in &want_files {
        assert!(
            fs::read(want.join(rel)).unwrap() == fs::read(got.join(rel)).unwrap(),
            "{} differs from a fresh regeneration",
            want.join(rel).display()
        );
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(root, &path, out);
        } else {
            out.push(path.strip_prefix(root).unwrap().to_path_buf());
        }
    }
}
