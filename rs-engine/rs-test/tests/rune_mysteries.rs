//! 289's Rune Mysteries hooks: the wizards who teleport a player into the
//! rune essence mine, on 2011's npcs and op slots.
//!
//! - Sedridor (npc 300, the Wizards' Tower basement) teleports on op3, as 289
//!   has it, once the quest is complete; before, he says it needs the quest.
//! - Aubury (npc 5913, Varrock's rune shop) trades on op4 and teleports on op5
//!   in 2011 (cache 786), where 289 traded on op3 and teleported on op4.
//!
//! The ops are the client's (OPNPC3: `p1_alt3(ctrl) p2_alt2(nid)`; OPNPC4:
//! `p1_alt3(ctrl) p2_alt3(nid)`; OPNPC5: `p1_alt2(ctrl) p2_alt1(nid)`). The
//! essence mine is map square 45,75.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const SEDRIDOR: u16 = 300;
const AUBURY: u16 = 5913;
const SHOP: u16 = 620;
const COMPLETE: i32 = 6;

// The cases share one loaded engine and each starts with a reset, which would
// log the others' players out: one at a time.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The spawned npc `id`: its nid and tile.
fn npc(engine: &Engine, id: u16) -> (u16, u16, u16) {
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == id)
        .map(|n| (n.npc.uid.nid(), n.npc.pathing.coord))
        .map(|(nid, c)| (nid, c.x(), c.z()))
        .next()
        .unwrap_or_else(|| panic!("npc {id} is spawned"))
}

/// A player beside npc `id`, with Rune Mysteries at `stage`.
fn visitor(engine: &mut Engine, name: &str, id: u16, stage: i32) -> (HarnessClient, u16) {
    engine.harness_reset();
    let (nid, x, z) = npc(engine, id);
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let (sx, sz) = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, -1),
        (2, 0),
        (0, 2),
    ]
    .into_iter()
    .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
    .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
    .expect("a walkable tile beside the npc");
    engine.harness_telejump(client.pid, sx, 0, sz);
    let varp = *engine.cache.varps.debugnames.get("runemysteries").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(varp, rs_pack::cache::VarValue::Int(stage));
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    (client, nid)
}

/// Cycles `n` ticks: the game messages said.
fn run(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

fn in_the_mine(engine: &Engine, pid: u16) -> bool {
    let c = engine.harness_player(pid).unwrap().pathing.coord;
    c.x() / 64 == 45 && c.z() / 64 == 75
}

#[test]
fn sedridor_teleports_a_player_who_finished_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, "apprentice", SEDRIDOR, COMPLETE);
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt2(nid as i32);
        client.send(ClientProt::OpNpc3, &p.finish());
        let said = run(engine, &mut client, 12);
        assert!(
            in_the_mine(engine, client.pid),
            "teleported into the essence mine; said {said:?}"
        );

        // Before the quest is done, he will not.
        let (mut client, nid) = visitor(engine, "novice", SEDRIDOR, 0);
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt2(nid as i32);
        client.send(ClientProt::OpNpc3, &p.finish());
        let said = run(engine, &mut client, 8);
        assert!(!in_the_mine(engine, client.pid), "stays; said {said:?}");
        assert_eq!(
            said,
            ["You need to have completed the Rune Mysteries Quest to use this feature."]
        );
    });
}

#[test]
fn aubury_trades_on_op4_and_teleports_on_op5() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, "runebuyer", AUBURY, COMPLETE);
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt3(nid as i32);
        client.send(ClientProt::OpNpc4, &p.finish());
        let said = run(engine, &mut client, 6);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(SHOP),
            "the rune shop is open; said {said:?}"
        );

        let (mut client, nid) = visitor(engine, "runetraveller", AUBURY, COMPLETE);
        let mut p = ClientPacket::new();
        p.p1_alt2(0).p2_alt1(nid as i32);
        client.send(ClientProt::OpNpc5, &p.finish());
        let said = run(engine, &mut client, 12);
        assert!(
            in_the_mine(engine, client.pid),
            "teleported into the essence mine; said {said:?}"
        );
    });
}
