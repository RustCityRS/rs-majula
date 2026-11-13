//! Taverley Dungeon at 647 (`areas/area_taverly/dungeon`).
//!
//! The gate to the Cauldron of Thunder is 289's (`prison_doors.rs2`): from the suits' side each
//! suit of armour comes to life before the gate lets anyone through, and from the cauldron's
//! side it only opens. Cache 786 has the gate as `di2_doubleprisondoorr`/`l` on the tile west of
//! the wall line (2888,9830 and 2888,9831, angle 2), the suits as `di2_suitofarmour` on 289's
//! two tiles, and the same gate locs at the dungeon's two other gates.
//!
//! The locked doors are 289's too (`jail_doors.rs2`): the jailer's key on the Black Knights'
//! cells, the dusty key on the gate to the blue dragons, and no key needed on the way out.
//! Cache 786 has the cells' doors as `di2_singleprisondoor` on the cell's side of the wall
//! line (2931,9689 angle 1 and 2931,9695 angle 3) and the gate as 289's `deepdungeondoor` on
//! 289's tile. Velrak (`velrak_the_explorer.rs2`) hands over the dusty key, and one more lies
//! by the lava eels (2011 wiki, Dusty key, 2011-05-16).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, talk_to};
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
/// Velrak's cell door (maps/l45_151.jm2: `0 51 25`, angle 1: the wall is its north edge).
const CELL_DOOR: (u16, u16) = (2931, 9689);
/// The cell beside it (`0 51 31`, angle 3: the wall is its south edge).
const OTHER_CELL_DOOR: (u16, u16) = (2931, 9695);
/// The gate to the blue dragons (maps/l45_153.jm2: `0 44 11`, angle 0: the wall is its west edge).
const DUSTY_GATE: (u16, u16) = (2924, 9803);
/// Velrak's spawn (maps/n45_151.jm2: `0 51 21`) and the dusty key's (maps/o45_152.jm2: `0 20 38`).
const VELRAK: (u16, u16) = (2931, 9685);
const DUSTY_KEY: (u16, u16) = (2900, 9766);
const BACKPACK: i32 = 149 << 16;

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
    let map = match kind {
        "obj" => &engine.cache.objs.debugnames,
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

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str) {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, o, 1);
}

fn held(engine: &Engine, client: &HarnessClient, name: &str) -> u32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(o) as u32
}

/// Uses the backpack's `slot` (holding `obj`) on the loc `name` at that tile; the game
/// messages of the next `ticks` ticks.
fn use_on(
    engine: &mut Engine,
    client: &mut HarnessClient,
    (obj, slot): (&str, u16),
    name: &str,
    (x, z): (u16, u16),
    ticks: usize,
) -> Vec<String> {
    let obj = id(engine, "obj", obj);
    let loc = id(engine, "loc", name);
    let mut p = ClientPacket::new();
    p.p2_alt2(obj as i32)
        .p2_alt2(slot as i32)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(loc as i32);
    client.send(ClientProt::OpLocT, &p.finish());
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

/// Talks to Velrak from his own tile, taking `choices`.
fn talk_to_velrak(
    engine: &mut Engine,
    client: &mut HarnessClient,
    choices: &[&str],
) -> Conversation {
    let want = id(engine, "npc", "velrak_the_explorer");
    let (nid, x, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == want)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .expect("Velrak is spawned");
    // In the cell: south of its door's wall line.
    assert!(z <= CELL_DOOR.1, "Velrak at {x},{z} is in his cell");
    engine.harness_telejump(client.pid, x, 0, z);
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    talk_to(client, nid);
    let talk = converse(engine, client, choices, 300);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    talk
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

#[test]
fn the_black_knights_cells_need_the_jailers_key_to_enter() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // The corridor between the cells: north of Velrak's door, south of the other.
        let corridor = (CELL_DOOR.0, CELL_DOOR.1 + 1);
        let mut client = visitor(engine, "jail_in", corridor);
        let said = open(engine, &mut client, "di2_singleprisondoor", CELL_DOOR, 3);
        assert_eq!(said, ["This door is locked."]);
        assert_eq!(at(engine, &client), corridor);

        // 289: any other item on the door does nothing.
        give(engine, &client, 0, "dusty_key");
        let said = use_on(
            engine,
            &mut client,
            ("dusty_key", 0),
            "di2_singleprisondoor",
            CELL_DOOR,
            3,
        );
        assert_eq!(said, ["Nothing interesting happens."]);
        assert_eq!(at(engine, &client), corridor);

        give(engine, &client, 1, "jail_key");
        let said = use_on(
            engine,
            &mut client,
            ("jail_key", 1),
            "di2_singleprisondoor",
            CELL_DOOR,
            4,
        );
        assert_eq!(said, ["You unlock the door."]);
        assert_eq!(at(engine, &client), CELL_DOOR, "into the cell");
        assert_eq!(held(engine, &client, "jail_key"), 1, "the key is kept");

        // Out again needs no key.
        for _ in 0..4 {
            engine.cycle();
        }
        client.packets();
        let said = open(engine, &mut client, "di2_singleprisondoor", CELL_DOOR, 4);
        assert_eq!(said, ["The door locks shut behind you."]);
        assert_eq!(at(engine, &client), corridor);

        // The cell beside it is the same door, facing the other way.
        let corridor = (OTHER_CELL_DOOR.0, OTHER_CELL_DOOR.1 - 1);
        engine.harness_telejump(client.pid, corridor.0, 0, corridor.1);
        for _ in 0..6 {
            engine.cycle();
        }
        client.packets();
        let said = open(
            engine,
            &mut client,
            "di2_singleprisondoor",
            OTHER_CELL_DOOR,
            3,
        );
        assert_eq!(said, ["This door is locked."]);
        assert_eq!(at(engine, &client), corridor);
        assert!(
            engine.script_errors.take().is_empty(),
            "no script error at the cells"
        );
    });
}

