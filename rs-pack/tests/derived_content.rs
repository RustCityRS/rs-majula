//! The cache's own content is derived, not committed.
//!
//! `scripts/_unpack` and `maps/_unpack` are regenerated from `expected/` by
//! `cargo regen-content`; authored content lives beside them and is laid over
//! them at pack time (`pack::pack::overlay_section`, `pack::other::map::source_dirs`).
//! These tests pin both halves: a regeneration is a function of the cache and
//! the registries alone, and every kind of authored overlay still reaches the
//! packed definitions.
//!
//! The ignored cases need `expected/` (`cargo fetch-expected`) and the
//! regenerated content:
//! `REV=647 cargo test -p rs-pack --test derived_content -- --ignored`
#![cfg(rev = "647")]

use rs_io::jag::{JagCompression, JagFile};
use rs_pack::cache::varp::VarPlayerScope;
use rs_pack::pack::pack;
use rs_pack::pack::pack_registry::{PackFile, PackRegistry};
use rs_pack::types::ScriptVarType;
use rs_pack::unpack::{self, Domains};
use rs_pack::{ParamValue, startup};
use std::panic::catch_unwind;
use std::path::{Path, PathBuf};
use std::{env, fs};

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
    let err = startup::require_derived_content(&content).unwrap_err();
    assert!(err.to_string().contains("cargo regen-content"), "{err}");

    fs::create_dir_all(content.join("scripts/_unpack")).unwrap();
    fs::write(content.join("scripts/_unpack/all.loc"), "").unwrap();
    let err = startup::require_derived_content(&content).unwrap_err();
    assert!(err.to_string().contains("manifest.json"), "{err}");

    fs::create_dir_all(content.join("maps/_unpack")).unwrap();
    fs::write(content.join("maps/_unpack/manifest.json"), "{}").unwrap();
    startup::require_derived_content(&content).unwrap();
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
    fs::write(pack.join("npc.pack"), "1=guard\n").unwrap();
    fs::write(
        content.join("scripts/_unpack/all.npc"),
        "[guard]\nmultivarp=renamed_varp\nmultivar=null\nmultidefault=-1\nmulti1=1\n",
    )
    .unwrap();
    fs::write(
        content.join("scripts/_unpack/all.seq"),
        "[seq_893]\npriority=5\n",
    )
    .unwrap();
    fs::write(content.join("scripts/_unpack/all.loc"), "").unwrap();
    fs::write(content.join("maps/_unpack/manifest.json"), "{}").unwrap();
    fs::write(output.join("server/seq.dat"), b"previous output").unwrap();

    for verify in [false, true] {
        let err = startup::pack_all(&content, &pack, &output, verify)
            .err()
            .expect("stale generated names must stop packing")
            .to_string();
        for expected in [
            "all.seq",
            "seq_893",
            "human_shearing",
            "all.npc:2",
            "reference multivarp=renamed_varp",
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

#[test]
fn config_crcs_are_checked_before_saving_and_on_cached_startup() {
    let content = scratch("config_crc");
    let scripts = content.join("scripts");
    let pack_dir = content.join("pack");
    let output = content.join("output");
    let server = output.join("server");
    let client = output.join("client");
    fs::create_dir_all(&pack_dir).unwrap();
    fs::create_dir_all(scripts.join("_unpack")).unwrap();
    fs::create_dir_all(content.join("maps/_unpack")).unwrap();
    fs::write(scripts.join("_unpack/all.loc"), "").unwrap();
    fs::write(content.join("maps/_unpack/manifest.json"), "{}").unwrap();

    let error = catch_unwind(|| {
        pack::pack_all(&scripts, &pack_dir, &server, &client, true).unwrap();
    })
    .expect_err("strict packing must reject incomplete configs");
    assert!(
        error
            .downcast_ref::<String>()
            .unwrap()
            .contains("CRC mismatch ['seq']")
    );
    assert!(!server.join("seq.dat").exists());
    assert!(!client.join("config").exists());

    pack::pack_all(&scripts, &pack_dir, &server, &client, false).unwrap();
    let config_path = client.join("config");
    let mut config = JagFile::new();
    config.write("seq.dat", vec![1, 2, 3]);
    fs::write(&config_path, config.build(JagCompression::PerFile)).unwrap();
    let saved = fs::read(&config_path).unwrap();
    let error = catch_unwind(|| {
        startup::pack_all(&content, &pack_dir, &output, true).unwrap();
    })
    .expect_err("strict startup must verify the cached bytes");
    let message = error.downcast_ref::<String>().unwrap();
    assert!(
        message.contains("CRC mismatch ['seq']: Got: 1438416925"),
        "{message}"
    );
    assert_eq!(fs::read(config_path).unwrap(), saved);
}

/// One authored overlay of every kind, read back out of the packed configs.
#[test]
#[ignore = "runs the full pack pipeline against content/647 (minutes)"]
fn every_kind_of_authored_overlay_reaches_the_packed_configs() {
    let root = repo_root();
    env::set_current_dir(&root).unwrap();
    let registry = PackRegistry::load(Path::new(rs_pack::PACK_DIR)).unwrap();
    // A fresh output directory: the incremental stamps must not decide what
    // this test sees.
    let output = scratch("pack");
    let result = startup::pack_all(
        Path::new(rs_pack::CONTENT_DIR),
        Path::new(rs_pack::PACK_DIR),
        &output,
        true,
    )
    .unwrap();
    let config = &result.config;
    let id = |pack: &PackFile, name: &str| -> u16 {
        pack.get_by_debugname(name)
            .unwrap_or_else(|| panic!("{name} is not registered")) as u16
    };

    assert_eq!(config.categories.count(), registry.category.max as usize);
    for (&id, name) in &registry.category.id_to_debugname {
        assert_eq!(
            config.categories.get_by_debugname(name).unwrap().id,
            id as u16
        );
    }

    // player/configs/wearpos.obj -- the cache has no wearpos opcode.
    let platebody = config
        .objs
        .get_by_id(id(&registry.obj, "bronze_platebody"))
        .unwrap();
    assert_eq!(platebody.wearpos.map(|v| v as u8), Some(4), "torso");

    // general_use/configs/barriers.loc -- a category on a cache loc.
    let gate = config
        .locs
        .get_by_id(id(&registry.loc, "fishinggateclosedl"))
        .unwrap();
    assert_eq!(
        gate.category,
        Some(
            registry
                .category
                .get_by_debugname("gate_main_closed")
                .unwrap() as u16
        )
    );

    // general/configs/npc_behaviour.npc -- movement on a cache npc.
    let hans = config.npcs.get_by_id(id(&registry.npc, "hans")).unwrap();
    assert_eq!(hans.maxrange, 50);

    let hellhound = config.npcs.get_by_debugname("hellhound").unwrap();
    assert_eq!(hellhound.huntmode, Some(id(&registry.hunt, "cowardly")));
    assert_eq!(
        config.hunts.debugnames.len(),
        registry.hunt.id_to_debugname.len()
    );

    let unlocks = config
        .enums
        .get_by_debugname("levelup_unlocks_attack")
        .unwrap();
    assert_eq!(unlocks.inputtype, ScriptVarType::Int);
    assert_eq!(unlocks.values.len(), 6);
    for (key, expected) in [5, 10, 20, 30, 40, 60].into_iter().enumerate() {
        assert!(
            matches!(unlocks.values[&(key as i32)], ParamValue::Int(value) if value == expected)
        );
    }

    // general/configs/shops.inv -- stock on a cache inv.
    let shop = config.invs.get_by_id(id(&registry.inv, "inv_486")).unwrap();
    assert_eq!(
        shop.stockobj.as_deref().unwrap().first().copied(),
        Some(id(&registry.obj, "junk_19713"))
    );

    let tackler = config
        .hunts
        .get_by_id(id(&registry.hunt, "gnomeball_tackler"))
        .unwrap();
    let check = tackler.check_inv.as_ref().unwrap();
    assert_eq!(check.inv, id(&registry.inv, "worn"));
    assert_eq!(check.obj, id(&registry.obj, "gnomeball"));
    assert_eq!((check.condition.as_str(), check.value), (">", 0));
    let gujuo = config.hunts.get_by_id(id(&registry.hunt, "gujuo")).unwrap();
    assert_eq!(gujuo.extracheck_vars.len(), 2);
    for (check, expected) in gujuo.extracheck_vars.iter().zip([(">", 0), ("<", 75)]) {
        assert_eq!(check.varp, id(&registry.varp, "legendsquest"));
        assert_eq!((check.condition.as_str(), check.value), expected);
    }

    // player/configs/perm.varp -- persistence on a cache varp.
    let varp = config
        .varps
        .get_by_id(id(&registry.varp, "learning_the_ropes"))
        .unwrap();
    assert_eq!(varp.scope, VarPlayerScope::Perm);

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
    env::set_current_dir(&root).unwrap();
    let content = root.join(rs_pack::CONTENT_DIR);
    let out = scratch("regen");
    let pack = out.join("pack");
    fs::create_dir_all(&pack).unwrap();
    for entry in fs::read_dir(content.join("pack")).unwrap() {
        let path = entry.unwrap().path();
        fs::copy(&path, pack.join(path.file_name().unwrap())).unwrap();
    }

    unpack::unpack(
        &root.join("expected"),
        &out,
        &pack,
        Domains::parse("configs,maps").unwrap(),
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
