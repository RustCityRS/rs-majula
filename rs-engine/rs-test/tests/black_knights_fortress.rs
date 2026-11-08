//! Black Knights' Fortress at 647, after its rework (2011 wiki, 2011-04-27): Sir Amik's 2011 lines
//! and dossier at 13 quest points, the witch's lines at the grill, the cabbage on the hole with the
//! minimap blacked out, and the reward (2011-05-04 recording, YouTube TZC9nJHK7bI; transcript in
//! runescape/target/refs/bkf/transcript_0504.md). %spy is the cache's varp 130, which quest 53
//! counts from 1 to 4.

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
use rs_test::talk::{converse, settexts, talk_to};
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const BACKPACK: i32 = 149 << 16;
/// The grill (maps/l47_54.jm2: level 0, 18,51) and the hole (level 1, 23,51).
const GRILL: (u16, u16, u16, u8) = (2342, 3026, 3507, 0);
const HOLE: (u16, u16, u16, u8) = (2336, 3031, 3507, 1);

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
    let names = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
        _ => &engine.cache.invs.debugnames,
    };
    *names
        .get(name)
        .unwrap_or_else(|| panic!("no {kind} {name}"))
}

fn spy(engine: &Engine, client: &HarnessClient) -> i32 {
    let varp = id(engine, "varp", "spy");
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(varp)
        .as_int()
}

/// A walkable tile beside (x, z) on `level`.
fn beside(x: u16, z: u16, level: u8) -> (u16, u16) {
    [
        (1i16, 0i16),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
    ]
    .into_iter()
    .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
    .find(|&(x, z)| !rsmod::is_flagged(x, z, level, 0x100 | 0x200000))
    .unwrap_or_else(|| panic!("a walkable tile beside {x},{z} level {level}"))
}

/// Sir Amik's nid, after moving the player beside him.
fn to_sir_amik(engine: &mut Engine, client: &HarnessClient) -> u16 {
    let amik = id(engine, "npc", "sir_amik_varze");
    let (nid, x, y, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == amik)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.y(), c.z())
        })
        .expect("Sir Amik is spawned");
    let (sx, sz) = beside(x, z, y);
    engine.harness_telejump(client.pid, sx, y, sz);
    for _ in 0..3 {
        engine.cycle();
    }
    nid
}

fn minimap_states(packets: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let opcode = ServerProt::MinimapToggle as u16;
    packets
        .iter()
        .filter(|(op, p)| *op == opcode && p.len() == 1)
        .map(|(_, p)| p[0])
        .collect()
}

