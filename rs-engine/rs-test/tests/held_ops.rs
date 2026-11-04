//! Rev-647 `IfButtonT` -- something used on an item in an inventory -- reaches
//! `[opheldu]` or `[opheldt]`.
//!
//! 647 sends item-on-item and spell-on-item as one packet, which the engine
//! did not dispatch at all: it fell through to the unhandled catch-all, so no
//! item could be used on another. Firemaking's own `[opheldu,tinderbox]` is
//! the witness here -- tinderbox and logs, in either order, should drop and
//! light the logs.

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

/// Lumbridge, east of the castle: open ground a fire may be lit on.
const FIRE_TILE: (u16, u16) = (3221, 3219);

/// The `object_id` the client sends for a source holding no item.
const NO_OBJ: i32 = 0xFFFF;

fn id(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

/// A fresh world with one player at the fire tile, holding `items` from
/// backpack slot 0 on.
fn player_holding(engine: &mut Engine, name: &str, items: &[u16]) -> HarnessClient {
    engine.harness_reset();
    let client = engine.harness_login_returning(name).expect("login");
    engine.harness_telejump(client.pid, FIRE_TILE.0, 0, FIRE_TILE.1);
    engine.cycle();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let backpack = engine
        .harness_player_mut(client.pid)
        .and_then(|p| p.invs.get_mut(&inv))
        .expect("backpack");
    for (slot, &obj) in items.iter().enumerate() {
        backpack.set(slot as u16, obj, 1);
    }
    rs_vm::ops::test::forget_calls();
    client
}

/// `IfButtonT`, as `Class141.method876`'s target branch writes it:
/// `p4(to_com) ip2(to_slot) ip2(from_slot) p2_alt1(from_obj) ip4(from_com)
/// p2(to_obj)`.
fn if_buttont(from: (i32, i32), to: (i32, i32)) -> Vec<u8> {
    let (from_slot, from_obj) = from;
    let (to_slot, to_obj) = to;
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .ip2(to_slot)
        .ip2(from_slot)
        .p2_alt1(from_obj)
        .ip4(BACKPACK)
        .p2(to_obj);
    p.finish()
}

fn lit_logs(engine: &Engine, pid: u16) -> bool {
    let tried = rs_vm::ops::test::calls_for("mes")
        .iter()
        .any(|m| m == "You attempt to light the logs.");
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let logs_gone = engine
        .harness_player(pid)
        .and_then(|p| p.invs.get(&inv))
        .is_some_and(|backpack| !backpack.has_at(1, id(engine, "logs")));
    tried && logs_gone
}

#[test]
fn something_used_on_an_item_reaches_its_held_trigger() {
    runner::on_loaded_engine(|engine| {
        let tinderbox = id(engine, "tinderbox") as i32;
        let logs = id(engine, "logs") as i32;

        // Tinderbox (slot 0) used on the logs (slot 1). No `[opheldu,logs]`
        // exists at 647, so the search falls to the item used.
        let mut client = player_holding(
            engine,
            "held_tinderbox_on_logs",
            &[tinderbox as u16, logs as u16],
        );
        client.send(
            ClientProt::IfButtonT,
            &if_buttont((0, tinderbox), (1, logs)),
        );
        engine.harness_inputs();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.last_item, tinderbox, "keyed on the tinderbox");
        assert_eq!(
            player.last_use_item,
            Some(logs as u16),
            "the logs are the other item"
        );
        assert_eq!(player.last_use_slot, Some(1));
        assert!(
            lit_logs(engine, client.pid),
            "[opheldu,tinderbox] dropped and lit the logs; mes: {:?}",
            rs_vm::ops::test::calls_for("mes")
        );

        // The logs used on the tinderbox: the target is searched first, so the
        // same script answers with nothing to swap.
        let mut client = player_holding(
            engine,
            "held_logs_on_tinderbox",
            &[tinderbox as u16, logs as u16],
        );
        client.send(
            ClientProt::IfButtonT,
            &if_buttont((1, logs), (0, tinderbox)),
        );
        engine.harness_inputs();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.last_item, tinderbox);
        assert_eq!(player.last_use_item, Some(logs as u16));
        assert!(lit_logs(engine, client.pid), "either order lights the logs");

        // A spell cast on an item drives `[opheldt]`, with the item as
        // `last_item`. No 647 content answers one yet.
        let mut client = player_holding(
            engine,
            "held_spell_on_item",
            &[tinderbox as u16, logs as u16],
        );
        let high_alchemy = (192 << 16) | 59;
        let mut p = ClientPacket::new();
        p.p4(BACKPACK)
            .ip2(1)
            .ip2(NO_OBJ)
            .p2_alt1(NO_OBJ)
            .ip4(high_alchemy)
            .p2(logs);
        client.send(ClientProt::IfButtonT, &p.finish());
        engine.harness_inputs();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.last_item, logs, "the target is last_item");
        assert_eq!(player.last_slot, 1);
        assert_eq!(player.last_use_item, None, "a spell is not a used item");

        // A target the slot does not hold is dropped: nothing runs.
        let mut client = player_holding(engine, "held_forged", &[tinderbox as u16, logs as u16]);
        client.send(
            ClientProt::IfButtonT,
            &if_buttont((0, tinderbox), (1, tinderbox)),
        );
        engine.harness_inputs();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.last_use_item, None, "nothing recorded as used");
        assert!(rs_vm::ops::test::calls_for("mes").is_empty(), "nothing ran");
    });
}
