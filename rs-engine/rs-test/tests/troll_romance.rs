//! Troll Romance at 647 (`quests/quest_troll_love`): 289's quest. Cache 786's map has Trollweiss
//! and its tunnels on 289's tiles (two of the tunnels' locs are renamed); Arrg's fight is in
//! 2011's arena, east of 289's. The rewards are the 2011 wiki's and Sal's Realm's as 289's.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, settexts, talk_to};
use rs_ver::TypeId;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const STRENGTH: usize = 2;
const AGILITY: usize = 16;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `^troll_love_started` .. `^troll_love_complete`.
const STARTED: i32 = 5;
const AGA_WANTS_TROLLWEISS: i32 = 10;
const LEARNT_ABOUT_TROLLWEISS: i32 = 15;
const WAXED_SLED: i32 = 25;
const PICKED_TROLLWEISS: i32 = 30;
const DISPOSE_OF_ARRG: i32 = 35;
const DEFEATED_ARRG: i32 = 40;
const COMPLETE: i32 = 45;

/// Cache 786's map (289's tiles).
const BARRIER: (u16, u16) = (2772, 3835);
const FLOWERS: (u16, u16) = (2775, 3782);
/// Where each tunnel puts the player, and where the ice wolves stand.
const TUNNEL_ENDS: [(u16, u16); 8] = [
    (2803, 10187),
    (2822, 3744),
    (2778, 3869),
    (2772, 10231),
    (2799, 10134),
    (2797, 3719),
    (2730, 3713),
    (2773, 10162),
];
const WOLVES: [(u16, u16); 6] = [
    (2761, 3863),
    (2769, 3854),
    (2769, 3861),
    (2790, 3852),
    (2793, 3847),
    (2801, 3848),
];
/// 2011's troll arena, where Arrg's fight is (`^troll_arena_dad`).
const ARENA: (u16, u16) = (2921, 3613);

fn var(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let v = id(engine, "varp", name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(v.g2())
        .as_int()
}

fn set_var(engine: &mut Engine, client: &HarnessClient, name: &str, value: i32) {
    let v = id(engine, "varp", name);
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v.g2(), VarValue::Int(value));
}

fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "troll_love", stage);
    client
}

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

fn tele(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16), level: u8) {
    assert!(
        walkable(x, level, z),
        "({x}, {z}) on level {level} is walkable"
    );
    engine.harness_telejump(client.pid, x, level, z);
    settle(engine, client, 3);
}

/// Every spawned npc of that name: its nid, tile and level.
fn npcs(engine: &Engine, name: &str) -> Vec<(u16, u16, u16, u8)> {
    let want = id(engine, "npc", name);
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == want)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.z(), c.y())
        })
        .collect()
}

/// Talks to an npc that wanders or stands in a cell: from each walkable tile near it in turn,
/// until one reaches it.
fn talk_near(
    engine: &mut Engine,
    client: &mut HarnessClient,
    npc: &str,
    choices: &[&str],
) -> Conversation {
    let mut last = Conversation::default();
    for attempt in 0..12usize {
        let (nid, x, z, level) = npcs(engine, npc)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("{npc} is spawned"));
        let tiles: Vec<(u16, u16)> = (1i16..=2)
            .flat_map(|r| {
                [
                    (r, 0),
                    (-r, 0),
                    (0, r),
                    (0, -r),
                    (r, r),
                    (-r, -r),
                    (r, -r),
                    (-r, r),
                ]
            })
            .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
            .filter(|&(ax, az)| walkable(ax, level, az))
            .collect();
        assert!(!tiles.is_empty(), "a tile near {npc}");
        let at = tiles[attempt % tiles.len()];
        engine.harness_telejump(client.pid, at.0, level, at.1);
        settle(engine, client, 2);
        talk_to(client, nid);
        // A short look first: a talk that never starts must not idle the client out. The
        // first box's lines are kept for what the talk said.
        let mut first: Vec<String> = Vec::new();
        let mut chat: Vec<String> = Vec::new();
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            first.extend(settexts(&packets).into_iter().map(|(_, t)| t));
            chat.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, m)| m),
            );
            if !first.is_empty() {
                break;
            }
        }
        last = Conversation::default();
        last.messages = chat;
        if !first.is_empty() {
            let more = converse(engine, client, choices, 300);
            assert!(more.unanswered.is_none(), "{:?}", more.unanswered);
            first.extend(more.texts);
            last.texts = first;
            last.messages.extend(more.messages);
            return last;
        }
    }
    panic!("{npc} was never reached: {:?}", last.messages);
}

