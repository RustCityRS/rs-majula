//! Thieving from 289, at 2011's npcs, stalls and chests.
//!
//! Each case clicks as the 647 client does (`OPNPC3`: `g1_alt3(ctrl)
//! g2_alt2(npc)`; `OPLOC1`/`OPLOC2`: `g2_alt1(loc) g1_alt2(ctrl) g2_alt2(z)
//! g2_alt3(x)`) with the player at 99 Thieving, so 289's success rolls
//! almost always pass, and waits for the outcome.
//!
//! - A Lumbridge man (2011's `man_7873`-`man_7879`, void-634's
//!   `lumbridge_man`, joined to 289's man row) pays 289's 3 coins and 8 xp.
//! - 2011's Ardougne market keeps 289's baker's stall at 2667,3310 (2011's
//!   loc 34384, which took 289's `bakers_stall_stealing`) and 289's baker
//!   beside it at 2669,3310. From the stall's south side the stall hides the
//!   thief from the baker, and the stall gives a cake, bread or a chocolate
//!   slice for 16 xp; in the baker's sight he stops the theft.
//! - The 10-coin chest (loc 2566, 289's `chest_10_coins`) at 2676,3310:
//!   searched for traps it gives 10 coins and 7.8 xp; opened without, its trap
//!   takes 289's damage (a tenth of the current life points times 12/100,
//!   plus 3 hitpoints) as life points.
//!
//! Thieving pays through `~givexp`, so 2011's xp counter (varp 1801, in tenths
//! of the xp) counts each award.
//!
//! The PR's 289 marks a robbed stall on the cache's `marketthief` varbits
//! (786's 158-170 on varp 174, bits 1-13, as 289's), lets Miscellania's and
//! Etceteria's stalls be robbed, and calls the guards that carry its
//! `market_guard` param.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::ParamValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const THIEVING: usize = 17;
const LUMBRIDGE_MEN: std::ops::RangeInclusive<u16> = 7873..=7879;
const BAKERS_STALL: (u16, u16, u16) = (34384, 2667, 3310);
const CHEST_10_COINS: (u16, u16, u16) = (2566, 2676, 3310);
/// Miscellania's veg stall, 289's `misc_veg_market` (786 places it at
/// 0_39_60_21_22).
const MISC_VEG_STALL: (u16, u16, u16) = (4706, 2517, 3862);
/// 289's `marketthief`, the cache's varp 174; `marketthief_cake` is its bit 1.
const MARKETTHIEF: u16 = 174;

/// Each 647 OPLOC has its own field order (`op_loc1.rs`, `op_loc2.rs`).
fn op_loc(client: &mut HarnessClient, prot: ClientProt, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    match prot {
        ClientProt::OpLoc1 => p
            .p2_alt1(z as i32)
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(x as i32),
        _ => p
            .p2_alt1(loc as i32)
            .p1_alt2(0)
            .p2_alt2(z as i32)
            .p2_alt3(x as i32),
    };
    client.send(prot, &p.finish());
}

/// A logged-in player at 99 Thieving, standing at `x`,`z`.
fn thief(engine: &mut Engine, name: &str, x: u16, z: u16) -> HarnessClient {
    engine.harness_reset();
    // 289 refuses thieving within 10 ticks of `%lastcombat`, 0 for a new
    // player, and a reset world's clock starts near 0.
    for _ in 0..20 {
        engine.cycle();
    }
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.stats.levels[THIEVING] = 99;
    player.stats.base_levels[THIEVING] = 99;
    engine.harness_telejump(client.pid, x, 0, z);
    for _ in 0..12 {
        engine.cycle();
    }
    client.packets();
    client
}

/// Cycles until `done` holds or `ticks` pass; the game messages said meanwhile.
fn run(
    engine: &mut Engine,
    client: &mut HarnessClient,
    ticks: usize,
    done: impl Fn(&Engine, u16) -> bool,
) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..ticks {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
        if done(engine, client.pid) {
            break;
        }
    }
    said
}

