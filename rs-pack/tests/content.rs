//! Repository configs are packed directly from source.
#![cfg(rev = "647")]

use rs_pack::cache::varp::VarPlayerScope;
use rs_pack::pack::pack;
use rs_pack::pack::pack_registry::{PackFile, PackRegistry};
use rs_pack::types::ScriptVarType;
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
fn a_registry_rename_stops_packing() {
    let content = scratch("renamed");
    let pack = content.join("pack");
    let output = content.join("output");
    fs::create_dir_all(&pack).unwrap();
    fs::create_dir_all(content.join("scripts/globalscript/_unpack")).unwrap();
    fs::create_dir_all(output.join("server")).unwrap();
    fs::write(pack.join("seq.pack"), "893=human_shearing\n").unwrap();
    fs::write(pack.join("npc.pack"), "1=guard\n").unwrap();
    fs::write(
        content.join("scripts/globalscript/_unpack/all.npc"),
        "[guard]\nmultivar=renamed_varp\nmultinpc=1,guard\n",
    )
    .unwrap();
    fs::write(
        content.join("scripts/globalscript/_unpack/all.seq"),
        "[seq_893]\npriority=5\n",
    )
    .unwrap();
    fs::write(content.join("scripts/globalscript/_unpack/all.loc"), "").unwrap();
    fs::write(output.join("server/seq.dat"), b"previous output").unwrap();

    for verify in [false, true] {
        let err = pack::pack_configs(&content.join("scripts"), &pack, verify)
            .err()
            .expect("stale generated names must stop packing")
            .to_string();
        for expected in [
            "all.seq",
            "seq_893",
            "human_shearing",
            "all.npc:2",
            "reference multivar=renamed_varp",
            "Rename the matching content sections",
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
fn config_crcs_reject_incomplete_source() {
    let content = scratch("config_crc");
    let scripts = content.join("scripts/runescript");
    let pack_dir = content.join("pack");
    fs::create_dir_all(&pack_dir).unwrap();
    fs::create_dir_all(&scripts).unwrap();
    let error = catch_unwind(|| {
        pack::pack_configs(&scripts, &pack_dir, true).unwrap();
    })
    .expect_err("strict packing must reject incomplete configs");
    assert!(
        error
            .downcast_ref::<String>()
            .unwrap()
            .contains("CRC mismatch ['seq']")
    );
    let custom = pack::pack_configs(&scripts, &pack_dir, false).unwrap();
    assert!(custom.contains_key("seq"));
    assert!(!content.join("client").exists());
    assert!(!content.join("server").exists());
}

/// Authored properties of every kind, read back out of the packed configs.
#[test]
#[ignore = "runs the full pack pipeline against content/647 (minutes)"]
fn authored_properties_reach_the_packed_configs() {
    let root = repo_root();
    env::set_current_dir(&root).unwrap();
    let registry = PackRegistry::load(Path::new(rs_pack::PACK_DIR)).unwrap();
    let result = startup::pack_all(
        Path::new(rs_pack::CONTENT_DIR),
        Path::new(rs_pack::PACK_DIR),
        true,
        true,
    )
    .unwrap();
    let config = &result.store;
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
