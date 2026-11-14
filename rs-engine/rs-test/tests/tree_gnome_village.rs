//! Tree Gnome Village at 647 (`quests/quest_tree`; the gnomes and the spirit trees in
//! `areas/area_gnome`): 289's quest, with the ballista's coordinates as the 2011 wiki has them
//! (Tree Gnome Village (quest), 2011-03-29): the first two trackers' coordinates are remembered
//! and the ballista asks the third alone. Reward: 2 quest points, 11,450 Attack xp, the gnome
//! amulet and the spirit trees.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_pack::types::NpcStat;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, settexts, talk_to};
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const ATTACK: usize = 0;
/// View on the quest list's row (questlist_v4:quests) and the journal's first line (qj1).
const QUEST_LIST: i32 = 190 << 16 | 18;
const TREE_GNOME_VILLAGE: i32 = 110;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;

/// `^tree_started` .. `^tree_complete`.
const STARTED: i32 = 1;
const SPOKEN_MONTAI: i32 = 2;
const GIVEN_LOGS: i32 = 3;
const FINDING_TRACKERS: i32 = 4;
const BALLISTA_FIRED: i32 = 5;
const RETRIEVED_ORB: i32 = 6;
const RETURNED_FIRST_ORB: i32 = 7;
const DEFEATED_WARLORD: i32 = 8;
const COMPLETE: i32 = 9;

/// maps/l39_50.jm2 (mapsquare base 2496, 3200): the battlefield.
const BALLISTA: (u16, u16) = (2508, 3210);
const STRONGHOLD_DOOR: (u16, u16) = (2502, 3250);
const CRUMBLED_WALL: (u16, u16) = (2509, 3253);
const CHEST: (u16, u16) = (2506, 3259);
/// `^elkoy_maze_coord`, inside the maze by the village, and `^elkoy_entrance_coord`.
const MAZE_INSIDE: (u16, u16) = (2515, 3159);
const MAZE_ENTRANCE: (u16, u16) = (2504, 3192);
/// maps/l39_49.jm2: the village's spirit tree, and `^khazard_tree`.
const VILLAGE_TREE: (u16, u16) = (2543, 3168);
const KHAZARD_TREE: (u16, u16) = (2555, 3259);

fn var(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let v = id(engine, "varp", name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(v)
        .as_int()
}

fn set_var(engine: &mut Engine, client: &HarnessClient, name: &str, value: i32) {
    let v = id(engine, "varp", name);
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v, VarValue::Int(value));
}

fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "treequest", stage);
    client
}

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
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

/// `OPLOC1`: `p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`, and what it shows.
fn op_loc1(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
    converse(engine, client, &[], 12)
}

/// `OPLOC2`: `p2_alt1(loc) p1_alt2(ctrl) p2_alt2(z) p2_alt3(x)`, taking `choices`.
fn op_loc2(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
    choices: &[&str],
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt1(loc as i32)
        .p1_alt2(0)
        .p2_alt2(z as i32)
        .p2_alt3(x as i32);
    client.send(ClientProt::OpLoc2, &p.finish());
    let mut t = converse(engine, client, choices, 60);
    // The spear's flight closes the box for a tick or two before what it hits is told.
    let more = converse(engine, client, &[], 30);
    t.texts.extend(more.texts);
    t.messages.extend(more.messages);
    t
}

fn level_of(engine: &Engine, client: &HarnessClient) -> u8 {
    engine.harness_player(client.pid).unwrap().pathing.coord.y()
}