fn id(engine: &Engine, kind: &str, name: &str) -> TypeId {
    let map = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "loc" => &engine.cache.locs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
        _ => unreachable!(),
    };
    *map.get(name).unwrap_or_else(|| panic!("no {kind} {name}"))
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

fn held(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(o) as i32
}

fn settle(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) {
    for _ in 0..ticks {
        engine.cycle();
    }
    client.packets();
}

fn says(t: &Conversation, lines: &[&str]) {
    for line in lines {
        assert!(
            t.said(line),
            "289 has {line:?}; {:?} (chat {:?})",
            t.texts,
            t.messages
        );
    }
}

fn tells(t: &Conversation, line: &str) {
    assert!(
        t.messages.iter().any(|m| m == line),
        "289 has {line:?}; chat {:?} (texts {:?})",
        t.messages,
        t.texts
    );
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

/// `OPLOC1` (`p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`) or `OPLOC2` (`p2_alt1(loc)
/// p1_alt2(ctrl) p2_alt2(z) p2_alt3(x)`) on a loc, taking `choices`.
fn op_loc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    op: u8,
    loc: &str,
    (x, z): (u16, u16),
    choices: &[&str],
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    if op == 1 {
        p.p2_alt1(z as i32)
            .p2_alt1(loc.int())
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
    } else {
        p.p2_alt1(loc.int())
            .p1_alt2(0)
            .p2_alt2(z as i32)
            .p2_alt3(x as i32);
        client.send(ClientProt::OpLoc2, &p.finish());
    }
    shown(engine, client, choices)
}

/// What an op shows: a short look first (an op that only writes to the chat must not idle the
/// client out), then the boxes, taking `choices`.
fn shown(engine: &mut Engine, client: &mut HarnessClient, choices: &[&str]) -> Conversation {
    let mut first: Vec<String> = Vec::new();
    let mut chat: Vec<String> = Vec::new();
    for _ in 0..12 {
        engine.cycle();
        let packets = client.packets();
        first.extend(settexts(&packets).into_iter().map(|(_, t)| t));
        chat.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
        if !first.is_empty() {
            break;
        }
    }
    let mut t = Conversation {
        messages: chat,
        ..Default::default()
    };
    if !first.is_empty() {
        let more = converse(engine, client, choices, 400);
        assert!(more.unanswered.is_none(), "{:?}", more.unanswered);
        first.extend(more.texts);
        t.messages.extend(more.messages);
    }
    t.texts = first;
    t
}

/// What the chat is told over `ticks` (an op that opens no box).
fn watch(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> Conversation {
    let mut t = Conversation::default();
    for _ in 0..ticks {
        engine.cycle();
        let packets = client.packets();
        t.texts
            .extend(settexts(&packets).into_iter().map(|(_, text)| text));
        t.messages.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
    }
    t
}

/// `OPLOC1` on a loc, and what the chat is told over `ticks`.
fn search(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
    ticks: usize,
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc.int())
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
    watch(engine, client, ticks)
}

/// Level 99 in every skill (its xp, in tenths) and full life: no level-up box follows a reward,
/// and a fall is survived.
fn seasoned(engine: &mut Engine, client: &HarnessClient) {
    let player = engine.harness_player_mut(client.pid).unwrap();
    for skill in 0..player.stats.levels.len() {
        player.stats.levels[skill] = 99;
        player.stats.base_levels[skill] = 99;
        player.stats.xp[skill] = 130_344_310;
    }
    let bit0 = player.vars.get(LIFE).as_int() & 1;
    player.vars.set(LIFE, VarValue::Int((990 << 1) | bit0));
}

