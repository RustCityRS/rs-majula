//! Picking the field wheat of 530, 647 and 742.
//!
//! Besides `wheat` (313), these caches' maps place field wheat under
//! `fai_varrock_wheat` and its corner and small pieces and `brain_farm_wheat`
//! (Mos Le'Harmless), each `name=Wheat`, `op2=Pick` in the rev's own
//! `all.loc`. 742 adds `wheat_67266`..`wheat_67268` in Taverley's field.
//! Before, only `wheat` had a Pick trigger, so the rest said "Nothing
//! interesting happens."; now each gives grain as `wheat` does in that rev.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::{runner, world};
use rs_ver::TypeId;

static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// One placement of each field wheat piece, the same in 530's, 647's and
/// 742's maps: `l49_51` 0 18 33, 0 18 32 (corner), 0 19 38 (small);
/// `l59_44` 0 29 34.
const FIELD_WHEAT: [(&str, u16, u16); 4] = [
    ("fai_varrock_wheat", 3154, 3297),
    ("fai_varrock_wheat_corner", 3154, 3296),
    ("fai_varrock_wheat_small", 3155, 3302),
    ("brain_farm_wheat", 3805, 2850),
];

/// 742's Taverley wheat (`l45_53`: 0 10 3, 0 10 2, 0 11 3).
#[cfg(since_742)]
const TAVERLEY_WHEAT: [(&str, u16, u16); 3] = [
    ("wheat_67266", 2890, 3395),
    ("wheat_67267", 2890, 3394),
    ("wheat_67268", 2891, 3395),
];

fn loc_id(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .locs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no loc {name}"))
        .id
}

/// Whether the loc `name` stands at `x`,0,`z` now.
fn loc_at(engine: &Engine, name: &str, x: u16, z: u16) -> bool {
    let loc = loc_id(engine, name);
    engine
        .zones
        .zone(x, 0, z)
        .is_some_and(|zone| zone.get_loc(x, z, loc).is_some())
}

fn grain_held(engine: &Engine, pid: u16) -> u32 {
    let grain = engine.cache.objs.get_by_debugname("grain").unwrap().id;
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let backpack = engine.harness_player(pid).unwrap().invs.get(&inv).unwrap();
    (0..backpack.capacity as u16)
        .filter_map(|s| backpack.get(s))
        .filter(|i| i.obj == grain)
        .map(|i| i.num)
        .sum()
}

/// A fresh player beside the wheat picks it: they are told "You pick some
/// grain.", hold one grain, and the wheat is gone.
fn picks_grain(engine: &mut Engine, name: &str, x: u16, z: u16) {
    engine.harness_reset();
    let mut client = engine.harness_login_returning("picker").expect("login");
    let (sx, sz) = world::beside(0, x, z).expect("a walkable tile beside the wheat");
    engine.harness_telejump(client.pid, sx, 0, sz);
    for _ in 0..3 {
        engine.cycle();
    }
    assert!(loc_at(engine, name, x, z), "{name} stands at {x},{z}");
    client.packets();
    let _ = engine.script_errors.take();

    client.op_loc(2, x, z, loc_id(engine, name));
    let mut told = Vec::new();
    for _ in 0..8 {
        engine.cycle();
        told.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }

    assert!(
        told.iter().any(|m| m == "You pick some grain."),
        "{name}: told {told:?}"
    );
    assert!(
        !told.iter().any(|m| m == "Nothing interesting happens."),
        "{name}: told {told:?}"
    );
    assert_eq!(grain_held(engine, client.pid), 1, "{name}: one grain");
    assert!(!loc_at(engine, name, x, z), "{name} was picked");
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "{name}: {errors:?}");
}

#[test]
fn field_wheat_gives_grain() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for (name, x, z) in FIELD_WHEAT {
            picks_grain(engine, name, x, z);
        }
    });
}

#[cfg(since_742)]
#[test]
fn taverley_wheat_gives_grain() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for (name, x, z) in TAVERLEY_WHEAT {
            picks_grain(engine, name, x, z);
        }
    });
}
