//! 2011's reworked Restless Ghost, played through as the client does:
//! Father Aereck (456) starts it; Father Urhney (458) gives the Amulet of
//! Ghostspeak; the ghost (457), risen from the opened coffin (2145 at
//! 3249,3192), tells of its skull; the swamp rocks (47713 at 3234,3145) give
//! the muddy skull and raise a skeleton warlock; the skull in the open coffin
//! (15061) ends it with the 2011 wiki's reward (The Restless Ghost,
//! 2011-05-22): 125 Prayer experience and 5 ancient bones.
//!
//! Loc ops are OPLOC1 (`p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`) and OPLOCT
//! (`p2_alt2(obj) p2_alt2(slot) p2(z) p4_alt2(com) p2_alt2(x) p1_alt1(ctrl)
//! p2_alt1(loc)`).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{converse, talk_to};

const AERECK: u16 = 456;
const GHOST: u16 = 457;
const URHNEY: u16 = 458;
const WARLOCK: u16 = 459;
const PRIESTSTART: u16 = 107;
const PRAYER: usize = 5;
/// `inventory:inv`.
const BACKPACK: i32 = 149 << 16;
const COFFIN: (u16, u16, u16) = (2145, 3249, 3192);
const OPEN_COFFIN: u16 = 15061;
const ROCKS: (u16, u16, u16) = (47713, 3234, 3145);

fn id(engine: &Engine, obj: &str) -> u16 {
    *engine.cache.objs.debugnames.get(obj).unwrap()
}

/// A player at `stage`, beside the npc `npc` (or at `at`), carrying `objs`
/// and wearing `worn`.
fn player(
    engine: &mut Engine,
    stage: i32,
    objs: &[&str],
    worn: Option<&str>,
    at: (u16, u16),
) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine
        .harness_login_returning("ghostbuster")
        .expect("login");
    for _ in 0..10 {
        engine.cycle();
    }
    engine.harness_telejump(client.pid, at.0, 0, at.1);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let worn_inv = *engine.cache.invs.debugnames.get("worn").unwrap();
    let ids: Vec<u16> = objs.iter().map(|o| id(engine, o)).collect();
    let worn_id = worn.map(|o| id(engine, o));
    let p = engine.harness_player_mut(client.pid).unwrap();
    p.vars.set(PRIESTSTART, VarValue::Int(stage));
    let backpack = p.invs.get_mut(&inv).unwrap();
    for (slot, id) in ids.into_iter().enumerate() {
        backpack.set(slot as u16, id, 1);
    }
    if let Some(w) = worn_id {
        // The amulet's slot: wearpos front is 2.
        p.invs.get_mut(&worn_inv).unwrap().set(2, w, 1);
    }
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    client
}

fn npc_near(engine: &Engine, npc: u16) -> Option<(u16, u16, u16)> {
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == npc)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
}

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(PRIESTSTART)
        .as_int()
}

fn total(engine: &Engine, client: &HarnessClient, inv: &str, obj: &str) -> i64 {
    let inv = *engine.cache.invs.debugnames.get(inv).unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(id(engine, obj)) as i64
}