#[test]
fn velrak_passes_over_the_dusty_key_in_his_cell() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        assert!(!rsmod::is_flagged(VELRAK.0, VELRAK.1, 0, 0x100 | 0x200000));
        let mut client = visitor(engine, "velrak", VELRAK);
        let talk = talk_to_velrak(
            engine,
            &mut client,
            &["So... do you know anywhere good to explore?", "Yes please!"],
        );
        assert!(talk.said("Thank you for rescuing me!"), "{:?}", talk.texts);
        // 2011 wiki (Velrak the explorer, 2011-05-18): he "reaches somewhere mysterious".
        assert!(
            talk.said("Velrak reaches somewhere mysterious and passes you a key."),
            "{:?}",
            talk.texts
        );
        assert_eq!(held(engine, &client, "dusty_key"), 1);

        let talk = talk_to_velrak(engine, &mut client, &[]);
        assert!(talk.said("Are you still here?"), "{:?}", talk.texts);
        assert_eq!(held(engine, &client, "dusty_key"), 1);
        assert!(
            engine.script_errors.take().is_empty(),
            "no script error from Velrak"
        );
    });
}

#[test]
fn the_gate_to_the_blue_dragons_needs_the_dusty_key() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // One dusty key lies by the lava eels.
        let key = id(engine, "obj", "dusty_key");
        assert!(
            engine
                .zones
                .zone(DUSTY_KEY.0, 0, DUSTY_KEY.1)
                .is_some_and(|zone| zone.get_obj(DUSTY_KEY.0, DUSTY_KEY.1, key, None).is_some()),
            "the dusty key's spawn"
        );

        // 289: the key is needed from the gate's own tile, the lesser demons' side.
        let mut client = visitor(engine, "dusty", DUSTY_GATE);
        let said = open(engine, &mut client, "deepdungeondoor", DUSTY_GATE, 3);
        assert_eq!(said, ["This gate is locked."]);
        assert_eq!(at(engine, &client), DUSTY_GATE);

        give(engine, &client, 0, "dusty_key");
        let said = use_on(
            engine,
            &mut client,
            ("dusty_key", 0),
            "deepdungeondoor",
            DUSTY_GATE,
            4,
        );
        assert_eq!(said, ["You unlock the gate."]);
        assert_eq!(at(engine, &client), (DUSTY_GATE.0 - 1, DUSTY_GATE.1));
        assert_eq!(held(engine, &client, "dusty_key"), 1, "the key is kept");

        // Back out needs no key.
        for _ in 0..4 {
            engine.cycle();
        }
        client.packets();
        let said = open(engine, &mut client, "deepdungeondoor", DUSTY_GATE, 4);
        assert_eq!(said, ["The gate locks shut behind you."]);
        assert_eq!(at(engine, &client), DUSTY_GATE);
        assert!(
            engine.script_errors.take().is_empty(),
            "no script error at the gate"
        );
    });
}
