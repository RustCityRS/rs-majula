//! 289's Treasure Trails on 2011's clue interfaces.
//!
//! 289 read a clue in one of two boxes, trail_clue (four lines) or
//! trail_cluelong (eight); 2011 has only trail_cluelong (345), whose lines are
//! com1-8. 289's numbered map interfaces are 2011's trail_map07-17 (352-362).
//! 2011's IF_SETPOSITION sets a component's position outright where 289's
//! moved it from its layer's, so the sextant (365) adds 289's offsets to its
//! components' own positions. The puzzle (363) and reward (364) boxes show
//! the inventories their client interfaces transmit: 140 and 207, and 141.
//! A backpack op is IF_BUTTON1 on `inventory:inv`: `p4(com) p2_alt3(obj)
//! p2_alt1(slot)`.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::names::BACKPACK;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls, set_recording};

const CLUE_BOX: u16 = 345;
const PUZZLE: u16 = 363;
const REWARD: u16 = 364;
const SEXTANT: u16 = 365;
/// 2011's sextant parts: the horizon (com2, cache position 370,33), the sun
/// (com5, 409,131) and the Tilt Down button (com3).
fn horizon() -> i32 {
    com("trail_sextant:sextant_horizon")
}

fn sun() -> i32 {
    com("trail_sextant:sextant_sun")
}

fn tilt_down() -> i32 {
    com("trail_sextant:sextant_tilt_up")
}

fn obj(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
        .id
}

/// A player in Lumbridge holding `items`, one to a slot from slot 0.
fn holder(engine: &mut Engine, name: &str, items: &[&str]) -> HarnessClient {
    engine.harness_reset();
    let client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let objs: Vec<TypeId> = items.iter().map(|i| obj(engine, i)).collect();
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let backpack = engine
        .harness_player_mut(client.pid)
        .and_then(|p| p.invs.get_mut(&inv))
        .unwrap();
    for (slot, o) in objs.into_iter().enumerate() {
        backpack.set(slot as u16, o, 1);
    }
    client
}

