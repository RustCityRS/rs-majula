//! 289's mill at 2011's Mill Lane Mill.
//!
//! 2011 rebuilt the Lumbridge mill under new ids: hopper 36881 and controls
//! 2718 on the top floor, flour bin 36880 on the ground floor. Every 2011 flour
//! bin is a multiloc on varp 695, 289's `%flour`, so the client draws it full
//! or empty and 289's loc changes go. `OPLOC1`: `ip2(z) ip2(loc) p1(ctrl)
//! p2_alt1(x)`; `OPLOCT`: `p2_alt1(obj) p2_alt1(slot) p2(z) p4_alt2(com)
//! p2_alt1(x) p1_alt3(ctrl) ip2(loc)`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;
/// 289's `%flour`: 2011's varp 695.
const FLOUR: u16 = 695;
const HOPPER: (i32, u16, u16) = (36881, 3166, 3307);
const CONTROLS: (i32, u16, u16) = (2718, 3166, 3305);
const FLOUR_BIN: (i32, u16, u16) = (36880, 3166, 3306);

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn slot_of(engine: &Engine, pid: u16, obj: u16) -> Option<u16> {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let inv = engine.harness_player(pid).unwrap().invs.get(&inv).unwrap();
    (0..inv.capacity as u16).find(|&s| inv.get(s).is_some_and(|i| i.obj == obj))
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
    let mut p = ClientPacket::new();
    p.ip2(z as i32).ip2(loc).p1(0).p2_alt1(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

#[test]
fn grain_in_the_hopper_comes_out_as_flour_in_the_bin() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("miller").expect("login");
        engine.cycle();
        let (grain, pot, flour_pot) = (
            obj(engine, "grain"),
            obj(engine, "empty_pot"),
            obj(engine, "pot_of_flour"),
        );
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
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

        let mut p = ClientPacket::new();
        p.p2_alt1(grain as i32)
            .p2_alt1(0)
            .p2(HOPPER.2 as i32)
            .p4_alt2(BACKPACK)
            .p2_alt1(HOPPER.1 as i32)
            .p1_alt3(0)
            .ip2(HOPPER.0);
        rs_vm::ops::test::set_recording(true);
        rs_vm::ops::test::forget_calls();
        client.send(ClientProt::OpLocT, &p.finish());
        ticks(engine, &mut client, 6);
        // What the scripts said (the call log).
        let said = rs_vm::ops::test::calls_for("mes");
        assert!(
            said.iter().any(|m| m == "You put the grain in the hopper."),
            "said {said:?}"
        );
        assert_eq!(
            slot_of(engine, client.pid, grain),
            None,
            "the grain went in"
        );

        op_loc1(&mut client, CONTROLS);
        ticks(engine, &mut client, 6);
        // What the scripts said (the call log).
        let said = rs_vm::ops::test::calls_for("mes");
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
        let said = rs_vm::ops::test::calls_for("mes");
        assert!(
            said.iter()
                .any(|m| m == "You fill a pot with the last of the flour in the bin."),
            "said {said:?}"
        );
        assert!(
            slot_of(engine, client.pid, flour_pot).is_some(),
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
        let (bucket, milk) = (obj(engine, "bucket"), obj(engine, "bucket_of_milk"));
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        engine
            .harness_player_mut(client.pid)
            .and_then(|p| p.invs.get_mut(&inv))
            .unwrap()
            .set(0, bucket, 1);
        engine.harness_telejump(client.pid, 3251, 0, 3275);
        ticks(engine, &mut client, 10);
        rs_vm::ops::test::set_recording(true);
        rs_vm::ops::test::forget_calls();
        op_loc1(&mut client, (8689, 3252, 3275));
        ticks(engine, &mut client, 10);
        let said = rs_vm::ops::test::calls_for("mes");
        assert!(
            said.iter().any(|m| m == "You milk the cow."),
            "said {said:?}"
        );
        assert_eq!(
            slot_of(engine, client.pid, bucket),
            None,
            "the bucket is used"
        );
        assert!(
            slot_of(engine, client.pid, milk).is_some(),
            "a bucket of milk"
        );
    });
}

fn set_varp(engine: &mut Engine, pid: u16, name: &str, value: i32) {
    let id = *engine.cache.varps.debugnames.get(name).unwrap();
    engine
        .harness_player_mut(pid)
        .unwrap()
        .vars
        .set(id, rs_pack::cache::VarValue::Int(value));
}

/// Cook's Assistant's 2011 rework: once Millie Miller has set the millstones,
/// Mill Lane's bin gives extra fine flour (void-634 Windmill); 289's mill gave
/// a pot of flour.
#[test]
fn mill_lane_mills_extra_fine_flour_for_the_cook() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("finemiller").expect("login");
        engine.cycle();
        let (grain, pot, fine) = (
            obj(engine, "grain"),
            obj(engine, "empty_pot"),
            obj(engine, "extra_fine_flour"),
        );
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
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
        let mut p = ClientPacket::new();
        p.p2_alt1(grain as i32)
            .p2_alt1(0)
            .p2(HOPPER.2 as i32)
            .p4_alt2(BACKPACK)
            .p2_alt1(HOPPER.1 as i32)
            .p1_alt3(0)
            .ip2(HOPPER.0);
        client.send(ClientProt::OpLocT, &p.finish());
        ticks(engine, &mut client, 6);
        op_loc1(&mut client, CONTROLS);
        ticks(engine, &mut client, 6);
        engine.harness_telejump(client.pid, 3168, 0, 3306);
        ticks(engine, &mut client, 3);
        rs_vm::ops::test::set_recording(true);
        rs_vm::ops::test::forget_calls();
        op_loc1(&mut client, FLOUR_BIN);
        ticks(engine, &mut client, 8);
        let said = rs_vm::ops::test::calls_for("mes");
        assert!(
            said.iter()
                .any(|m| m == "You fill a pot with the extra fine flour from the bin."),
            "said {said:?}"
        );
        assert!(
            slot_of(engine, client.pid, fine).is_some(),
            "extra fine flour"
        );
    });
}

/// The rework's prized dairy cow (47721, by Gillie Groats's field) gives
/// top-quality milk during the quest.
#[test]
fn the_prized_cow_gives_top_quality_milk() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine
            .harness_login_returning("prizemilker")
            .expect("login");
        engine.cycle();
        let (bucket, top) = (obj(engine, "bucket"), obj(engine, "top_quality_milk"));
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        engine
            .harness_player_mut(client.pid)
            .and_then(|p| p.invs.get_mut(&inv))
            .unwrap()
            .set(0, bucket, 1);
        set_varp(engine, client.pid, "cookquest", 1);
        engine.harness_telejump(client.pid, 3263, 0, 3277);
        ticks(engine, &mut client, 10);
        op_loc1(&mut client, (47721, 3264, 3277));
        ticks(engine, &mut client, 10);
        assert!(
            slot_of(engine, client.pid, top).is_some(),
            "top-quality milk"
        );
        assert_eq!(slot_of(engine, client.pid, bucket), None);
    });
}