/// A walkable tile within `reach` of an npc that wanders or stands behind bars.
fn stand_near(engine: &mut Engine, client: &mut HarnessClient, npc: &str, reach: i16) -> u16 {
    let (nid, x, z, level) = npcs(engine, npc)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{npc} is spawned"));
    let at = (1..=reach)
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
        .find(|&(ax, az)| walkable(ax, level, az))
        .unwrap_or_else(|| panic!("a tile near {npc}"));
    tele(engine, client, at, level);
    nid
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

/// The journal's text once View is clicked on the quest's row, its colour tags taken off.
fn journal(engine: &mut Engine, client: &mut HarnessClient) -> String {
    let mut p = ClientPacket::new();
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(TREE_GNOME_VILLAGE);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut texts = Vec::new();
    for _ in 0..3 {
        engine.cycle();
        texts.extend(settexts(&client.packets()));
    }
    engine.harness_close_modal(client.pid);
    let mut plain = String::new();
    for line in 0..40 {
        let text = texts
            .iter()
            .find(|(c, _)| *c == JOURNAL_LINE_1 + line)
            .map_or("", |(_, t)| t.as_str());
        let mut tag = false;
        for ch in text.chars() {
            match ch {
                '<' => tag = true,
                '>' => tag = false,
                _ if !tag => plain.push(ch),
                _ => {}
            }
        }
        plain.push(' ');
    }
    plain
}

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
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

/// King Bolren asks for the orb back and his assistant leads the player out of the maze.
#[test]
fn king_bolren_asks_for_the_orb() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tg_start", 0);
        let lines = journal(engine, &mut client);
        assert!(!lines.trim().is_empty(), "the journal has the quest");

        let mut t = talk_near(
            engine,
            &mut client,
            "king_bolren",
            &["Can I help at all?", "I would be glad to help."],
        );
        let more = converse(engine, &mut client, &[], 60);
        t.texts.extend(more.texts);
        says(
            &t,
            &[
                "My name's Bolren, I'm the king of the tree gnomes.",
                "Khazard troops seized the orb.",
                "My assistant shall guide you out.",
                "We're out of the maze now.",
            ],
        );
        assert_eq!(var(engine, &client, "treequest"), STARTED);
        assert_eq!(
            coord(engine, &client),
            (MAZE_ENTRANCE.0, MAZE_ENTRANCE.1 - 1),
            "led out to the maze's entrance"
        );
        no_errors(engine);
    });
}

/// Commander Montai wants six logs for the battlements, then the trackers found.
#[test]
fn commander_montai_wants_logs_then_trackers() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tg_montai", STARTED);
        let t = talk_near(
            engine,
            &mut client,
            "commander_montai",
            &["Ok, I'll gather some wood."],
        );
        says(&t, &["We desperately need wood to make more battlements"]);
        assert_eq!(var(engine, &client, "treequest"), SPOKEN_MONTAI);

        let t = talk_near(engine, &mut client, "commander_montai", &[]);
        says(&t, &["We need six loads of normal logs."]);
        assert_eq!(var(engine, &client, "treequest"), SPOKEN_MONTAI);

        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let logs = id(engine, "obj", "logs");
        for slot in 0..6 {
            engine
                .harness_player_mut(client.pid)
                .unwrap()
                .invs
                .get_mut(&inv)
                .unwrap()
                .set(slot, logs, 1);
        }
        let t = talk_near(engine, &mut client, "commander_montai", &[]);
        says(&t, &["I have some here."]);
        assert_eq!(var(engine, &client, "treequest"), GIVEN_LOGS);
        assert_eq!(held(engine, &client, "logs"), 0, "the six logs are given");

        talk_near(
            engine,
            &mut client,
            "commander_montai",
            &["I'll try my best."],
        );
        assert_eq!(var(engine, &client, "treequest"), FINDING_TRACKERS);
        no_errors(engine);
    });
}

/// The trackers, as the 2011 wiki has them: the first two tell their coordinate without the
/// player hearing a number to enter, and the third gives 289's riddle for 3.
#[test]
fn the_trackers_give_their_coordinates() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tg_trackers", FINDING_TRACKERS);
        let t = talk_near(engine, &mut client, "tracker1", &[]);
        says(
            &t,
            &[
                "Do you know the coordinates of the Khazard stronghold?",
                "The gnome tells you the height coordinate.",
            ],
        );
        assert!(!t.said("The height coordinate is 4."), "no number to enter");

        let t = talk_near(engine, &mut client, "tracker2", &[]);
        says(
            &t,
            &[
                "They caught me spying on the stronghold.",
                "The gnome tells you the y coordinate.",
            ],
        );
        assert!(!t.said("The y coordinate is 5."), "no number to enter");

        let t = talk_near(engine, &mut client, "tracker3", &[]);
        says(
            &t,
            &[
                "He's clearly lost the plot.",
                "More than me, less than our feet.",
                "More than we, and Khazard's men are beat.",
            ],
        );
        assert_eq!(var(engine, &client, "treequest"), FINDING_TRACKERS);
        no_errors(engine);
    });
}

