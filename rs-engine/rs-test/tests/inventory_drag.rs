//! Dragging one backpack slot onto another moves the obj, as the client
//! reports the drag.
//!
//! IF_BUTTOND (`SceneGraphNode_Sub6_Sub1.method3715`) carries both ends. The
//! drop target is a component of interface 149's second bank of 28 backing
//! sprites, which login unlocks for drag, so the target slot arrives as
//! 28 + slot and `[inv_buttond,inventory:inv]` takes the offset back off.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

/// `inventory:inv`.
const BACKPACK: i32 = 149 << 16;

#[test]
fn a_drag_moves_and_swaps_backpack_slots() {
    runner::on_loaded_engine(|engine| {
        let logs = *engine.cache.objs.debugnames.get("logs").unwrap();
        let sword = *engine.cache.objs.debugnames.get("bronze_sword").unwrap();

        // Into an empty slot: the obj moves and its slot empties.
        let slots = drag(engine, &[(0, logs)], 0, 5);
        assert_eq!(
            (slots[0], slots[5]),
            (None, Some(logs)),
            "logs moved to slot 5"
        );

        // Onto an occupied slot: the two change places.
        let slots = drag(engine, &[(0, logs), (3, sword)], 0, 3);
        assert_eq!(
            (slots[0], slots[3]),
            (Some(sword), Some(logs)),
            "logs and sword swapped"
        );
    });
}

/// A returning player holding `held`, after one drag from `from` onto the
/// backing sprite of `to`. Returns the backpack, slot by slot.
fn drag(engine: &mut Engine, held: &[(u16, u16)], from: u16, to: u16) -> Vec<Option<u16>> {
    engine.harness_reset();
    let mut client = engine.harness_login_returning("dragger").expect("login");
    engine.cycle();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let backpack = engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap();
    for &(slot, obj) in held {
        backpack.set(slot, obj, 1);
    }
    let from_obj = held.iter().find(|(s, _)| *s == from).unwrap().1;

    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .ip2_alt1(from as i32)
        .ip2_alt1(28 + to as i32)
        .p4_alt2(BACKPACK)
        .ip2(from_obj as i32)
        .p2(-1);
    client.send(ClientProt::IfButtonD, &p.finish());
    engine.cycle();

    let backpack = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap();
    (0..28)
        .map(|slot| backpack.get(slot).map(|o| o.obj))
        .collect()
}