fn op1(client: &mut HarnessClient, com: i32, obj: u16, slot: u16) {
    client.if_button(1, com, slot, TypeId::from_int(obj as i32));
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<(u16, Vec<u8>)> {
    let mut packets = Vec::new();
    for _ in 0..n {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

/// Each IF_SETTEXT on `iface`: its component and text.
fn texts(packets: &[(u16, Vec<u8>)], iface: u16) -> Vec<(u16, String)> {
    server::settexts(packets)
        .into_iter()
        .filter(|(com, _)| (com >> 16) as u16 == iface)
        .map(|(com, text)| (com as u16, text))
        .collect()
}

/// Each IF_SETPOSITION: `p4_alt3(com) p2_alt3(x) p2_alt3(y)` (the client
/// takes the second as x and the third as y: VarpBitType, state change 11).
fn positions(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, i16, i16)> {
    server::positions(packets)
}

/// The inventory ids UPDATE_INV_FULL and UPDATE_INV_PARTIAL sent.
fn invs_sent(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    let full = server::invs(packets).into_iter().map(|i| i.inv as u16);
    let partial = server::inv_changes(packets)
        .into_iter()
        .map(|(inv, _, _)| inv as u16);
    full.chain(partial).collect()
}

#[test]
fn a_clue_reads_in_2011s_one_clue_box() {
    runner::on_loaded_engine(|engine| {
        let clue = obj(engine, "trail_clue_easy_vague003");
        let mut client = holder(engine, "cluereader", &["trail_clue_easy_vague003"]);
        op1(&mut client, BACKPACK, clue.g2(), 0);
        let packets = ticks(engine, &mut client, 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(CLUE_BOX),
            "2011's clue box"
        );
        // Two lines sit in the middle of the eight, as in 289's four-line box.
        let lines = texts(&packets, CLUE_BOX);
        let line = |com: u16| {
            lines
                .iter()
                .rev()
                .find(|(c, _)| *c == com)
                .map(|(_, t)| t.as_str())
        };
        assert_eq!(
            line(4),
            Some("Search the drawers found upstairs"),
            "{lines:?}"
        );
        assert_eq!(line(5), Some("in East Ardougne's houses."), "{lines:?}");
        for com in [1, 2, 3, 6, 7, 8] {
            assert_eq!(line(com), Some(""), "line {com}: {lines:?}");
        }
    });
}

#[test]
fn a_map_clue_opens_2011s_map() {
    runner::on_loaded_engine(|engine| {
        let (text, map) = (
            obj(engine, "trail_clue_easy_vague003"),
            obj(engine, "trail_clue_easy_map004"),
        );
        let mut client = holder(
            engine,
            "mapreader",
            &["trail_clue_easy_vague003", "trail_clue_easy_map004"],
        );
        op1(&mut client, BACKPACK, text.g2(), 0);
        ticks(engine, &mut client, 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(CLUE_BOX),
            "the clue box"
        );
        // Reading the map with the clue box open closes the box, as 289's
        // OPHELD did, and opens the map: 289's inter_176, 2011's trail_map09.
        op1(&mut client, BACKPACK, map.g2(), 1);
        ticks(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(player.is_interface_open(354), "trail_map09");
        assert!(!player.is_interface_open(CLUE_BOX), "the clue box closed");
    });
}

#[test]
fn the_sextant_moves_on_2011s_coordinates() {
    runner::on_loaded_engine(|engine| {
        let sextant = obj(engine, "trail_sextant");
        let mut client = holder(engine, "navigator", &["trail_sextant"]);
        op1(&mut client, BACKPACK, sextant.g2(), 0);
        let packets = ticks(engine, &mut client, 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(SEXTANT),
            "the sextant"
        );
        let moved = positions(&packets);
        let at = |moved: &[(i32, i16, i16)], com: i32| {
            moved
                .iter()
                .rev()
                .find(|(c, _, _)| *c == com)
                .map(|&(_, x, y)| (x, y))
        };
        // 289 starts the horizon -56..42 and the sun -60..58 from their places.
        let (hx, hy) = at(&moved, horizon()).expect("the horizon placed");
        assert_eq!(hx, 370, "{moved:?}");
        assert!((33 - 56..=33 + 42).contains(&hy), "{moved:?}");
        let (sx, sy) = at(&moved, sun()).expect("the sun placed");
        assert_eq!(sx, 409, "{moved:?}");
        assert!((131 - 60..=131 + 58).contains(&sy), "{moved:?}");

        // Tilt Down: the horizon 7 lower, the sun 2 higher. The sextant opens on a random frame,
        // and on the last one (horizon at 33 + 42) only the other button moves it, back.
        let (button, step) = if hy < 33 + 42 {
            (tilt_down(), 1)
        } else {
            (com("trail_sextant:sextant_tilt_down"), -1)
        };
        client.if_button(1, button, 0xFFFF, TypeId::NULL);
        let moved = positions(&ticks(engine, &mut client, 2));
        assert_eq!(
            at(&moved, horizon()),
            Some((370, hy + 7 * step)),
            "{moved:?}"
        );
        assert_eq!(at(&moved, sun()), Some((409, sy - 2 * step)), "{moved:?}");
    });
}

#[test]
fn a_puzzle_box_shows_its_pieces() {
    runner::on_loaded_engine(|engine| {
        let puzzle = obj(engine, "trail_clue_hard_riddle014_puzzlebox");
        let mut client = holder(engine, "puzzler", &["trail_clue_hard_riddle014_puzzlebox"]);
        op1(&mut client, BACKPACK, puzzle.g2(), 0);
        let packets = ticks(engine, &mut client, 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(PUZZLE),
            "the puzzle"
        );
        let sent = invs_sent(&packets);
        assert!(
            sent.contains(&140) && sent.contains(&207),
            "the pieces and the hint; sent {sent:?}"
        );
        // The client sends a piece's Move only for the slots IF_SETEVENTS
        // enabled.
        let events = server::events(&packets);
        assert!(
            events
                .iter()
                .any(|&(c, from, to, ops)| c == com("trail_puzzle:puzzle_layer")
                    && from == 0
                    && to == 24
                    && ops != 0),
            "the pieces' Move enabled; sent {events:?}"
        );

        // Move a piece beside the gap: it slides into it.
        let pieces = engine
            .cache
            .invs
            .get_by_debugname("trail_puzzleinv")
            .unwrap()
            .id;
        let pid = client.pid;
        let grid = |engine: &Engine| {
            let inv = engine
                .harness_player(pid)
                .unwrap()
                .invs
                .get(&pieces)
                .unwrap();
            (0..25u16)
                .map(|s| inv.get(s).map(|i| i.obj))
                .collect::<Vec<_>>()
        };
        let before = grid(engine);
        let gap = before.iter().position(Option::is_none).expect("a gap") as u16;
        let beside = if gap.is_multiple_of(5) {
            gap + 1
        } else {
            gap - 1
        };
        let piece = before[beside as usize].expect("a piece beside the gap");
        client.if_button(1, com("trail_puzzle:puzzle_layer"), beside, piece);
        let moved = ticks(engine, &mut client, 2);
        assert!(
            invs_sent(&moved).contains(&140),
            "the pieces sent again; sent {:?}",
            invs_sent(&moved)
        );
        let after = grid(engine);
        assert_eq!(after[gap as usize], Some(piece), "the piece filled the gap");
        assert_eq!(after[beside as usize], None, "its slot is the gap now");
    });
}

#[test]
fn the_last_casket_of_an_easy_trail_opens_2011s_reward_box() {
    runner::on_loaded_engine(|engine| {
        let casket = obj(engine, "trail_clue_easy_map001_casket");
        let mut client = holder(engine, "treasure", &["trail_clue_easy_map001_casket"]);
        // 289's %trail_status: the trail's steps so far in bits 0-3, at an
        // easy trail's last (^trail_easy_maxsteps, 4).
        let status = engine
            .cache
            .varps
            .get_by_debugname("trail_status")
            .unwrap()
            .id;
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(status.g2(), VarValue::Int(4));
        set_recording(true);
        forget_calls();
        op1(&mut client, BACKPACK, casket.g2(), 0);
        let packets = ticks(engine, &mut client, 3);
        let said = calls_for("mes");
        assert!(
            said.iter()
                .any(|m| m == "Well done you've completed the Treasure Trail!"),
            "said {said:?}"
        );
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(REWARD),
            "the reward box"
        );
        let sent = invs_sent(&packets);
        assert!(sent.contains(&141), "the reward; sent {sent:?}");
        // 289 queues the reward into the backpack as a normal queue, which
        // waits for the reward box to close.
        client.send(ClientProt::CloseModal, &[]);
        ticks(engine, &mut client, 2);
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let backpack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&inv)
            .unwrap();
        assert!(
            (0..backpack.capacity as u16).all(|s| backpack.get(s).is_none_or(|i| i.obj != casket)),
            "the casket is gone"
        );
        assert!(
            (0..backpack.capacity as u16).any(|s| backpack.get(s).is_some()),
            "the reward is in the backpack"
        );
    });
}