/// The ballista asks the third coordinate alone (2011 wiki): a wrong one misses, 289's 3 hits
/// the stronghold.
#[test]
fn the_ballista_takes_the_third_coordinate() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tg_ballista", GIVEN_LOGS);
        let beside = [
            (1i16, 0i16),
            (-1, 0),
            (0, 1),
            (0, -1),
            (2, 0),
            (0, 2),
            (-2, 0),
            (0, -2),
        ]
        .into_iter()
        .map(|(dx, dz)| {
            (
                BALLISTA.0.wrapping_add_signed(dx),
                BALLISTA.1.wrapping_add_signed(dz),
            )
        })
        .find(|&(x, z)| walkable(x, 0, z))
        .expect("a tile by the ballista");
        tele(engine, &mut client, beside, 0);
        let t = op_loc2(engine, &mut client, "catabow", BALLISTA, &[]);
        says(&t, &["The ballista is damaged."]);

        set_var(engine, &client, "treequest", FINDING_TRACKERS);
        let t = op_loc2(engine, &mut client, "catabow", BALLISTA, &["0001"]);
        says(
            &t,
            &[
                "To fire the ballista you must first set the coordinates.",
                "Enter the x-coordinate of the stronghold",
                "You fire the ballista.",
                "You've missed the target.",
            ],
        );
        assert!(
            !t.said("Enter the height-coordinate of the stronghold")
                && !t.said("Enter the y-coordinate of the stronghold"),
            "the third coordinate alone is asked: {:?}",
            t.texts
        );
        assert_eq!(var(engine, &client, "treequest"), FINDING_TRACKERS);

        let t = op_loc2(engine, &mut client, "catabow", BALLISTA, &["0003"]);
        says(&t, &["front entrance is reduced to rubble."]);
        assert_eq!(var(engine, &client, "treequest"), BALLISTA_FIRED);

        let t = op_loc2(engine, &mut client, "catabow", BALLISTA, &[]);
        says(&t, &["The Khazard stronghold has already been breached."]);
        no_errors(engine);
    });
}

