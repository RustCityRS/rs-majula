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
use rs_grid::CoordGrid;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::info_prot::PlayerInfoProt;
use rs_test::runner;
use rs_vm::ops::test::{calls_for, forget_calls};

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
    forget_calls();
    client
}

/// `IfButtonT`, as `Class141.method876`'s target branch writes it:
/// `p4(to_com) p2_alt1(to_slot) p2_alt1(from_slot) p2_alt2(from_obj) p4_alt1(from_com)
/// p2(to_obj)`.
fn if_buttont(from: (i32, i32), to: (i32, i32)) -> Vec<u8> {
    let (from_slot, from_obj) = from;
    let (to_slot, to_obj) = to;
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .p2_alt1(to_slot)
        .p2_alt1(from_slot)
        .p2_alt2(from_obj)
        .p4_alt1(BACKPACK)
        .p2(to_obj);
    p.finish()
}

fn lit_logs(engine: &Engine, pid: u16) -> bool {
    let tried = calls_for("mes")
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
        assert_eq!(
            player.last_item,
            Some(tinderbox as u16),
            "keyed on the tinderbox"
        );
        assert_eq!(
            player.last_use_item,
            Some(logs as u16),
            "the logs are the other item"
        );
        assert_eq!(player.last_use_slot, Some(1));
        assert!(
            lit_logs(engine, client.pid),
            "[opheldu,tinderbox] dropped and lit the logs; mes: {:?}",
            calls_for("mes")
        );
        let player = &mut engine.player_list.players[client.pid as usize]
            .as_mut()
            .unwrap()
            .player;
        player.reorient(None);
        assert_eq!(player.info.face_x, Some(CoordGrid::fine(FIRE_TILE.0, 1)));
        assert_eq!(player.info.face_z, Some(CoordGrid::fine(FIRE_TILE.1, 1)));
        engine.player_renderer.compute_info(
            client.pid as usize,
            &player.info,
            player.pathing.coord,
        );
        assert_eq!(
            engine.player_renderer.high_block(client.pid)[0] & PlayerInfoProt::FaceCoord as u8,
            0,
            "lighting logs underfoot must not force a turn south"
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
        assert_eq!(player.last_item, Some(tinderbox as u16));
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
            .p2_alt1(1)
            .p2_alt1(NO_OBJ)
            .p2_alt2(NO_OBJ)
            .p4_alt1(high_alchemy)
            .p2(logs);
        client.send(ClientProt::IfButtonT, &p.finish());
        engine.harness_inputs();
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.last_item,
            Some(logs as u16),
            "the target is last_item"
        );
        assert_eq!(player.last_slot, Some(1));
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
        assert!(calls_for("mes").is_empty(), "nothing ran");
    });
}

/// 289's `[opheld2,nightshade]` (player/scripts/consumption/nightshade.rs2) on
/// 2011's cave nightshade, whose Eat is op4 (cache 786): it is eaten, says
/// 289's lines, and hurts 150 life points, 289's 15 hitpoints (2011 wiki, Cave
/// nightshade, 2011-04-06: "this causes 150 life points of damage").
#[test]
fn eating_cave_nightshade_hurts_150_life_points() {
    runner::on_loaded_engine(|engine| {
        let nightshade = id(engine, "nightshade");
        let mut client = player_holding(engine, "nightshade", &[nightshade]);
        // %life_points: varbit 7198, bits 1-15 of varp 1240.
        let life = |e: &Engine, pid: u16| {
            (e.harness_player(pid).unwrap().vars.get(1240).as_int() >> 1) & 0x7FFF
        };
        // Level 99 Constitution and 990 life points, so 150 does not kill.
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.stats.levels[3] = 99;
        player.stats.base_levels[3] = 99;
        let bit0 = player.vars.get(1240).as_int() & 1;
        player.vars.set(1240, VarValue::Int((990 << 1) | bit0));
        engine.cycle();
        let before = life(engine, client.pid);
        assert_eq!(before, 990);
        // The client puts an obj's iop4 on the slot's op 7.
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(nightshade as i32).p2_alt1(0);
        client.send(ClientProt::IfButton7, &p.finish());
        for _ in 0..5 {
            engine.cycle();
        }
        let said = calls_for("mes");
        assert!(
            said.iter()
                .any(|m| m == "The nightshade was highly poisonous."),
            "{said:?}"
        );
        // The queued damage runs once the player's script lets go: after the
        // "Ahhhh! What have I done!" box, as a player clicks past it.
        let chat = engine
            .harness_player(client.pid)
            .unwrap()
            .modal_chat
            .expect("the player says Ahhhh");
        let mut p = ClientPacket::new();
        let com = engine
            .cache
            .interfaces
            .types
            .iter()
            .find(|(_, com)| com.root_layer == chat as i32 && com.active_properties & 1 != 0)
            .map(|(&id, _)| id)
            .expect("continue button");
        p.p2_alt1(0xFFFF).p4_alt2(com);
        client.send(ClientProt::ResumePauseButton, &p.finish());
        for _ in 0..3 {
            engine.cycle();
        }
        assert_eq!(before - life(engine, client.pid), 150, "150 life points");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let held = engine
            .harness_player(client.pid)
            .and_then(|p| p.invs.get(&inv))
            .is_some_and(|b| b.has_at(0, nightshade));
        assert!(!held, "the nightshade is eaten");
    });
}