fn xp(engine: &Engine, client: &HarnessClient, skill: usize) -> i32 {
    engine.harness_player(client.pid).unwrap().stats.xp[skill]
}

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    var(engine, client, "troll_love")
}

/// With Troll Stronghold done, Ug tells of Aga and the player offers to help.
#[test]
fn ug_is_offered_help() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tr_start", 0);
        seasoned(engine, &client);
        let t = talk_near(engine, &mut client, "trollromance_ug", &[]);
        says(&t, &["He doesn't look like he feels like talking."]);
        assert_eq!(stage(engine, &client), 0);

        set_var(engine, &client, "troll_quest", 50);
        talk_near(
            engine,
            &mut client,
            "trollromance_ug",
            &[
                "Awww, you poor troll. What seems to be the problem?",
                "Don't worry now, I'll see what I can do.",
            ],
        );
        assert_eq!(stage(engine, &client), STARTED);
        no_errors(engine);
    });
}

/// Tenzing tells where Trollweiss grows and how to get there.
#[test]
fn tenzing_tells_of_trollweiss() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tr_tenzing", AGA_WANTS_TROLLWEISS);
        seasoned(engine, &client);
        set_var(engine, &client, "death_equiproom", 80);
        let t = talk_near(
            engine,
            &mut client,
            "death_sherpa",
            &[
                "Do you know where I can find Trollweiss?",
                "What would I need to make such a sled?",
                "Nothing, thanks!",
            ],
        );
        assert_eq!(
            stage(engine, &client),
            LEARNT_ABOUT_TROLLWEISS,
            "{:?}",
            t.texts
        );
        no_errors(engine);
    });
}

/// The sled run wants 28 Agility and the waxed sled worn.
#[test]
fn the_sled_run_wants_the_waxed_sled() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tr_sled", WAXED_SLED);
        let mut told = None;
        for dz in 1..=2u16 {
            for dx in [0i16, -1, 1, 2] {
                let at = (BARRIER.0.wrapping_add_signed(dx), BARRIER.1 + dz);
                if !walkable(at.0, 0, at.1) {
                    continue;
                }
                tele(engine, &mut client, at, 0);
                let t = search(
                    engine,
                    &mut client,
                    "trollromance_piste_walk_barrier_down",
                    BARRIER,
                    4,
                );
                if t.messages
                    .iter()
                    .any(|m| m == "You need an agility level of at least 28!")
                {
                    told = Some(at);
                    break;
                }
            }
            if told.is_some() {
                break;
            }
        }
        let at = told.expect("a fresh player is told of the 28 Agility");
        seasoned(engine, &client);
        tele(engine, &mut client, at, 0);
        let t = search(
            engine,
            &mut client,
            "trollromance_piste_walk_barrier_down",
            BARRIER,
            4,
        );
        tells(&t, "You need to have a waxed sled equipped!");
        no_errors(engine);
    });
}

/// Trollweiss is picked on its patch.
#[test]
fn trollweiss_is_picked() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tr_flower", WAXED_SLED);
        seasoned(engine, &client);
        let mut picked = false;
        for (dx, dz) in [(0i16, -1i16), (0, 1), (-1, 0), (1, 0)] {
            let at = (
                FLOWERS.0.wrapping_add_signed(dx),
                FLOWERS.1.wrapping_add_signed(dz),
            );
            if !walkable(at.0, 0, at.1) {
                continue;
            }
            tele(engine, &mut client, at, 0);
            op_loc(
                engine,
                &mut client,
                2,
                "trollromance_rareflowers",
                FLOWERS,
                &[],
            );
            if held(engine, &client, "trollromance_rare_flower") > 0 {
                picked = true;
                break;
            }
        }
        assert!(picked, "a Trollweiss is picked");
        assert_eq!(stage(engine, &client), PICKED_TROLLWEISS);
        no_errors(engine);
    });
}