/// The stronghold: its door stays shut, the wall is climbed once the ballista has breached it,
/// and the chest upstairs holds the orb.
#[test]
fn the_stronghold_gives_up_the_orb() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tg_stronghold", FINDING_TRACKERS);
        {
            let player = engine.harness_player_mut(client.pid).unwrap();
            for skill in [0, 1, 2, 3] {
                player.stats.levels[skill] = 99;
                player.stats.base_levels[skill] = 99;
            }
            let bit0 = player.vars.get(1240).as_int() & 1;
            player.vars.set(1240, VarValue::Int((990 << 1) | bit0));
        }
        // The door, from outside (south of it).
        tele(
            engine,
            &mut client,
            (STRONGHOLD_DOOR.0, STRONGHOLD_DOOR.1 - 1),
            0,
        );
        let t = op_loc1(
            engine,
            &mut client,
            "khazard_stronghold_door",
            STRONGHOLD_DOOR,
        );
        says(&t, &["The door seems to be locked from the inside."]);

        // The wall, before and after the ballista.
        let south = (CRUMBLED_WALL.0, CRUMBLED_WALL.1 - 1);
        tele(engine, &mut client, south, 0);
        let t = op_loc1(engine, &mut client, "khazzacklowwall", CRUMBLED_WALL);
        tells(&t, "The wall is damaged, but not enough to climb.");
        assert_eq!(coord(engine, &client), south);

        set_var(engine, &client, "treequest", BALLISTA_FIRED);
        let t = op_loc1(engine, &mut client, "khazzacklowwall", CRUMBLED_WALL);
        says(&t, &["The wall has been reduced to rubble."]);
        settle(engine, &mut client, 12);
        let (x, z) = coord(engine, &client);
        assert!(z > CRUMBLED_WALL.1, "over the wall: ({x}, {z})");

        // The chest, upstairs.
        let by_chest = [(0i16, -1i16), (1, 0), (-1, 0), (0, 1)]
            .into_iter()
            .map(|(dx, dz)| {
                (
                    CHEST.0.wrapping_add_signed(dx),
                    CHEST.1.wrapping_add_signed(dz),
                )
            })
            .find(|&(x, z)| walkable(x, 1, z))
            .expect("a tile by the chest");
        tele(engine, &mut client, by_chest, 1);
        op_loc1(engine, &mut client, "chestclosed_khazard", CHEST);
        let t = op_loc1(engine, &mut client, "chestopen_khazard", CHEST);
        says(
            &t,
            &["Inside you find the gnomes' stolen orb of protection."],
        );
        assert_eq!(held(engine, &client, "orb_of_protection"), 1);
        assert_eq!(var(engine, &client, "treequest"), RETRIEVED_ORB);
        assert_eq!(level_of(engine, &client), 1);
        let t = op_loc1(engine, &mut client, "chestopen_khazard", CHEST);
        tells(&t, "You search the chest, but find nothing.");
        no_errors(engine);
    });
}

/// Elkoy leads the orb's bearer through the maze, and the king sends them after the warlord.
#[test]
fn elkoy_leads_the_orb_to_the_king() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tg_elkoy", RETRIEVED_ORB);
        give(engine, &client, 0, "orb_of_protection");
        let mut t = talk_near(engine, &mut client, "elkoy", &["Yes please."]);
        let more = converse(engine, &mut client, &[], 40);
        t.texts.extend(more.texts);
        says(
            &t,
            &[
                "You're back! And the orb?",
                "Ok then, follow me.",
                "Here we are. Take the orb to King Bolren",
            ],
        );
        assert_eq!(coord(engine, &client), MAZE_INSIDE, "led into the village");

        let t = talk_near(
            engine,
            &mut client,
            "king_bolren",
            &["I will find the warlord and bring back the orbs."],
        );
        assert_eq!(
            var(engine, &client, "treequest"),
            RETURNED_FIRST_ORB,
            "{:?}",
            t.texts
        );
        assert_eq!(
            held(engine, &client, "orb_of_protection"),
            0,
            "the orb is handed over"
        );
        no_errors(engine);
    });
}

/// The Khazard warlord falls and gives up the other two orbs.
#[test]
fn the_warlord_gives_up_the_orbs() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tg_warlord", RETURNED_FIRST_ORB);
        {
            let player = engine.harness_player_mut(client.pid).unwrap();
            for skill in [0, 1, 2, 3] {
                player.stats.levels[skill] = 99;
                player.stats.base_levels[skill] = 99;
            }
            let bit0 = player.vars.get(1240).as_int() & 1;
            player.vars.set(1240, VarValue::Int((990 << 1) | bit0));
        }
        let nid = stand_near(engine, &mut client, "khazard_warlord", 2);
        let mut texts: Vec<String> = Vec::new();
        for tick in 0..600 {
            if let Some(n) = engine.get_npc_mut(nid) {
                // 1 life point, and defence 1 so a swing lands: the case is his death's
                // trigger, not the fight's dice.
                let stats = &mut n.npc.stats.levels;
                stats[NpcStat::Hitpoints as usize] = stats[NpcStat::Hitpoints as usize].min(1);
                stats[NpcStat::Defence as usize] = 1;
            }
            if tick % 8 == 0 {
                // He wanders and gives chase: stand beside him again before each swing.
                if let Some(n) = engine.get_npc(nid) {
                    let (x, z) = (n.npc.pathing.coord.x(), n.npc.pathing.coord.z());
                    let (px, pz) = coord(engine, &client);
                    if px.abs_diff(x) > 1 || pz.abs_diff(z) > 1 {
                        if let Some(at) = [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
                            .into_iter()
                            .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
                            .find(|&(ax, az)| walkable(ax, 0, az))
                        {
                            engine.harness_telejump(client.pid, at.0, 0, at.1);
                        }
                    }
                }
                let mut p = ClientPacket::new();
                p.p1_alt2(0).p2_alt3(nid as i32);
                client.send(ClientProt::OpNpc2, &p.finish());
            }
            engine.cycle();
            texts.extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
            if held(engine, &client, "orbs_of_protection") > 0 {
                break;
            }
        }
        assert_eq!(held(engine, &client, "orbs_of_protection"), 1, "{texts:?}");
        assert_eq!(var(engine, &client, "treequest"), DEFEATED_WARLORD);
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 4);
        no_errors(engine);
    });
}

