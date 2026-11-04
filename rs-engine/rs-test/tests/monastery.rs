//! 289's monastery (areas/monastery) at 2011's Edgeville monastery.
//!
//! - Its altar (loc 2640, 3051,3498 upstairs) restores Prayer to two over the
//!   level, as 289 has it and the 2011 wiki's Monastery page still says; the
//!   altars' shared handler only restores to the level.
//! - Its ladder (loc 2641 at 3057,3483) keeps a player out of the order
//!   downstairs, with Abbot Langley (npc 801, beside it) saying so; a member
//!   (`%prayer_guild` 1) climbs.
//!
//! Clicks are the client's OPLOC1 (`ip2(z) ip2(loc) p1(ctrl) p2_alt1(x)`).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const PRAYER: usize = 5;
const ALTAR: (u16, u16, u16) = (2640, 3051, 3498);
const LADDER: (u16, u16, u16) = (2641, 3057, 3483);

// The cases share one loaded engine and each starts with a reset, which would
// log the others' players out: one at a time.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn op_loc1(client: &mut HarnessClient, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    p.ip2(z as i32).ip2(loc as i32).p1(0).p2_alt1(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

/// Every IF_SETTEXT's text in `packets`: what a chat line says.
fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    let opcode = ServerProt::IfSetText as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, p)| {
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            String::from_utf8_lossy(&p[4..end]).into_owned()
        })
        .collect()
}

/// A player at Prayer `level` on a walkable tile beside `(x, z)` on `y`.
fn monk(engine: &mut Engine, name: &str, y: u8, (x, z): (u16, u16), level: u16) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let (sx, sz) = [
        (0, -1),
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -2),
        (2, 0),
        (-1, -1),
        (2, -1),
    ]
    .into_iter()
    .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
    .find(|&(x, z)| !rsmod::is_flagged(x, z, y, 0x100 | 0x200000))
    .expect("a walkable tile beside it");
    engine.harness_telejump(client.pid, sx, y, sz);
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.stats.base_levels[PRAYER] = level;
    player.stats.levels[PRAYER] = level;
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    client
}

/// Cycles `n` ticks: the game messages and the texts set.
fn run(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> (Vec<String>, Vec<String>) {
    let (mut said, mut texts) = (Vec::new(), Vec::new());
    for _ in 0..n {
        engine.cycle();
        let packets = client.packets();
        said.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
        texts.extend(settexts(&packets));
    }
    (said, texts)
}

#[test]
fn the_monastery_altar_restores_two_over_the_level() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = monk(engine, "devout", 1, (ALTAR.1, ALTAR.2), 40);
        op_loc1(&mut client, ALTAR);
        let (said, _) = run(engine, &mut client, 10);
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.stats.levels[PRAYER], 42, "the game said {said:?}");
        assert_eq!(said, ["You boost your Prayer points."]);
    });
}

#[test]
fn the_monastery_ladder_keeps_out_all_but_the_order() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = monk(engine, "outsider", 0, (LADDER.1, LADDER.2), 1);
        op_loc1(&mut client, LADDER);
        let (said, texts) = run(engine, &mut client, 8);
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.pathing.coord.y(),
            0,
            "kept downstairs; said {said:?}, texts {texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|t| t.contains("Only members of our order can go up there.")),
            "Abbot Langley says why; texts {texts:?}"
        );

        // A member of the order climbs.
        let mut client = monk(engine, "member", 0, (LADDER.1, LADDER.2), 31);
        let guild = *engine.cache.varps.debugnames.get("prayer_guild").unwrap();
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .vars
            .set(guild, rs_pack::cache::VarValue::Int(1));
        op_loc1(&mut client, LADDER);
        let (said, _) = run(engine, &mut client, 8);
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.pathing.coord.y(), 1, "upstairs; said {said:?}");
    });
}