fn oploc1(client: &mut HarnessClient, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

fn messages(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        let packets = client.packets();
        said.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
        // Interface texts too (the quest scroll's lines).
        said.extend(
            rs_test::talk::settexts(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

fn beside(engine: &Engine, npc: u16) -> (u16, (u16, u16)) {
    let (nid, x, z) = npc_near(engine, npc).expect("spawned");
    (nid, (x + 1, z))
}

/// "I'm looking for a quest." opens 2011's quest-start window (interface 178), and its Yes
/// (component 73, IF_BUTTON1 `p4(com) p2_alt3(obj) p2_alt1(slot)`) starts the quest with 2011's
/// lines (2011-04-20 and 2011-06-24 recordings).
#[test]
fn father_aereck_starts_the_quest() {
    runner::on_loaded_engine(|engine| {
        let (nid, at) = beside(engine, AERECK);
        let mut client = player(engine, 0, &[], None, at);
        talk_to(&mut client, nid);
        let talk = converse(engine, &mut client, &["I'm looking for a quest."], 60);
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(178),
            "the quest-start window; {:?}",
            talk.texts
        );
        let mut p = ClientPacket::new();
        p.p4((178 << 16) | 73).p2_alt3(0xFFFF).p2_alt1(0xFFFF);
        client.send(ClientProt::IfButton1, &p.finish());
        let talk = converse(engine, &mut client, &[], 60);
        assert!(
            talk.said("He's currently living in a little shack in the south of Lumbridge Swamps, near the coast."),
            "{:?}",
            talk.texts
        );
        assert_eq!(stage(engine, &client), 1);
    });
}

#[test]
fn father_urhney_gives_the_amulet() {
    runner::on_loaded_engine(|engine| {
        let (nid, at) = beside(engine, URHNEY);
        let mut client = player(engine, 1, &[], None, at);
        talk_to(&mut client, nid);
        let talk = converse(
            engine,
            &mut client,
            &[
                "Father Aereck sent me to talk to you.",
                "He's got a ghost haunting his graveyard.",
            ],
            80,
        );
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        assert_eq!(stage(engine, &client), 2);
        assert_eq!(total(engine, &client, "inv", "ghostspeak_amulet"), 1);
    });
}

#[test]
fn the_ghost_rises_and_asks_for_its_skull() {
    runner::on_loaded_engine(|engine| {
        let mut client = player(engine, 2, &[], Some("ghostspeak_amulet"), (3249, 3191));
        oploc1(&mut client, COFFIN);
        messages(engine, &mut client, 6);
        let (nid, _, _) = npc_near(engine, GHOST).expect("the ghost rises from the coffin");
        talk_to(&mut client, nid);
        let talk = converse(
            engine,
            &mut client,
            &["Yep, now tell me what the problem is."],
            80,
        );
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        assert!(talk.said("attacked by a warlock"));
        assert_eq!(stage(engine, &client), 3);
    });
}

#[test]
fn the_rocks_give_the_skull_and_raise_a_warlock() {
    runner::on_loaded_engine(|engine| {
        // A walkable tile beside the 2x2 rocks.
        let at = [
            (-1, 0),
            (-1, 1),
            (0, -1),
            (1, -1),
            (2, 0),
            (2, 1),
            (0, 2),
            (1, 2),
        ]
        .into_iter()
        .map(|(dx, dz)| {
            (
                ROCKS.1.wrapping_add_signed(dx),
                ROCKS.2.wrapping_add_signed(dz),
            )
        })
        .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
        .expect("a walkable tile beside the rocks");
        let mut client = player(engine, 3, &[], None, at);
        oploc1(&mut client, ROCKS);
        // "You take the skull from the pile of rocks." is a message box.
        let said = converse(engine, &mut client, &[], 30).messages;
        assert!(
            said.iter().any(|m| m == "A skeleton warlock has appeared."),
            "{said:?}"
        );
        assert_eq!(total(engine, &client, "inv", "muddy_skull"), 1);
        assert_eq!(stage(engine, &client), 4);
        assert!(npc_near(engine, WARLOCK).is_some(), "the warlock");
        // rocks_restless_ghost (varbit 2130): bit 2 of varp 728.
        let v = engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(728)
            .as_int();
        assert_eq!(v >> 2 & 1, 1, "the rocks show no skull");
    });
}

#[test]
fn the_skull_in_the_coffin_lays_the_ghost_to_rest() {
    runner::on_loaded_engine(|engine| {
        let skull = id(engine, "muddy_skull");
        let mut client = player(engine, 4, &["muddy_skull"], None, (3249, 3191));
        let prayer = engine.harness_player(client.pid).unwrap().stats.xp[PRAYER];
        oploc1(&mut client, COFFIN);
        messages(engine, &mut client, 6);
        let mut p = ClientPacket::new();
        p.p2_alt2(skull as i32)
            .p2_alt2(0)
            .p2(COFFIN.2 as i32)
            .p4_alt2(BACKPACK)
            .p2_alt2(COFFIN.1 as i32)
            .p1_alt1(0)
            .p2_alt1(OPEN_COFFIN as i32);
        client.send(ClientProt::OpLocT, &p.finish());
        let said = messages(engine, &mut client, 10);
        assert!(
            said.iter().any(|m| m == "You put the skull in the coffin."),
            "{said:?}"
        );
        // 2011's scroll (2011-04-20 i7PABMV6o2k, 351s).
        for line in [
            "You have completed The Restless Ghost!",
            "1 Quest Point,",
            "125 Prayer XP and five",
            "ancient bones",
        ] {
            assert!(
                said.iter().any(|m| m == line),
                "the 2011 scroll shows {line:?}; {said:?}"
            );
        }
        assert_eq!(stage(engine, &client), 5, "complete");
        assert_eq!(total(engine, &client, "inv", "muddy_skull"), 0);
        assert_eq!(total(engine, &client, "inv", "ancient_bones"), 5);
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.xp[PRAYER] - prayer,
            1250,
            "125 Prayer xp, in tenths"
        );
        let v = engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(728)
            .as_int();
        assert_eq!(v >> 1 & 1, 1, "the coffin shows the skull");
    });
}
