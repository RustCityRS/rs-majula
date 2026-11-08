//! Agility from 289, with 2011's stances.
//!
//! 289 sets each base animation by seq while a player balances or climbs;
//! 2011's appearance carries one render anim (BAS), which pairs a ready and a
//! walk seq, so 289's log-balance seqs (763, 762) are 2011's BAS 155
//! (`void-634`'s `rope_balance`). The case walks the Gnome Stronghold course's
//! log (loc 2295) as the client clicks it (OPLOC1: `p2_alt1(z) p2_alt1(loc) p1(ctrl)
//! p2_alt2(x)`): the player crosses seven tiles south in the balance stance,
//! takes 7.5 xp, and stands as before on the far side. Agility pays through
//! `~givexp`, so 2011's xp counter (varp 1801, in tenths) counts it.
//!
//! The Barbarian Outpost's obstacle net (loc 20211 at 2538,3545) pays 2011's
//! 8.2 xp (2011 wiki, Barbarian Outpost), where 289 pays 7.5, and lifts the
//! climber to the ledge level above.
//!
//! The PR's 289 gives each of the course's two obstacle pipes its own "being
//! used" timer, so two players squeeze through 2011's two pipes (43543 at
//! 2487,3431 and 43544 at 2483,3431) side by side. And the Wilderness course's
//! stepping stones wait for the player to arrive and land the last jump a tick
//! sooner.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use std::sync::Mutex;

const AGILITY: usize = 16;
/// 2011's xp counter, which counts ten times what `~givexp` pays.
const XP_COUNTER: u16 = 1801;

// The cases share one loaded engine and each starts with a reset, which would
// log the others' players out: one at a time.
static TURN: Mutex<()> = Mutex::new(());

/// 289's `log_balance`, the course's first obstacle, at 2474,3435.
const LOG: (u16, u16, u16) = (2295, 2474, 3435);
const START: (u16, u16) = (2474, 3436);
const END: (u16, u16) = (2474, 3429);
const BALANCE_BAS: u16 = 155;
const HUMAN_BAS: u16 = 1426;

#[test]
fn the_gnome_course_log_in_2011s_balance_stance() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("acrobat").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, START.0, 0, START.1);
        engine.cycle();
        engine.cycle();
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[AGILITY];
        let counted = engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(XP_COUNTER)
            .as_int();
        client.packets();

        let (loc, x, z) = LOG;
        let mut p = ClientPacket::new();
        p.p2_alt1(z as i32)
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());

        let mut said = Vec::new();
        let mut stances = Vec::new();
        for _ in 0..20 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            let player = engine.harness_player(client.pid).unwrap();
            stances.push(player.info.readyanim);
            let c = player.pathing.coord;
            if (c.x(), c.z()) == END && said.len() >= 2 {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        let c = player.pathing.coord;
        assert_eq!((c.x(), c.z()), END, "the game said {said:?}");
        assert_eq!(
            said,
            [
                "You walk carefully across the slippery log...",
                "...You make it safely to the other side.",
            ]
        );
        assert_eq!(player.stats.xp[AGILITY] - xp, 75);
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 750);
        assert!(
            stances.contains(&Some(BALANCE_BAS)),
            "balanced in BAS {BALANCE_BAS}; stances {stances:?}"
        );
        assert_eq!(player.info.readyanim, Some(HUMAN_BAS), "stands as before");
    });
}

#[test]
fn the_barbarian_net_pays_2011s_xp() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (loc, x, z) = (20211u16, 2538u16, 3545u16);
        engine.harness_reset();
        let mut client = engine.harness_login_returning("climber").expect("login");
        engine.cycle();
        // East of the net, the side 289 climbs it from.
        engine.harness_telejump(client.pid, x + 1, 0, z);
        engine.cycle();
        engine.cycle();
        let player = engine.harness_player(client.pid).unwrap();
        let xp = player.stats.xp[AGILITY];
        let counted = player.vars.get(XP_COUNTER).as_int();
        client.packets();

        let mut p = ClientPacket::new();
        p.p2_alt1(z as i32)
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let mut said = Vec::new();
        for _ in 0..10 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            if engine.harness_player(client.pid).unwrap().stats.xp[AGILITY] > xp {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(said, ["You climb the netting..."]);
        assert_eq!(player.stats.xp[AGILITY] - xp, 82, "8.2 xp");
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 820);
        let c = player.pathing.coord;
        assert_eq!(
            (c.x(), c.y(), c.z()),
            (x - 1, 1, z + 1),
            "on the ledge level"
        );
    });
}

/// A logged-in player at `x`,`z` (level 0) with `agility`, its packets drained.
fn athlete(engine: &mut Engine, name: &str, x: u16, z: u16, agility: u16) -> HarnessClient {
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    assert!(
        !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000),
        "{x},{z} is walkable"
    );
    engine.harness_telejump(client.pid, x, 0, z);
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.stats.levels[AGILITY] = agility;
    player.stats.base_levels[AGILITY] = agility;
    engine.cycle();
    engine.cycle();
    client.packets();
    client
}

fn op_loc1(client: &mut HarnessClient, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

#[test]
fn two_players_squeeze_through_the_two_gnome_pipes_at_once() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut east = athlete(engine, "pipeeast", 2487, 3430, 99);
        let mut west = athlete(engine, "pipewest", 2483, 3430, 99);
        op_loc1(&mut east, (43543, 2487, 3431));
        op_loc1(&mut west, (43544, 2483, 3431));
        let mut said = Vec::new();
        for _ in 0..20 {
            engine.cycle();
            for client in [&mut east, &mut west] {
                said.extend(
                    HarnessClient::game_messages(&client.packets())
                        .into_iter()
                        .map(|(_, t)| t),
                );
            }
        }
        assert!(
            !said.iter().any(|s| s == "The pipe is being used"),
            "{said:?}"
        );
        for (client, x) in [(&east, 2487), (&west, 2483)] {
            let c = engine.harness_player(client.pid).unwrap().pathing.coord;
            assert_eq!((c.x(), c.z()), (x, 3437), "through the pipe");
        }
    });
}

/// 786 places the course's stepping stone (37704) at 3001,3960. From the tile
/// east of it, the ticks from the op to the first message, and from it to the
/// last, of a crossing that does not fall.
#[test]
fn the_wilderness_stepping_stones_wait_for_arrival_and_land_sooner() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let stone = (37704u16, 3001u16, 3960u16);
        let mut timings = Vec::new();
        // 99 Agility still falls about one crossing in 43: up to five tries.
        for attempt in 0..5 {
            engine.harness_reset();
            let mut client = athlete(engine, &format!("stones{attempt}"), 3003, 3960, 99);
            op_loc1(&mut client, stone);
            let (mut first, mut last, mut fell) = (None, None, false);
            for tick in 1..40 {
                engine.cycle();
                for (_, t) in HarnessClient::game_messages(&client.packets()) {
                    if t == "You carefully start crossing the stepping stones..." {
                        first = Some(tick);
                    } else if t == "...You safely cross to the other side." {
                        last = Some(tick);
                    } else if t.contains("lose your footing") {
                        fell = true;
                    }
                }
                if last.is_some() || fell {
                    break;
                }
            }
            if fell {
                continue;
            }
            let (first, last) = (
                first.expect("crossing starts"),
                last.expect("crossing ends"),
            );
            timings.push((first, last - first));
            break;
        }
        assert_eq!(timings, [(3, 11)], "(op to start, start to end)");
    });
}