/// 289's water_sources.rs2: a bucket of water used on a jug pours into it
/// (`@water_fill`): a jug of water and an empty bucket.
#[test]
fn a_bucket_of_water_pours_into_a_jug() {
    runner::on_loaded_engine(|engine| {
        let (water, jug) = (id(engine, "bucket_water"), id(engine, "jug_empty"));
        let mut client = player_holding(engine, "pourer", &[water, jug]);
        client.send(
            ClientProt::IfButtonT,
            &if_buttont((0, water as i32), (1, jug as i32)),
        );
        for _ in 0..3 {
            engine.cycle();
        }
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let backpack = engine
            .harness_player(client.pid)
            .and_then(|p| p.invs.get(&inv))
            .expect("backpack");
        assert!(
            backpack.has_at(0, id(engine, "bucket_empty")),
            "the bucket is empty"
        );
        assert!(
            backpack.has_at(1, id(engine, "jug_water")),
            "the jug is full"
        );
    });
}

/// The client puts an obj's iop5 (Drop, Destroy) on the slot's op 8
/// (inventory_setophelds_specified). 289 answers it with `[opheld5,<obj>]`,
/// and drops by `[opheld5,_]` (player/scripts/drop.rs2): a troll thistle
/// withers away (quest_eadgar), anything else is put down.
#[test]
fn drop_runs_the_objs_own_op5_or_puts_it_down() {
    runner::on_loaded_engine(|engine| {
        let (thistle, logs) = (id(engine, "eadgar_troll_thistle"), id(engine, "logs"));
        let mut client = player_holding(engine, "thistle", &[thistle, logs]);
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        // The drop's sound, by its id in 647's synth registry.
        let pack = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/647/pack/synth.pack"
        );
        let put_down = std::fs::read_to_string(pack)
            .expect(pack)
            .lines()
            .find_map(|l| l.strip_suffix("=put_down").map(str::to_owned))
            .expect("synth.pack names put_down");
        let put_downs = || {
            calls_for("sound_synth")
                .iter()
                .filter(|a| a.split(',').next() == Some(put_down.as_str()))
                .count()
        };
        let empty = |e: &Engine, pid: u16, slot: u16, obj: u16| {
            e.harness_player(pid)
                .and_then(|p| p.invs.get(&inv))
                .is_some_and(|b| !b.has_at(slot, obj))
        };

        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(thistle as i32).p2_alt1(0);
        client.send(ClientProt::IfButton8, &p.finish());
        for _ in 0..2 {
            engine.cycle();
        }
        let said = calls_for("mes");
        assert!(
            said.iter()
                .any(|m| m == "The Troll Thistle withers away as soon as it hits the ground."),
            "{said:?}"
        );
        assert!(empty(engine, client.pid, 0, thistle), "the thistle is gone");
        assert_eq!(put_downs(), 0, "the thistle is not put down");

        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(logs as i32).p2_alt1(1);
        client.send(ClientProt::IfButton8, &p.finish());
        for _ in 0..2 {
            engine.cycle();
        }
        assert!(empty(engine, client.pid, 1, logs), "the logs are dropped");
        assert_eq!(put_downs(), 1, "the logs are put down");
    });
}
