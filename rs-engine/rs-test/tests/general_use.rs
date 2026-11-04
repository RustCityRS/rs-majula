//! 289's general-use scripts (`general_use/scripts`) on 2011's locs and objs:
//! the hat stand (loc 374, 28 places in the 786 map) and the hammer on a
//! gem. `OPLOCT` and `IFBUTTONT` are written as the 647 client's send sites
//! write them (see `targeted_ops.rs`, `held_ops.rs`).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

/// `inventory:inv`.
const BACKPACK: i32 = 149 << 16;
/// Seers' Village's hat stand (`l42_54`: 0 22 26), and the tile south of it.
const HAT_STAND: (u16, u16) = (2710, 3482);
const STAND_TILE: (u16, u16) = (2710, 3481);

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

/// A fresh world with one player at `x, z` holding `objs` from slot 0.
fn holding(engine: &mut Engine, name: &str, x: u16, z: u16, objs: &[u16]) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.harness_telejump(client.pid, x, 0, z);
    engine.cycle();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let backpack = engine
        .harness_player_mut(client.pid)
        .and_then(|p| p.invs.get_mut(&inv))
        .expect("backpack");
    for (slot, &o) in objs.iter().enumerate() {
        backpack.set(slot as u16, o, 1);
    }
    engine.cycle();
    client.packets();
    rs_vm::ops::test::forget_calls();
    client
}

/// The item in backpack slot 0 used on the hat stand.
fn use_on_hat_stand(client: &mut HarnessClient, item: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt1(item as i32)
        .p2_alt1(0)
        .p2(HAT_STAND.1 as i32)
        .p4_alt2(BACKPACK)
        .p2_alt1(HAT_STAND.0 as i32)
        .p1_alt3(0)
        .ip2(374);
    client.send(ClientProt::OpLocT, &p.finish());
}

fn said(line: &str) -> bool {
    rs_vm::ops::test::calls_for("mes").iter().any(|m| m == line)
}

/// A hat on the hat stand falls off: 289's joke, the hat dropped to the floor.
#[test]
fn a_hat_falls_off_the_hat_stand_and_nothing_else_goes_on_it() {
    runner::on_loaded_engine(|engine| {
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let hat = obj(engine, "wizard_hat");
        let mut client = holding(engine, "hatter", STAND_TILE.0, STAND_TILE.1, &[hat]);
        use_on_hat_stand(&mut client, hat);
        for _ in 0..10 {
            engine.cycle();
        }
        let backpack = engine
            .harness_player(client.pid)
            .and_then(|p| p.invs.get(&inv))
            .unwrap();
        assert!(!backpack.has_at(0, hat), "the hat left the backpack");

        let logs = obj(engine, "logs");
        let mut client = holding(engine, "hatter2", STAND_TILE.0, STAND_TILE.1, &[logs]);
        use_on_hat_stand(&mut client, logs);
        for _ in 0..10 {
            engine.cycle();
        }
        assert!(said("You can't put that on a hat stand."), "289's refusal");
    });
}

/// A hammer used on a red topaz, opal or jade (cut or not): 289's line.
#[test]
fn a_hammer_on_a_semi_precious_gem() {
    runner::on_loaded_engine(|engine| {
        let (hammer, topaz) = (obj(engine, "hammer"), obj(engine, "red_topaz"));
        let mut client = holding(engine, "smasher", 3222, 3219, &[hammer, topaz]);
        let mut p = ClientPacket::new();
        // The topaz (slot 1) used on the hammer (slot 0): 289's [opheldu,hammer]
        // switches on the item used.
        p.p4(BACKPACK)
            .ip2(0)
            .ip2(1)
            .p2_alt1(topaz as i32)
            .ip4(BACKPACK)
            .p2(hammer as i32);
        client.send(ClientProt::IfButtonT, &p.finish());
        for _ in 0..3 {
            engine.cycle();
        }
        assert!(said("You can't think of any reason to do this."));
    });
}
