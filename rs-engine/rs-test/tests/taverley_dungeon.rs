//! Taverley Dungeon at 647 (`areas/area_taverly/dungeon`).
//!
//! The gate to the Cauldron of Thunder is 289's (`prison_doors.rs2`): from the suits' side each
//! suit of armour comes to life before the gate lets anyone through, and from the cauldron's
//! side it only opens. Cache 786 has the gate as `di2_doubleprisondoorr`/`l` on the tile west of
//! the wall line (2888,9830 and 2888,9831, angle 2), the suits as `di2_suitofarmour` on 289's
//! two tiles, and the same gate locs at the dungeon's two other gates.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

/// The gate's two leaves (maps/l45_153.jm2: `0 8 38` and `0 8 39`).
const GATE_R: (u16, u16) = (2888, 9830);
const GATE_L: (u16, u16) = (2888, 9831);
/// The suits of armour (`0 7 37` and `0 7 40`).
const SUIT_SOUTH: (u16, u16) = (2887, 9829);
const SUIT_NORTH: (u16, u16) = (2887, 9832);
/// Another gate of the same locs (`0 13 33` and `0 12 33`, angle 1).
const OTHER_GATE_R: (u16, u16) = (2893, 9825);
const COMES_TO_LIFE: &str = "Suddenly the suit of armour comes to life!";

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
    let map = match kind {
        "npc" => &engine.cache.npcs.debugnames,
        "loc" => &engine.cache.locs.debugnames,
        _ => unreachable!(),
    };
    *map.get(name).unwrap_or_else(|| panic!("no {kind} {name}"))
}

fn visitor(engine: &mut Engine, name: &str, (x, z): (u16, u16)) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    assert!(
        !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000),
        "{x},{z} is walkable"
    );
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, x, 0, z);
    for _ in 0..4 {
        engine.cycle();
    }
    client.packets();
    client
}

/// Open on the loc `name` at that tile; the game messages of the next `ticks` ticks.
fn open(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    (x, z): (u16, u16),
    ticks: usize,
) -> Vec<String> {
    let loc = id(engine, "loc", name);
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
    let mut said = Vec::new();
    for _ in 0..ticks {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

fn stands(engine: &Engine, name: &str, (x, z): (u16, u16)) -> bool {
    let loc = id(engine, "loc", name);
    engine
        .zones
        .zone(x, 0, z)
        .is_some_and(|zone| zone.get_loc(x, z, loc).is_some())
}

fn suits_alive(engine: &Engine) -> usize {
    let suit = id(engine, "npc", "suit_of_armour");
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == suit)
        .count()
}

fn at(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

#[test]
fn the_suits_of_armour_come_to_life_before_the_gate_opens() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = visitor(engine, "gate_in", GATE_R);
        assert!(stands(engine, "di2_suitofarmour", SUIT_NORTH));
        assert!(stands(engine, "di2_suitofarmour", SUIT_SOUTH));
        assert_eq!(suits_alive(engine), 0);

        // 289: the northern suit first, and the gate stays shut.
        let said = open(engine, &mut client, "di2_doubleprisondoorr", GATE_R, 3);
        assert_eq!(said, [COMES_TO_LIFE]);
        assert!(!stands(engine, "di2_suitofarmour", SUIT_NORTH));
        assert!(stands(engine, "di2_suitofarmour", SUIT_SOUTH));
        assert_eq!(suits_alive(engine), 1);
        assert_eq!(at(engine, &client).0, GATE_R.0);

        engine.harness_telejump(client.pid, GATE_L.0, 0, GATE_L.1);
        let said = open(engine, &mut client, "di2_doubleprisondoorl", GATE_L, 3);
        assert!(said.iter().any(|m| m == COMES_TO_LIFE), "{said:?}");
        assert!(!stands(engine, "di2_suitofarmour", SUIT_SOUTH));
        assert_eq!(suits_alive(engine), 2);
        assert_eq!(at(engine, &client).0, GATE_L.0);

        // Both suits are up: the gate lets the player through to the cauldron's side.
        engine.harness_telejump(client.pid, GATE_R.0, 0, GATE_R.1);
        let said = open(engine, &mut client, "di2_doubleprisondoorr", GATE_R, 4);
        assert!(!said.iter().any(|m| m == COMES_TO_LIFE), "{said:?}");
        assert_eq!(at(engine, &client), (GATE_R.0 + 1, GATE_R.1), "{said:?}");
        assert!(
            engine.script_errors.take().is_empty(),
            "no script error at the gate"
        );
    });
}

#[test]
fn the_gate_opens_from_the_cauldrons_side_without_waking_the_suits() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = visitor(engine, "gate_out", (GATE_L.0 + 1, GATE_L.1));
        // The suits as the case before left them on the shared engine: a suit woken there
        // stays up for 500 ticks.
        let before = (
            stands(engine, "di2_suitofarmour", SUIT_NORTH),
            stands(engine, "di2_suitofarmour", SUIT_SOUTH),
            suits_alive(engine),
        );
        let said = open(engine, &mut client, "di2_doubleprisondoorl", GATE_L, 4);
        assert!(said.is_empty(), "{said:?}");
        assert_eq!(at(engine, &client), GATE_L);
        let after = (
            stands(engine, "di2_suitofarmour", SUIT_NORTH),
            stands(engine, "di2_suitofarmour", SUIT_SOUTH),
            suits_alive(engine),
        );
        assert_eq!(after, before, "no suit woke");
    });
}

#[test]
fn the_dungeons_other_gate_of_the_same_locs_is_walked_through() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // Angle 1: the wall line is the tile's north edge.
        let mut client = visitor(engine, "gate_other", OTHER_GATE_R);
        let said = open(
            engine,
            &mut client,
            "di2_doubleprisondoorr",
            OTHER_GATE_R,
            4,
        );
        assert!(said.is_empty(), "{said:?}");
        assert_eq!(
            at(engine, &client),
            (OTHER_GATE_R.0, OTHER_GATE_R.1 + 1),
            "through the gate"
        );
        assert_eq!(suits_alive(engine), 0);
        // The gate has shut behind the player: both leaves are back.
        for _ in 0..6 {
            engine.cycle();
        }
        assert!(stands(engine, "di2_doubleprisondoorr", OTHER_GATE_R));
        assert!(stands(
            engine,
            "di2_doubleprisondoorl",
            (OTHER_GATE_R.0 - 1, OTHER_GATE_R.1)
        ));
    });
}