#[test]
fn black_knights_fortress_as_2011_had_it() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let inv = id(engine, "inv", "inv");
        let (dossier, cabbage, coins) = (
            id(engine, "obj", "dossier"),
            id(engine, "obj", "cabbage"),
            id(engine, "obj", "coins"),
        );
        let qp = id(engine, "varp", "qp");
        let _ = engine.script_errors.take();
        engine.harness_reset();
        let mut client: HarnessClient = engine.harness_login_returning("bkf_2011").expect("login");
        for _ in 0..10 {
            engine.cycle();
        }
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(qp, VarValue::Int(13));

        // Sir Amik: the 2011 start, and the dossier.
        let nid = to_sir_amik(engine, &client);
        client.packets();
        talk_to(&mut client, nid);
        let start = converse(
            engine,
            &mut client,
            &[
                "I seek a quest!",
                "I laugh in the face of danger!",
                "Ok, I'll do my best.",
            ],
            400,
        );
        assert_eq!(start.unanswered, None, "{:?}", start.texts);
        for line in [
            "I am the leader of the White Knights of Falador. Why do you seek my audience?",
            "Well, I need some spy work doing but it's quite dangerous. It will involve going into the Black Knights' fortress.",
            "I laugh in the face of danger!",
            "You've come along at just the right time actually. All of my knights are already known to the Black Knights.",
            "That information is classified. I am forbidden to share it with outsiders.",
            "Your mission, should you decide to accept it, is to infiltrate their fortress, find out what their secret weapon is, and then sabotage it.",
            "Good luck! Let me know how you get on. Here's the dossier for the case, I've already given you the details.",
        ] {
            assert!(start.said(line), "2011 shows {line:?}; {:?}", start.texts);
        }
        assert_eq!(spy(engine, &client), 1, "started");
        let pack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&inv)
            .unwrap();
        assert_eq!(pack.total(dossier), 1, "the dossier");

        // The grill: the witch's 2011 lines.
        let (loc, x, z, level) = GRILL;
        // A wall decoration (shape 5, angle 2) on its tile's east wall: listened at from its tile.
        engine.harness_telejump(client.pid, x, level, z);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        let mut p = ClientPacket::new();
        p.p2_alt1(z as i32)
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let grill = converse(engine, &mut client, &[], 400);
        for line in [
            "So... how's the secret weapon coming along?",
            "It's taken me FIVE YEARS, but it's almost ready.",
            "Now, remember, Greldo, only a Draynor Manor cabbage will do! Don't get lazy and bring any old cabbage, THAT would ENTIRELY wreck the potion!",
            "Yeth, mithtreth.",
        ] {
            assert!(grill.said(line), "2011 shows {line:?}; {:?}", grill.texts);
        }
        assert_eq!(spy(engine, &client), 2, "the weapon found out");

        // The cabbage on the hole: the witch's room, the minimap blacked out.
        let (loc, x, z, level) = HOLE;
        let (sx, sz) = beside(x, z, level);
        engine.harness_telejump(client.pid, sx, level, sz);
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .set(5, cabbage, 1);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        let mut p = ClientPacket::new();
        p.p2_alt2(cabbage as i32)
            .p2_alt2(5)
            .p2(z as i32)
            .p4_alt2(BACKPACK)
            .p2_alt2(x as i32)
            .p1_alt1(0)
            .p2_alt1(loc as i32);
        client.send(ClientProt::OpLocT, &p.finish());
        let sabotage = converse(engine, &mut client, &[], 400);
        for line in [
            "Where has Greldo got to with that magic cabbage!",
            "My potion!",
            "Meow!",
            "Looks like my work here is done. Seems like that's successfully sabotaged their little secret weapon plan.",
        ] {
            assert!(
                sabotage.said(line),
                "2011 shows {line:?}; {:?}",
                sabotage.texts
            );
        }
        assert_eq!(
            minimap_states(&sabotage.packets),
            [2, 0],
            "the minimap blacked out, then back"
        );
        assert_eq!(spy(engine, &client), 3, "sabotaged");

        // Sir Amik's reward.
        let nid = to_sir_amik(engine, &client);
        client.packets();
        talk_to(&mut client, nid);
        let mut reward = converse(engine, &mut client, &[], 400);
        // The scroll opens from the queue after the last box.
        for _ in 0..3 {
            engine.cycle();
            reward
                .texts
                .extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
        }
        for line in [
            "I have ruined the Black Knights' invincibility potion. That should put a stop to your problem and an end to their little schemes.",
            "Now, I believe there was some talk of a cash reward...",
            "Absolutely right. Please accept this reward.",
            "Sir Amik hands you 2,500 coins.",
            "You have completed Black Knights' Fortress!",
            "2500 Coins",
        ] {
            assert!(reward.said(line), "2011 shows {line:?}; {:?}", reward.texts);
        }
        for _ in 0..3 {
            engine.cycle();
        }
        assert_eq!(spy(engine, &client), 4, "complete");
        let pack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&inv)
            .unwrap();
        assert_eq!(pack.total(coins), 2500, "2,500 coins");
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}

/// 289's six fortress guards, at 289's tiles: 647's spawn file had none, which left the dining-room
/// door shut (it opens only through the guard's warning).
#[test]
fn the_fortress_guards_are_spawned() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let guard = id(engine, "npc", "fortress_guard");
        let count = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .filter(|n| n.npc.uid.id() == guard)
            .count();
        assert_eq!(count, 6, "289's six fortress guards");
    });
}
