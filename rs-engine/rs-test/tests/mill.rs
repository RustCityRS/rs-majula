//! 289's mill at 2011's Mill Lane Mill.
//!
//! 2011 has the Lumbridge mill's hopper and flour bin under new ids (cache
//! 786; the 2011 wiki's Mill Lane Mill page shows it after a graphical
//! rework): hopper 36881 on the top floor, beside the controls, which keep
//! 289's 2718, and flour bin 36880 on the ground floor. Every 2011 flour
//! bin but one is a multiloc on varp 695, 289's `%flour` (22493 at 3786,2825
//! turns on varpbit 3405), so the client draws it full or empty and 289's loc
//! changes go.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
#[cfg(since_647)]
use rs_pack::cache::VarValue;
use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls, set_recording};

/// 289's `%flour`: 2011's varp 695.
const FLOUR: u16 = 695;
#[cfg(before_742)]
const HOPPER: (i32, u16, u16) = (36881, 3166, 3307);
#[cfg(before_742)]
const CONTROLS: (i32, u16, u16) = (2718, 3166, 3305);
/// 742's rebuilt mill: its own hopper, the controls a tile west.
#[cfg(since_742)]
const HOPPER: (i32, u16, u16) = (70034, 3166, 3307);
#[cfg(since_742)]
const CONTROLS: (i32, u16, u16) = (2718, 3165, 3305);
const FLOUR_BIN: (i32, u16, u16) = (36880, 3166, 3306);

fn obj(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
        .id
}

fn slot_of(engine: &Engine, pid: u16, obj: u16) -> Option<u16> {
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let inv = engine.harness_player(pid).unwrap().invs.get(&inv).unwrap();
    (0..inv.capacity as u16).find(|&s| inv.get(s).is_some_and(|i| i.obj == TypeId::new(obj)))
}

fn flour(engine: &Engine, pid: u16) -> i32 {
    engine.harness_player(pid).unwrap().vars.get(FLOUR).as_int()
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

fn op_loc1(client: &mut HarnessClient, (loc, x, z): (i32, u16, u16)) {
    client.op_loc(1, x, z, TypeId::from_int(loc));
}

#[test]
fn grain_in_the_hopper_comes_out_as_flour_in_the_bin() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("miller").expect("login");
        engine.cycle();
        let (grain, pot, flour_pot) = (
            obj(engine, "grain"),
            obj(engine, "pot_empty"),
            obj(engine, "pot_flour"),
        );
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        {
            let backpack = engine
                .harness_player_mut(client.pid)
                .and_then(|p| p.invs.get_mut(&inv))
                .unwrap();
            backpack.set(0, grain, 1);
            backpack.set(1, pot, 1);
        }
        // Top floor, by the hopper and its controls.
        engine.harness_telejump(client.pid, 3165, 2, 3307);
        ticks(engine, &mut client, 10);
        assert_eq!(flour(engine, client.pid), 0);

        set_recording(true);
        forget_calls();
        client.op_loc_t(
            HOPPER.1,
            HOPPER.2,
            TypeId::from_int(HOPPER.0),
            BACKPACK,
            0,
            grain,
        );
        ticks(engine, &mut client, 6);
        // What the scripts said (the call log).
        let said = calls_for("mes");
        assert!(
            said.iter().any(|m| m == "You put the grain in the hopper."),
            "said {said:?}"
        );
        assert_eq!(
            slot_of(engine, client.pid, grain.g2()),
            None,
            "the grain went in"
        );

        op_loc1(&mut client, CONTROLS);
        ticks(engine, &mut client, 6);
        // What the scripts said (the call log).
        let said = calls_for("mes");
        assert!(
            said.iter()
                .any(|m| m == "You operate the hopper. The grain slides down the chute."),
            "said {said:?}"
        );
        assert_eq!(
            flour(engine, client.pid),
            1,
            "the bin holds one pot of flour"
        );

        // Ground floor, by the flour bin.
        engine.harness_telejump(client.pid, 3168, 0, 3306);
        ticks(engine, &mut client, 3);
        op_loc1(&mut client, FLOUR_BIN);
        ticks(engine, &mut client, 8);
        // What the scripts said (the call log).
        let said = calls_for("mes");
        assert!(
            said.iter()
                .any(|m| m == "You fill a pot with the last of the flour in the bin."),
            "said {said:?}"
        );
        assert!(
            slot_of(engine, client.pid, flour_pot.g2()).is_some(),
            "a pot of flour"
        );
        assert_eq!(flour(engine, client.pid), 0, "the bin is empty again");
    });
}

