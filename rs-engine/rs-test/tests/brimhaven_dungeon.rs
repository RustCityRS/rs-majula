//! Brimhaven Dungeon from 289, on 2011's map and life points.
//!
//! Saniboch (npc 1595) takes 875 coins (289's fee, and the 2011 wiki's:
//! Brimhaven Dungeon, 16 May 2011) for one trip through the tree door (loc
//! 5083 at 2743,3153), which leads to 2713,9564; the exit beside it (loc 5084)
//! leads back. Cache 786's map places the moss giants' pipes (loc 5100) at
//! 2655,9567 and 2655,9572, the north one a tile further north than 289's, and
//! the squeeze still runs between 289's tiles (2655,9566 and 2655,9573) for
//! 289's 8.5 xp at 22 Agility. A fall into the lava from the stepping stones
//! (loc 5111 at 2647,9558) costs 289's tenth of the current hitpoints and one
//! more, in life points: 100 of 990.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{converse, settexts, talk_to};
use std::sync::Mutex;

const AGILITY: usize = 16;
const HITPOINTS: usize = 3;
/// Holds the life points, shifted by one.
const LIFE: u16 = 1240;

const TREE: (u16, u16, u16) = (5083, 2743, 3153);
const BY_TREE: (u16, u16) = (2745, 3152);
const EXIT: (u16, u16, u16) = (5084, 2714, 9564);
const INSIDE: (u16, u16) = (2713, 9564);
const SOUTH_PIPE: (u16, u16, u16) = (5100, 2655, 9567);
const PIPE_SOUTH_END: (u16, u16) = (2655, 9566);
const PIPE_NORTH_END: (u16, u16) = (2655, 9573);
const SOUTH_STONE: (u16, u16, u16) = (5111, 2647, 9558);
const STONES_SOUTH_END: (u16, u16) = (2647, 9557);
const STONES_NORTH_END: (u16, u16) = (2649, 9562);
const LAVA_BANK: (u16, u16) = (2648, 9557);

// The cases share one loaded engine and each starts with a reset, which would
// log the others' players out: one at a time.
static TURN: Mutex<()> = Mutex::new(());

/// A logged-in player at `at` (level 0), its packets drained.
fn visitor(engine: &mut Engine, name: &str, at: (u16, u16)) -> HarnessClient {
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    assert!(
        !rsmod::is_flagged(at.0, at.1, 0, 0x100 | 0x200000),
        "{at:?} is walkable"
    );
    engine.harness_telejump(client.pid, at.0, 0, at.1);
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

fn at(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

fn paid(engine: &Engine, client: &HarnessClient) -> i32 {
    let varp = *engine
        .cache
        .varps
        .debugnames
        .get("karam_dungeon_varbit")
        .unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(varp)
        .as_int()
}

fn coins(engine: &Engine, client: &HarnessClient) -> u32 {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let coins = *engine.cache.objs.debugnames.get("coins").unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .map_or(0, |i| i.total(coins))
}

#[test]
fn the_tree_door_stays_shut_to_those_who_have_not_paid() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = visitor(engine, "unpaid", BY_TREE);
        op_loc1(&mut client, TREE);
        let mut texts = Vec::new();
        for _ in 0..8 {
            engine.cycle();
            texts.extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
        }
        assert_eq!(at(engine, &client), BY_TREE);
        assert!(
            texts.iter().any(|t| t.contains("without paying")),
            "Saniboch objects: {texts:?}"
        );
    });
}

#[test]
fn saniboch_takes_875_coins_for_one_trip_down_and_the_exit_leads_back() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = visitor(engine, "wealthy", BY_TREE);
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let coin = *engine.cache.objs.debugnames.get("coins").unwrap();
        let saniboch = *engine
            .cache
            .npcs
            .debugnames
            .get("tribesman_doorman")
            .unwrap();
        let sanibochs: Vec<u16> = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .filter(|n| n.npc.uid.id() == saniboch)
            .map(|n| n.npc.uid.nid())
            .collect();
        assert_eq!(sanibochs.len(), 1, "one Saniboch");
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, coin, 1000);
        engine.cycle();
        client.packets();

        talk_to(&mut client, sanibochs[0]);
        let talk = converse(
            engine,
            &mut client,
            &[
                "Can I go through that door please?",
                "Ok, here's 875 coins.",
            ],
            60,
        );
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        assert!(talk.said("You may now pass the door."), "{:?}", talk.texts);
        assert_eq!(talk.messages, ["You pay Saniboch 875 coins."]);
        assert_eq!(coins(engine, &client), 125);
        assert_eq!(paid(engine, &client), 1);

        op_loc1(&mut client, TREE);
        for _ in 0..10 {
            engine.cycle();
            if at(engine, &client) == INSIDE {
                break;
            }
        }
        assert_eq!(at(engine, &client), INSIDE);
        assert_eq!(paid(engine, &client), 0, "the fee is for one trip");

        op_loc1(&mut client, EXIT);
        for _ in 0..10 {
            engine.cycle();
            if at(engine, &client) == BY_TREE {
                break;
            }
        }
        assert_eq!(at(engine, &client), BY_TREE);
    });
}