fn count(engine: &Engine, pid: u16, obj: &str) -> u32 {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let obj = *engine.cache.objs.debugnames.get(obj).unwrap();
    engine
        .harness_player(pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(obj)
}

fn xp(engine: &Engine, pid: u16) -> i32 {
    engine.harness_player(pid).unwrap().stats.xp[THIEVING]
}

/// 2011's xp counter, which counts ten times what `~givexp` pays.
fn counted(engine: &Engine, pid: u16) -> i32 {
    engine.harness_player(pid).unwrap().vars.get(1801).as_int()
}

#[test]
fn a_lumbridge_man_s_pocket_pays_three_coins() {
    runner::on_loaded_engine(|engine| {
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .map(|n| (n.npc.uid.nid(), n.npc.uid.id(), n.npc.pathing.coord))
            .find(|(_, id, _)| LUMBRIDGE_MEN.contains(id))
            .map(|(nid, _, c)| (nid, c.x(), c.z()))
            .expect("a man of Lumbridge");
        let mut client = thief(engine, "pickpocket", x + 1, z);
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt2(nid as i32);
        client.send(ClientProt::OpNpc3, &p.finish());
        let said = run(engine, &mut client, 20, |e, pid| count(e, pid, "coins") > 0);
        assert_eq!(
            count(engine, client.pid, "coins"),
            3,
            "the game said {said:?}"
        );
        assert_eq!(xp(engine, client.pid), 80, "8 xp");
        assert_eq!(counted(engine, client.pid), 800, "the xp counter counts it");
        assert_eq!(
            said.iter().map(String::as_str).collect::<Vec<_>>(),
            [
                "You attempt to pick the man's pocket.",
                "You pick the man's pocket."
            ]
        );
    });
}

#[test]
fn the_bakers_stall_hides_a_thief_from_the_baker() {
    runner::on_loaded_engine(|engine| {
        let mut client = thief(engine, "stallthief", 2667, 3309);
        // The baker and the market's Ardougne guards wander (wanderrange,
        // loaded since c7b4f69), and the stall's row stops a theft either sees
        // within 5 tiles (stealing_check_for_guard, _for_owner). The stall
        // hides the thief only while they stand behind it: wait for a tick
        // when none can see the thief's tile, as a player would.
        let watchers: Vec<u16> = ["baker", "ardougne_guard"]
            .iter()
            .map(|n| *engine.cache.npcs.debugnames.get(*n).unwrap())
            .collect();
        for tick in 0..200 {
            let seen = engine.npc_list.npcs.iter().flatten().any(|n| {
                let c = n.npc.pathing.coord;
                watchers.contains(&n.npc.uid.id())
                    && c.x().abs_diff(2667) <= 5
                    && c.z().abs_diff(3309) <= 5
                    && rsmod::has_line_of_sight(0, c.x(), c.z(), 2667, 3309, 1, 1, 1, 1, 0)
            });
            if !seen {
                break;
            }
            if tick % 20 == 19 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            engine.cycle();
        }
        op_loc(&mut client, ClientProt::OpLoc2, BAKERS_STALL);
        let said = run(engine, &mut client, 12, |e, pid| xp(e, pid) > 0);
        let goods: u32 = ["cake", "bread", "chocolate_slice"]
            .iter()
            .map(|o| count(engine, client.pid, o))
            .sum();
        assert_eq!(goods, 1, "one of the stall's goods; the game said {said:?}");
        assert_eq!(xp(engine, client.pid), 160, "16 xp");
        assert_eq!(
            counted(engine, client.pid),
            1600,
            "the xp counter counts it"
        );
        assert!(said.iter().any(|s| s.starts_with("You steal ")), "{said:?}");
        let marketthief = engine
            .harness_player(client.pid)
            .unwrap()
            .vars
            .get(MARKETTHIEF)
            .as_int();
        assert_eq!(marketthief & 2, 2, "marketthief_cake is set: {marketthief}");

        // In the baker's sight, beside him, he stops the theft.
        let mut client = thief(engine, "seenthief", 2669, 3311);
        op_loc(&mut client, ClientProt::OpLoc2, BAKERS_STALL);
        let said = run(engine, &mut client, 12, |_, _| false);
        assert_eq!(
            xp(engine, client.pid),
            0,
            "the baker saw; the game said {said:?}"
        );
        assert!(
            !said.iter().any(|s| s.starts_with("You steal ")),
            "{said:?}"
        );
    });
}

#[test]
fn a_chest_opened_unsearched_springs_its_trap_and_a_searched_one_pays() {
    runner::on_loaded_engine(|engine| {
        // Opened without a search, the trap takes 289's 3 + 12% of the
        // current hitpoints (10 at level 10, so 4) as life points: 40. The
        // chest stays shut.
        let mut client = thief(engine, "trapthief", 2675, 3310);
        // %life_points: varbit 7198, bits 1-15 of varp 1240.
        let life = |e: &Engine, pid: u16| {
            (e.harness_player(pid).unwrap().vars.get(1240).as_int() >> 1) & 0x7FFF
        };
        let before = life(engine, client.pid);
        op_loc(&mut client, ClientProt::OpLoc1, CHEST_10_COINS);
        let said = run(engine, &mut client, 12, |e, pid| life(e, pid) < before);
        assert!(
            said.contains(&"You have activated a trap on the chest.".to_string()),
            "{said:?}"
        );
        assert_eq!(
            before - life(engine, client.pid),
            40,
            "the trap's 4 hitpoints"
        );

        // Searched for traps, it pays 10 coins and 7.8 xp.
        let mut client = thief(engine, "chestthief", 2675, 3310);
        op_loc(&mut client, ClientProt::OpLoc2, CHEST_10_COINS);
        let said = run(engine, &mut client, 20, |e, pid| count(e, pid, "coins") > 0);
        assert_eq!(
            count(engine, client.pid, "coins"),
            10,
            "the game said {said:?}"
        );
        assert_eq!(xp(engine, client.pid), 78, "7.8 xp");
        assert_eq!(counted(engine, client.pid), 780, "the xp counter counts it");
        assert!(
            said.contains(&"You find treasure inside!".to_string()),
            "{said:?}"
        );
    });
}

/// Miscellania's veg stall takes a thief's attempt, as 289's `misc_veg_market`.
#[test]
fn miscellanias_veg_stall_can_be_robbed() {
    runner::on_loaded_engine(|engine| {
        let mut client = thief(engine, "vegthief", 2517, 3861);
        op_loc(&mut client, ClientProt::OpLoc2, MISC_VEG_STALL);
        let said = run(engine, &mut client, 12, |_, _| false);
        assert!(
            said.contains(
                &"You attempt to steal some vegetables from the vegtable stall.".to_string()
            ),
            "{said:?}"
        );
    });
}

/// 289's market guards carry its `market_guard` param, and so do 2011's
/// market paladins (paladin_2256).
#[test]
fn the_market_guards_carry_market_guard() {
    runner::on_loaded_engine(|engine| {
        let param: i32 = std::fs::read_to_string(format!(
            "{}/../../content/647/pack/param.pack",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
        .lines()
        .find_map(|l| l.strip_suffix("=market_guard")?.parse().ok())
        .expect("market_guard is in param.pack");
        for name in [
            "ardougne_guard",
            "paladin",
            "paladin_2256",
            "hero",
            "knight_of_ardougne",
            "knight_of_ardougne2",
            "market_guard",
            "viking_guard2",
            "skraeling",
            "skraeling_1368",
            "skraeling_1389",
            "skraeling_1391",
        ] {
            let id = *engine.cache.npcs.debugnames.get(name).expect(name);
            let value = engine
                .cache
                .npcs
                .get_by_id(id)
                .and_then(|n| n.params.as_deref())
                .and_then(|p| p.get(&param));
            assert!(
                matches!(value, Some(ParamValue::Int(1))),
                "{name}: {value:?}"
            );
        }
    });
}

/// 289's `[walktrigger,stunned]` (player/scripts/movecheck.rs2): a player a
/// failed pickpocket stunned (`%stunned` ahead of the clock, and
/// `walktrigger(stunned)`, as `~fail_pick_pocket` sets them) who clicks to walk
/// is told "You're stunned!" and stays where they are.
#[test]
fn a_stunned_player_cannot_walk() {
    runner::on_loaded_engine(|engine| {
        let (x, z) = (3222, 3218);
        let mut client = thief(engine, "stunnedthief", x, z);
        let walktrigger = engine
            .scripts
            .get_by_name("[walktrigger,stunned]")
            .map(|s| s.id)
            .expect("[walktrigger,stunned] is defined");
        let stunned = 2193; // `stunned`, a server-only varp (varp.pack)
        let until = engine.clock as i32 + 20;
        let player = engine.harness_player_mut(client.pid).unwrap();
        player
            .vars
            .set(stunned, rs_pack::cache::VarValue::Int(until));
        player.walktrigger = Some(walktrigger);
        // A walkable tile a few steps off: a click the pathfinder cannot reach
        // queues no steps, and the walk trigger fires only on a step.
        let (dx, dz) = [(3, 0), (-3, 0), (0, 3), (0, -3), (2, 2), (-2, -2)]
            .into_iter()
            .map(|(a, b): (i32, i32)| ((x as i32 + a) as u16, (z as i32 + b) as u16))
            .find(|&(tx, tz)| !rsmod::is_flagged(tx, tz, 0, 0x100 | 0x200000))
            .expect("a walkable tile near the thief");
        let mut p = ClientPacket::new();
        p.p2_alt1(dx as i32).p1_alt1(0).p2(dz as i32);
        client.send(ClientProt::MoveGameClick, &p.finish());
        let said = run(engine, &mut client, 5, |_, _| false);
        assert!(said.contains(&"You're stunned!".to_string()), "{said:?}");
        let c = engine.harness_player(client.pid).unwrap().pathing.coord;
        assert_eq!((c.x(), c.z()), (x, z), "still where the stun caught them");
    });
}