/// 2011 milks dairy cows, which are locs (8689 east of Lumbridge castle): an
/// empty bucket comes back full. "Dairy cows were introduced because players
/// could attack and kill a cow whilst another player was milking it" (2011
/// wiki, Dairy cow), so a bucket on a cow npc is turned away.
#[test]
fn a_dairy_cow_fills_an_empty_bucket() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("milkmaid").expect("login");
        engine.cycle();
        let (bucket, milk) = (obj(engine, "bucket_empty"), obj(engine, "bucket_milk"));
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        engine
            .harness_player_mut(client.pid)
            .and_then(|p| p.invs.get_mut(&inv))
            .unwrap()
            .set(0, bucket, 1);
        engine.harness_telejump(client.pid, 3251, 0, 3275);
        ticks(engine, &mut client, 10);
        set_recording(true);
        forget_calls();
        op_loc1(&mut client, (8689, 3252, 3275));
        ticks(engine, &mut client, 10);
        let said = calls_for("mes");
        assert!(
            said.iter().any(|m| m == "You milk the cow."),
            "said {said:?}"
        );
        assert_eq!(
            slot_of(engine, client.pid, bucket.g2()),
            None,
            "the bucket is used"
        );
        assert!(
            slot_of(engine, client.pid, milk.g2()).is_some(),
            "a bucket of milk"
        );
    });
}

#[cfg(since_647)]
fn set_varp(engine: &mut Engine, pid: u16, name: &str, value: i32) {
    let id = engine.cache.varps.get_by_debugname(name).unwrap().id;
    engine
        .harness_player_mut(pid)
        .unwrap()
        .vars
        .set(id.g2(), VarValue::Int(value));
}

/// Cook's Assistant's rework (15 December 2009, 2011 wiki): once Millie
/// Miller has set the millstones, Mill Lane's bin gives extra fine flour
/// (void-634 Windmill); 289's mill gave a pot of flour.
#[cfg(since_647)]
#[test]
fn mill_lane_mills_extra_fine_flour_for_the_cook() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("finemiller").expect("login");
        engine.cycle();
        let (grain, pot, fine) = (
            obj(engine, "grain"),
            obj(engine, "pot_empty"),
            obj(engine, "cook_flour"),
        );
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        {
            let backpack = engine
                .harness_player_mut(client.pid)
                .and_then(|p| p.invs.get_mut(&inv))
                .unwrap();
            backpack.set(0, grain, 1);
            backpack.set(1, pot, 1);
        }
        set_varp(engine, client.pid, "cookquest", 1);
        set_varp(engine, client.pid, "cookquest_millie", 1);
        engine.harness_telejump(client.pid, 3165, 2, 3307);
        ticks(engine, &mut client, 10);
        client.op_loc_t(
            HOPPER.1,
            HOPPER.2,
            TypeId::from_int(HOPPER.0),
            BACKPACK,
            0,
            grain,
        );
        ticks(engine, &mut client, 6);
        op_loc1(&mut client, CONTROLS);
        ticks(engine, &mut client, 6);
        engine.harness_telejump(client.pid, 3168, 0, 3306);
        ticks(engine, &mut client, 3);
        set_recording(true);
        forget_calls();
        op_loc1(&mut client, FLOUR_BIN);
        ticks(engine, &mut client, 8);
        let said = calls_for("mes");
        assert!(
            said.iter()
                .any(|m| m == "You fill a pot with the extra fine flour from the bin."),
            "said {said:?}"
        );
        assert!(
            slot_of(engine, client.pid, fine.g2()).is_some(),
            "extra fine flour"
        );
    });
}

/// The rework's prized dairy cow (47721, by Gillie Groats's field) gives
/// top-quality milk during the quest.
#[cfg(since_647)]
#[test]
fn the_prized_cow_gives_top_quality_milk() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine
            .harness_login_returning("prizemilker")
            .expect("login");
        engine.cycle();
        let (bucket, top) = (obj(engine, "bucket_empty"), obj(engine, "cook_milk"));
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        engine
            .harness_player_mut(client.pid)
            .and_then(|p| p.invs.get_mut(&inv))
            .unwrap()
            .set(0, bucket, 1);
        set_varp(engine, client.pid, "cookquest", 1);
        // 742 stands the cow a tile west.
        #[cfg(before_742)]
        let (cow_x, from_x) = (3264, 3263);
        #[cfg(since_742)]
        let (cow_x, from_x) = (3263, 3262);
        engine.harness_telejump(client.pid, from_x, 0, 3277);
        ticks(engine, &mut client, 10);
        op_loc1(&mut client, (47721, cow_x, 3277));
        ticks(engine, &mut client, 10);
        assert!(
            slot_of(engine, client.pid, top.g2()).is_some(),
            "top-quality milk"
        );
        assert_eq!(slot_of(engine, client.pid, bucket.g2()), None);
    });
}