#[test]
fn the_moss_giant_pipe_runs_between_289s_tiles_at_22_agility() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = visitor(engine, "squeezer", PIPE_SOUTH_END);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.stats.levels[AGILITY] = 21;
        player.stats.base_levels[AGILITY] = 21;
        op_loc1(&mut client, SOUTH_PIPE);
        let mut texts = Vec::new();
        for _ in 0..6 {
            engine.cycle();
            texts.extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
        }
        assert_eq!(at(engine, &client), PIPE_SOUTH_END);
        assert!(
            texts
                .join(" ")
                .replace("<br>", " ")
                .contains("You need an Agility level of 22 to squeeze through the pipe."),
            "{texts:?}"
        );

        engine.harness_reset();
        let mut client = visitor(engine, "squeezed", PIPE_SOUTH_END);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.stats.levels[AGILITY] = 22;
        player.stats.base_levels[AGILITY] = 22;
        let xp = player.stats.xp[AGILITY];
        op_loc1(&mut client, SOUTH_PIPE);
        for _ in 0..20 {
            engine.cycle();
            if engine.harness_player(client.pid).unwrap().stats.xp[AGILITY] > xp {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.stats.xp[AGILITY] - xp, 85, "8.5 xp");
        assert_eq!(at(engine, &client), PIPE_NORTH_END);
    });
}

/// 1 Agility falls about one crossing in six from the south stone, which asks
/// no level (289's): up to forty crossings for a fall. The stumbler's melee
/// stats are 99 and its combat level follows, so the moss giant by the stones
/// (level 42, hunting only those not too strong for it) leaves it alone, and
/// its 89 Hitpoints cap the natural recovery at 890 life points, so none
/// lands on the tick of the hit: the fall is the only change.
#[test]
fn a_fall_into_the_lava_costs_a_tenth_of_the_life_points_and_ten_more() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = visitor(engine, "stumbler", STONES_SOUTH_END);
        let life = |engine: &Engine, pid| {
            (engine.harness_player(pid).unwrap().vars.get(LIFE).as_int() >> 1) & 0x7FFF
        };
        let mut lost = None;
        for _ in 0..40 {
            engine.harness_telejump(client.pid, STONES_SOUTH_END.0, 0, STONES_SOUTH_END.1);
            let player = engine.harness_player_mut(client.pid).unwrap();
            for (stat, level) in [(0, 99), (1, 99), (2, 99), (HITPOINTS, 89)] {
                player.stats.levels[stat] = level;
                player.stats.base_levels[stat] = level;
            }
            player.combat_level = player.get_combat_level();
            assert!(player.combat_level > 84, "too strong for a moss giant");
            let lp = player.vars.get(LIFE).as_int();
            player.vars.set(LIFE, VarValue::Int((lp & 1) | 990 << 1));
            engine.cycle();
            engine.cycle();
            client.packets();
            op_loc1(&mut client, SOUTH_STONE);
            let mut fell = false;
            let mut drops = Vec::new();
            let pid = client.pid;
            let mut was = life(engine, pid);
            let mut tick = |engine: &mut Engine, drops: &mut Vec<i32>| {
                engine.cycle();
                let now = life(engine, pid);
                if now < was {
                    drops.push(was - now);
                }
                was = now;
            };
            for _ in 0..30 {
                tick(engine, &mut drops);
                for (_, t) in HarnessClient::game_messages(&client.packets()) {
                    fell |= t == "...You lose your footing and fall into the lava.";
                }
                let here = at(engine, &client);
                if here == STONES_NORTH_END || (fell && here == LAVA_BANK) {
                    break;
                }
            }
            if fell {
                // The hit lands with the fall's queue, the tick after.
                tick(engine, &mut drops);
                tick(engine, &mut drops);
                assert_eq!(at(engine, &client), LAVA_BANK);
                lost = Some(drops);
                break;
            }
            assert_eq!(at(engine, &client), STONES_NORTH_END, "crossed");
        }
        assert_eq!(lost, Some(vec![100]), "a fall in forty crossings, for 100");
    });
}
