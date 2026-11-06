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

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;
const CLUE_BOX: u16 = 345;
const PUZZLE: u16 = 363;
const REWARD: u16 = 364;
const SEXTANT: u16 = 365;
/// 2011's sextant parts: the horizon (com2, cache position 370,33), the sun
/// (com5, 409,131) and the Tilt Down button (com3).
const HORIZON: i32 = (SEXTANT as i32) << 16 | 2;
const SUN: i32 = (SEXTANT as i32) << 16 | 5;
const TILT_DOWN: i32 = (SEXTANT as i32) << 16 | 3;

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

/// A player in Lumbridge holding `items`, one to a slot from slot 0.
fn holder(engine: &mut Engine, name: &str, items: &[&str]) -> HarnessClient {
    engine.harness_reset();
    let client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let objs: Vec<u16> = items.iter().map(|i| obj(engine, i)).collect();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
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
    let mut p = ClientPacket::new();
    p.p4(com).p2_alt3(obj as i32).p2_alt1(slot as i32);
    client.send(ClientProt::IfButton1, &p.finish());
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<(u16, Vec<u8>)> {
    let mut packets = Vec::new();
    for _ in 0..n {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

fn of(packets: &[(u16, Vec<u8>)], prot: ServerProt) -> impl Iterator<Item = &Vec<u8>> {
    let opcode = prot as u16;
    packets
        .iter()
        .filter(move |(op, _)| *op == opcode)
        .map(|(_, p)| p)
}

/// Each IF_SETTEXT on `iface`: its component and text.
fn texts(packets: &[(u16, Vec<u8>)], iface: u16) -> Vec<(u16, String)> {
    of(packets, ServerProt::IfSetText)
        .filter_map(|p| {
            let com = u32::from_be_bytes([p[0], p[1], p[2], p[3]]);
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            ((com >> 16) as u16 == iface).then(|| {
                (
                    (com & 0xFFFF) as u16,
                    String::from_utf8_lossy(&p[4..end]).into_owned(),
                )
            })
        })
        .collect()
}

/// Each IF_SETPOSITION: `p4_alt3(com) p2_alt3(x) p2_alt3(y)` (the client
/// takes the second as x and the third as y: VarbitType, state change 11).
fn positions(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, i16, i16)> {
    of(packets, ServerProt::IfSetPosition)
        .map(|p| {
            let com = i32::from_be_bytes([p[1], p[0], p[3], p[2]]);
            let x = i16::from_le_bytes([p[4].wrapping_sub(128), p[5]]);
            let y = i16::from_le_bytes([p[6].wrapping_sub(128), p[7]]);
            (com, x, y)
        })
        .collect()
}

/// The inventory ids UPDATE_INV_FULL sent (`p2 inv`).
fn invs_sent(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    of(packets, ServerProt::UpdateInvFull)
        .map(|p| u16::from_be_bytes([p[0], p[1]]))
        .collect()
}

#[test]
fn a_clue_reads_in_2011s_one_clue_box() {
    runner::on_loaded_engine(|engine| {
        let clue = obj(engine, "trail_clue_easy_vague003");
        let mut client = holder(engine, "cluereader", &["trail_clue_easy_vague003"]);
        op1(&mut client, BACKPACK, clue, 0);
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
        op1(&mut client, BACKPACK, text, 0);
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
        op1(&mut client, BACKPACK, map, 1);
        ticks(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        assert!(player.is_interface_open(354), "trail_map09");
        assert!(!player.is_interface_open(CLUE_BOX), "the clue box closed");
    });
}

#[test]
fn the_sextant_moves_on_2011s_coordinates() {
    runner::on_loaded_engine(|engine| {
        let sextant = obj(engine, "sextant");
        let mut client = holder(engine, "navigator", &["sextant"]);
        op1(&mut client, BACKPACK, sextant, 0);
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
        let (hx, hy) = at(&moved, HORIZON).expect("the horizon placed");
        assert_eq!(hx, 370, "{moved:?}");
        assert!((33 - 56..=33 + 42).contains(&hy), "{moved:?}");
        let (sx, sy) = at(&moved, SUN).expect("the sun placed");
        assert_eq!(sx, 409, "{moved:?}");
        assert!((131 - 60..=131 + 58).contains(&sy), "{moved:?}");

        // Tilt Down: the horizon 7 lower, the sun 2 higher.
        let mut p = ClientPacket::new();
        p.p4(TILT_DOWN).p2_alt3(0xFFFF).p2_alt1(0xFFFF);
        client.send(ClientProt::IfButton1, &p.finish());
        let moved = positions(&ticks(engine, &mut client, 2));
        assert_eq!(at(&moved, HORIZON), Some((370, hy + 7)), "{moved:?}");
        assert_eq!(at(&moved, SUN), Some((409, sy - 2)), "{moved:?}");
    });
}

#[test]
fn a_puzzle_box_shows_its_pieces() {
    runner::on_loaded_engine(|engine| {
        let puzzle = obj(engine, "puzzle_box");
        let mut client = holder(engine, "puzzler", &["puzzle_box"]);
        op1(&mut client, BACKPACK, puzzle, 0);
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
        // enabled: `p2_alt2(from) p4_alt3(events) p4_alt3(com) p2_alt3(to)`.
        let events: Vec<(i32, u16, u16, i32)> = of(&packets, ServerProt::IfSetEvents)
            .map(|p| {
                let from = u16::from_be_bytes([p[0], p[1].wrapping_sub(128)]);
                let ops = i32::from_be_bytes([p[3], p[2], p[5], p[4]]);
                let com = i32::from_be_bytes([p[7], p[6], p[9], p[8]]);
                let to = u16::from_le_bytes([p[10].wrapping_sub(128), p[11]]);
                (com, from, to, ops)
            })
            .collect();
        assert!(
            events
                .iter()
                .any(|&(com, from, to, ops)| com == (PUZZLE as i32) << 16 | 4
                    && from == 0
                    && to == 24
                    && ops != 0),
            "the pieces' Move enabled; sent {events:?}"
        );

        // Move a piece beside the gap: it slides into it.
        let pieces = *engine.cache.invs.debugnames.get("trail_puzzleinv").unwrap();
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
        let mut p = ClientPacket::new();
        p.p4((PUZZLE as i32) << 16 | 4)
            .p2_alt3(piece as i32)
            .p2_alt1(beside as i32);
        client.send(ClientProt::IfButton1, &p.finish());
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
        let casket = obj(engine, "casket__easy");
        let mut client = holder(engine, "treasure", &["casket__easy"]);
        // 289's %trail_status: the trail's steps so far in bits 0-3, at an
        // easy trail's last (^trail_easy_maxsteps, 4).
        let status = *engine.cache.varps.debugnames.get("trail_status").unwrap();
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(status, VarValue::Int(4));
        rs_vm::ops::test::set_recording(true);
        rs_vm::ops::test::forget_calls();
        op1(&mut client, BACKPACK, casket, 0);
        let packets = ticks(engine, &mut client, 3);
        let said = rs_vm::ops::test::calls_for("mes");
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
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
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