/// Challenged, Arrg fights in 2011's arena; with him dead the quest moves on.
#[test]
fn arrg_is_fought_in_2011s_arena() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tr_arrg", DISPOSE_OF_ARRG);
        seasoned(engine, &client);
        talk_near(
            engine,
            &mut client,
            "trollromance_arrg",
            &["I am here to kill you!"],
        );
        let at = coord(engine, &client);
        assert!(
            at.0.abs_diff(ARENA.0) <= 3 && at.1.abs_diff(ARENA.1) <= 3,
            "in the arena: {at:?}"
        );
        assert!(walkable(at.0, 0, at.1));
        let (nid, ..) = npcs(engine, "trollromance_arrg_attackable")
            .into_iter()
            .next()
            .expect("Arrg is added in the arena");
        let hp = rs_pack::types::NpcStat::Hitpoints as usize;
        for tick in 0..300 {
            if stage(engine, &client) == DEFEATED_ARRG {
                break;
            }
            let Some(n) = engine.get_npc_mut(nid) else {
                break;
            };
            let c = n.npc.pathing.coord;
            let stats = &mut n.npc.stats.levels;
            stats[hp] = stats[hp].min(1);
            stats[rs_pack::types::NpcStat::Defence as usize] = 1;
            if tick % 6 == 0 {
                let near = [(-1i16, 0i16), (0, -1), (2, 0), (0, 2)]
                    .into_iter()
                    .map(|(dx, dz)| (c.x().wrapping_add_signed(dx), c.z().wrapping_add_signed(dz)))
                    .find(|&(ax, az)| walkable(ax, 0, az));
                if let Some(at) = near {
                    engine.harness_telejump(client.pid, at.0, 0, at.1);
                }
                let mut p = ClientPacket::new();
                p.p1_alt2(0).p2_alt3(nid as i32);
                client.send(ClientProt::OpNpc2, &p.finish());
            }
            engine.cycle();
            client.packets();
        }
        settle(engine, &mut client, 8);
        assert_eq!(stage(engine, &client), DEFEATED_ARRG, "Arrg is killed");
        let _ = engine.script_errors.take();
    });
}

/// Ug's reward: 2 quest points, 8,000 Agility and 4,000 Strength experience, an uncut diamond,
/// two uncut rubies and four uncut emeralds (2011 wiki and Sal's Realm, as 289).
#[test]
fn ug_rewards_the_matchmaker() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tr_done", DEFEATED_ARRG);
        seasoned(engine, &client);
        set_var(engine, &client, "troll_quest", 50);
        give(engine, &client, 0, "trollromance_rare_flower");
        let (agility, strength) = (xp(engine, &client, AGILITY), xp(engine, &client, STRENGTH));
        let t = talk_near(engine, &mut client, "trollromance_ug", &[]);
        assert!(
            !t.messages.iter().any(|m| m.starts_with("script error")),
            "{:?}",
            t.messages
        );
        settle(engine, &mut client, 8);
        assert_eq!(stage(engine, &client), COMPLETE, "{:?}", t.texts);
        assert_eq!(
            xp(engine, &client, AGILITY) - agility,
            80_000,
            "8,000 Agility xp"
        );
        assert_eq!(
            xp(engine, &client, STRENGTH) - strength,
            40_000,
            "4,000 Strength xp"
        );
        assert_eq!(held(engine, &client, "uncut_diamond"), 1);
        assert_eq!(held(engine, &client, "uncut_ruby"), 2);
        assert_eq!(held(engine, &client, "uncut_emerald"), 4);
        no_errors(engine);
    });
}

/// The tunnels' ends and the ice wolves' tiles (289's) are open ground in cache 786's map.
#[test]
fn the_tunnels_and_the_wolves_stand_on_open_ground() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for (x, z) in TUNNEL_ENDS {
            assert!(walkable(x, 0, z), "the tunnel's end ({x},{z}) is open");
        }
        for (x, z) in WOLVES {
            assert!(walkable(x, 0, z), "the wolf's tile ({x},{z}) is open");
        }
        assert_eq!(
            npcs(engine, "trollromance_ice_wolf_1").len(),
            6,
            "the six ice wolves are spawned"
        );
    });
}