/// King Bolren takes the orbs: 2 quest points, 11,450 Attack xp and the gnome amulet (2011
/// wiki), and the village's spirit tree carries the player from then on.
#[test]
fn the_orbs_return_and_the_spirit_tree_wakes() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tg_reward", DEFEATED_WARLORD);
        // The tree, before the quest is done.
        let by_tree = (1i16..=3)
            .flat_map(|r| [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r)])
            .map(|(dx, dz)| {
                (
                    VILLAGE_TREE.0.wrapping_add_signed(dx),
                    VILLAGE_TREE.1.wrapping_add_signed(dz),
                )
            })
            .find(|&(x, z)| walkable(x, 0, z))
            .expect("a tile by the spirit tree");
        tele(engine, &mut client, by_tree, 0);
        let t = op_loc1(engine, &mut client, "ent", VILLAGE_TREE);
        says(&t, &["The tree doesn't feel like talking."]);

        give(engine, &client, 0, "orbs_of_protection");
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[ATTACK];
        let qp = var(engine, &client, "qp");
        let mut t = talk_near(engine, &mut client, "king_bolren", &[]);
        for _ in 0..4 {
            let more = converse(engine, &mut client, &[], 60);
            t.texts.extend(more.texts);
            t.messages.extend(more.messages);
            if var(engine, &client, "treequest") == COMPLETE {
                break;
            }
        }
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            t.texts
                .extend(settexts(&packets).into_iter().map(|(_, t)| t));
        }
        says(
            &t,
            &[
                "The orbs of protection come to rest gently in the branches of the ancient spirit tree.",
                "You have completed the Tree Gnome Village Quest!",
                "11,450 Attack XP",
                "Gnome amulet",
            ],
        );
        assert_eq!(var(engine, &client, "treequest"), COMPLETE);
        assert_eq!(held(engine, &client, "orbs_of_protection"), 0);
        assert_eq!(held(engine, &client, "gnome_amulet"), 1);
        let gained = engine.harness_player(client.pid).unwrap().stats.xp[ATTACK] - xp;
        assert_eq!(gained, 114_500, "11,450 Attack xp (tenths)");
        assert_eq!(var(engine, &client, "qp") - qp, 2, "2 quest points");
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        let lines = journal(engine, &mut client);
        assert!(lines.contains("QUEST COMPLETE!"), "{lines}");

        // The tree now carries the player: to the battlefield's.
        tele(engine, &mut client, by_tree, 0);
        let loc = id(engine, "loc", "ent");
        let mut p = ClientPacket::new();
        p.p2_alt1(VILLAGE_TREE.1 as i32)
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(VILLAGE_TREE.0 as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let t = converse(
            engine,
            &mut client,
            &["Where can I go?", "Battlefield of Khazard."],
            120,
        );
        says(
            &t,
            &[
                "The tree talks in an old tired voice...",
                "You place your hands on the dry tough bark of the spirit tree",
            ],
        );
        settle(engine, &mut client, 4);
        assert_eq!(
            coord(engine, &client),
            KHAZARD_TREE,
            "carried to the battlefield"
        );
        no_errors(engine);
    });
}
