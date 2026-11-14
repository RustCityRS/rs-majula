//! Holy Grail at 647 (`quests/quest_grail`; the High Priest in `areas/area_entrana`, Galahad in
//! `areas/area_seers`, King Arthur and Merlin in `areas/area_camelot`): 289's quest. 2011 keeps
//! Merlin in his workshop for good (cache 786: a multinpc on Merlin's Crystal's varp) and wants
//! the magic gold feather carried to open Percival's sack (2011 wiki, Holy Grail, 2011-04-28).
//! Reward: 2 quest points, 11,000 Prayer xp and 15,300 Defence xp.

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

/// View on the quest list's row (questlist_v4:quests) and the journal's first line (qj1).
const QUEST_LIST: i32 = 190 << 16 | 18;
const HOLY_GRAIL: i32 = 61;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;
const BACKPACK: i32 = 149 << 16;
const RHAND: u16 = 3;
const DEFENCE: usize = 1;
const PRAYER: usize = 5;

/// `^arthur_complete`, and `^grail_started` .. `^grail_complete`.
const KNIGHTED: i32 = 7;
const STARTED: i32 = 2;
const SPOKEN_MERLIN: i32 = 3;
const SPOKEN_CRONE: i32 = 4;
const FAILED_TITAN: i32 = 7;
const FINDING_PERCIVAL: i32 = 8;
const GIVEN_WHISTLE: i32 = 9;
const COMPLETE: i32 = 10;

/// Cache 786's map and 289's tiles: the whistle room's door at the top of Draynor Manor and
/// where the whistles appear; under the tower on Karamja; where the whistle lands in the
/// realm, before and after Percival; the bell, and inside the castle; Percival's sacks in
/// Goblin Village; the Grail.
const WHISTLE_DOOR: (u16, u16) = (3106, 3361);
const WHISTLES: (u16, u16) = (3107, 3359);
const UNDER_THE_TOWER: (u16, u16) = (2741, 3235);
const REALM_BLIGHTED: (u16, u16) = (2806, 4715);
const REALM_RESTORED: (u16, u16) = (2678, 4715);
const BELL: (u16, u16) = (2762, 4694);
const INSIDE_THE_CASTLE: (u16, u16) = (2761, 4692);
const SACKS: (u16, u16) = (2962, 3505);
const GRAIL: (u16, u16) = (2649, 4684);

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
    set_var(engine, &client, "arthur", KNIGHTED);
    set_var(engine, &client, "grail", stage);
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
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(HOLY_GRAIL);
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

fn level_of(engine: &Engine, client: &HarnessClient) -> u8 {
    engine.harness_player(client.pid).unwrap().pathing.coord.y()
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

/// Stands the player on a walkable tile within three of a loc's tile, nearest first.
fn stand_by(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16), level: u8) {
    let mut tiles: Vec<(i16, i16)> = (-3i16..=3)
        .flat_map(|dx| (-3i16..=3).map(move |dz| (dx, dz)))
        .filter(|&(dx, dz)| (dx, dz) != (0, 0))
        .collect();
    tiles.sort_by_key(|&(dx, dz)| dx.abs() + dz.abs());
    let at = tiles
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(ax, az)| walkable(ax, level, az))
        .unwrap_or_else(|| panic!("a tile by ({x}, {z})"));
    tele(engine, client, at, level);
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
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
    } else {
        p.p2_alt1(loc as i32)
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

fn slot_of(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let player = engine.harness_player(client.pid).unwrap();
    let inv = player.invs.get(&inv).unwrap();
    (0..28)
        .find(|&s| inv.has_at(s, o))
        .unwrap_or_else(|| panic!("{name} is held")) as i32
}

/// Wields an obj: straight into the worn inventory's right hand.
fn wield(engine: &mut Engine, client: &HarnessClient, name: &str) {
    let o = id(engine, "obj", name);
    let worn = *engine.cache.invs.debugnames.get("worn").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&worn)
        .unwrap()
        .set(RHAND, o, 1);
}

/// The backpack's first button op on an obj (`[if_button1,inventory:inv]` runs `opheld1`):
/// Blow, Ring, Blow-on.
fn use_item(engine: &mut Engine, client: &mut HarnessClient, obj: &str) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let obj = id(engine, "obj", obj);
    let mut p = ClientPacket::new();
    p.p4(BACKPACK).p2_alt3(obj as i32).p2_alt1(slot);
    client.send(ClientProt::IfButton1, &p.finish());
    shown(engine, client, &[])
}

/// A whistle blown where it crosses realms. Arriving in a new region writes interface text
/// of its own (the music player's track), which is no talk to follow: just the ticks.
fn blow(engine: &mut Engine, client: &mut HarnessClient) {
    let slot = slot_of(engine, client, "magic_whistle");
    let obj = id(engine, "obj", "magic_whistle");
    let mut p = ClientPacket::new();
    p.p4(BACKPACK).p2_alt3(obj as i32).p2_alt1(slot);
    client.send(ClientProt::IfButton1, &p.finish());
    settle(engine, client, 4);
}

/// `OPOBJ3` (Take): `p2_alt1(obj) p2_alt2(z) p1(ctrl) p2(x)`, and what it shows.
fn take(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    (x, z): (u16, u16),
) -> Conversation {
    let mut p = ClientPacket::new();
    p.p2_alt1(id(engine, "obj", obj) as i32)
        .p2_alt2(z as i32)
        .p1(0)
        .p2(x as i32);
    client.send(ClientProt::OpObj3, &p.finish());
    shown(engine, client, &[])
}

fn xp(engine: &Engine, client: &HarnessClient, skill: usize) -> i32 {
    engine.harness_player(client.pid).unwrap().stats.xp[skill]
}

/// Fights an npc down to its death's trigger (1 life point, defence 1), standing beside it
/// again before each swing, until `done` holds; what the boxes said on the way.
fn fight(
    engine: &mut Engine,
    client: &mut HarnessClient,
    nid: u16,
    done: impl Fn(&Engine, &HarnessClient, &[String], &[String]) -> bool,
) -> (Vec<String>, Vec<String>) {
    {
        let player = engine.harness_player_mut(client.pid).unwrap();
        for skill in [0, 1, 2, 3] {
            player.stats.levels[skill] = 99;
            player.stats.base_levels[skill] = 99;
        }
        let bit0 = player.vars.get(1240).as_int() & 1;
        player.vars.set(1240, VarValue::Int((990 << 1) | bit0));
    }
    let level = level_of(engine, client);
    let mut texts: Vec<String> = Vec::new();
    let mut chat: Vec<String> = Vec::new();
    for tick in 0..600 {
        if let Some(n) = engine.get_npc_mut(nid) {
            let stats = &mut n.npc.stats.levels;
            stats[NpcStat::Hitpoints as usize] = stats[NpcStat::Hitpoints as usize].min(1);
            stats[NpcStat::Defence as usize] = 1;
        }
        if tick % 8 == 0 {
            if let Some(n) = engine.get_npc(nid) {
                let (x, z) = (n.npc.pathing.coord.x(), n.npc.pathing.coord.z());
                let (px, pz) = coord(engine, client);
                if px.abs_diff(x) > 1 || pz.abs_diff(z) > 1 {
                    if let Some(at) = [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
                        .into_iter()
                        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
                        .find(|&(ax, az)| walkable(ax, level, az))
                    {
                        engine.harness_telejump(client.pid, at.0, level, at.1);
                    }
                }
            }
            let mut p = ClientPacket::new();
            p.p1_alt2(0).p2_alt3(nid as i32);
            client.send(ClientProt::OpNpc2, &p.finish());
        }
        engine.cycle();
        let packets = client.packets();
        texts.extend(settexts(&packets).into_iter().map(|(_, t)| t));
        chat.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
        if done(engine, client, &texts, &chat) {
            break;
        }
    }
    (texts, chat)
}

/// King Arthur sends his new knight after the Grail; Merlin, in his workshop, points to
/// Entrana and Galahad.
#[test]
fn king_arthur_sends_the_knight_and_merlin_advises() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hg_start", 0);
        let t = talk_near(
            engine,
            &mut client,
            "king_arthur",
            &["Tell me of this quest.", "I'd enjoy trying that."],
        );
        says(
            &t,
            &[
                "Now I am a knight of the round table, do you have any more quests for me?",
                "He has set up his workshop in the room next to the library.",
            ],
        );
        assert_eq!(var(engine, &client, "grail"), STARTED);

        // 2011: Merlin stands in the workshop (cache 786's merlin2, shown once knighted).
        assert_eq!(
            npcs(engine, "merlin2").len(),
            1,
            "Merlin is in his workshop"
        );
        let t = talk_near(
            engine,
            &mut client,
            "merlin2",
            &["Thank you for the advice."],
        );
        says(
            &t,
            &[
                "Ah yes... the Holy Grail...",
                "I suppose you could also try speaking to Sir Galahad?",
            ],
        );
        assert_eq!(var(engine, &client, "grail"), SPOKEN_MERLIN);
        let lines = journal(engine, &mut client);
        assert!(!lines.trim().is_empty(), "the journal has the stage");
        no_errors(engine);
    });
}

/// Entrana's High Priest is cut short by the crone, who tells of the whistles; Galahad
/// then parts with his napkin.
#[test]
fn the_crone_and_galahad_point_to_the_whistles() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hg_crone", SPOKEN_MERLIN);
        let t = talk_near(
            engine,
            &mut client,
            "high_priest_of_entrana",
            &["Ok, I will go searching."],
        );
        says(
            &t,
            &[
                "Hello, I am in search of the Holy Grail.",
                "Go to where the six heads face,",
                "Good luck with that.",
            ],
        );
        assert_eq!(var(engine, &client, "grail"), SPOKEN_CRONE);
        assert_eq!(npcs(engine, "grail_crone").len(), 1, "the crone appears");

        let t = talk_near(
            engine,
            &mut client,
            "brother_galahad",
            &["I seek an item from the realm of the Fisher King."],
        );
        says(&t, &["Galahad reluctantly passes you a small cloth."]);
        assert_eq!(held(engine, &client, "holy_table_napkin"), 1);
        no_errors(engine);
    });
}

/// With the napkin carried, the door of the manor's top room lays out two whistles.
#[test]
fn the_whistles_appear_for_the_napkins_bearer() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hg_whistle", SPOKEN_CRONE);
        give(engine, &client, 0, "holy_table_napkin");
        stand_by(engine, &mut client, WHISTLE_DOOR, 2);
        let before = coord(engine, &client);
        let t = op_loc(engine, &mut client, 1, "whistledoor", WHISTLE_DOOR, &[]);
        settle(engine, &mut client, 3);
        let after = coord(engine, &client);
        // They lie on the table (289's tile): taken from inside the room, beside it.
        tele(engine, &mut client, (3106, 3360), 2);
        take(engine, &mut client, "magic_whistle", WHISTLES);
        settle(engine, &mut client, 2);
        take(engine, &mut client, "magic_whistle", WHISTLES);
        settle(engine, &mut client, 2);
        assert_eq!(
            held(engine, &client, "magic_whistle"),
            2,
            "two whistles; door from {before:?} to {after:?}, now {:?}; {:?}",
            coord(engine, &client),
            t.messages
        );

        no_errors(engine);
    });
}

/// A whistle blown under Karamja's tower crosses to the realm and back, and nowhere else.
#[test]
fn a_whistle_crosses_the_realms() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hg_cross", SPOKEN_CRONE);
        give(engine, &client, 0, "magic_whistle");
        // Nowhere in particular: no noise.
        let t = use_item(engine, &mut client, "magic_whistle");
        says(&t, &["The whistle makes no noise."]);

        tele(engine, &mut client, UNDER_THE_TOWER, 0);
        blow(engine, &mut client);
        assert_eq!(coord(engine, &client), REALM_BLIGHTED);
        blow(engine, &mut client);
        assert_eq!(coord(engine, &client), (2741, 3235));

        // Once Percival has his whistle the realm is the restored one.
        set_var(engine, &client, "grail", GIVEN_WHISTLE);
        blow(engine, &mut client);
        assert_eq!(coord(engine, &client), REALM_RESTORED);
        no_errors(engine);
    });
}

/// The Black Knight Titan gets up again unless Excalibur is in hand for the last blow.
#[test]
fn the_titan_falls_to_excalibur_alone() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hg_titan", SPOKEN_CRONE);
        wield(engine, &client, "bronze_sword");
        let nid = stand_near(engine, &mut client, "black_knight_titan", 2);
        let (texts, chat) = fight(engine, &mut client, nid, |_, _, texts, _| {
            texts.iter().any(|t| t.contains("INVINCIBLE"))
        });
        assert!(
            chat.iter()
                .any(|m| m == "Maybe you need something more to beat the titan?"),
            "{chat:?} {texts:?}"
        );
        assert_eq!(var(engine, &client, "grail"), FAILED_TITAN);
        assert!(engine.get_npc(nid).is_some(), "the Titan stands");
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 3);
        no_errors(engine);

        let mut client = login(engine, "hg_titan2", FAILED_TITAN);
        wield(engine, &client, "excalibur");
        let nid = stand_near(engine, &mut client, "black_knight_titan", 2);
        let (texts, chat) = fight(engine, &mut client, nid, |_, _, _, chat| {
            chat.iter()
                .any(|m| m == "Well done! You have defeated the Black Knight Titan!")
        });
        assert!(
            chat.iter()
                .any(|m| m == "Well done! You have defeated the Black Knight Titan!"),
            "{chat:?} {texts:?}"
        );
        settle(engine, &mut client, 4);
        no_errors(engine);
    });
}

/// The fisherman's bells, the maiden's welcome, and the Fisher King's wish for his son.
#[test]
fn the_bell_opens_the_castle_and_the_king_asks_for_percival() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hg_king", SPOKEN_CRONE);
        let t = talk_near(
            engine,
            &mut client,
            "grail_fisherman",
            &["Any idea how to get into the castle?"],
        );
        says(&t, &["Just ring one of the bells outside."]);
        stand_by(engine, &mut client, BELL, 0);
        take(engine, &mut client, "grail_bell", BELL);
        settle(engine, &mut client, 3);
        assert_eq!(held(engine, &client, "grail_bell"), 1, "the bell is taken");
        // Rung within four tiles of a maiden; they wander, so stand beside one.
        let (_, mx, mz, _) = npcs(engine, "grail_maiden")
            .into_iter()
            .filter(|&(_, x, _, _)| x >= 2752)
            .min_by_key(|&(_, x, z, _)| x.abs_diff(BELL.0) + z.abs_diff(BELL.1))
            .expect("a maiden outside the castle");
        stand_by(engine, &mut client, (mx, mz), 0);
        let t = use_item(engine, &mut client, "grail_bell");
        says(
            &t,
            &[
                "Ting-a-ling-a-ling!",
                "Somehow you are now inside the castle.",
            ],
        );
        assert_eq!(coord(engine, &client), INSIDE_THE_CASTLE);

        let t = talk_near(
            engine,
            &mut client,
            "fisher_king",
            &["You don't look too well."],
        );
        says(&t, &["He is known as Percival."]);
        assert_eq!(var(engine, &client, "grail"), FINDING_PERCIVAL);
        no_errors(engine);
    });
}

/// King Arthur's feather points the way; the sack opens for a knight carrying it (2011
/// wiki), and Percival takes a whistle.
#[test]
fn the_feather_finds_percival_in_his_sack() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hg_percival", FINDING_PERCIVAL);
        // No feather: the sack stays shut.
        give(engine, &client, 0, "magic_whistle");
        stand_by(engine, &mut client, SACKS, 0);
        let t = op_loc(engine, &mut client, 2, "percy_sacks", SACKS, &[]);
        tells(&t, "I have no reason to do that.");
        assert!(npcs(engine, "sir_percival").is_empty());

        let t = talk_near(engine, &mut client, "king_arthur", &[]);
        says(&t, &["King Arthur gives you a feather."]);
        assert_eq!(held(engine, &client, "magic_golden_feather"), 1);
        // From Camelot the sacks lie east, a little south: either, by where the king was reached.
        let t = use_item(engine, &mut client, "magic_golden_feather");
        assert!(
            t.messages
                .iter()
                .any(|m| m == "The feather points to the southeast."
                    || m == "The feather points to the east."),
            "{:?}",
            t.messages
        );

        stand_by(engine, &mut client, SACKS, 0);
        let t = op_loc(
            engine,
            &mut client,
            2,
            "percy_sacks",
            SACKS,
            &["Your father wishes to speak to you."],
        );
        says(
            &t,
            &[
                "You hear muffled noises from the sack.",
                "Wow, thank you! I could hardly breathe in there!",
                "Ok, I will see you there then!",
            ],
        );
        assert_eq!(var(engine, &client, "grail"), GIVEN_WHISTLE);
        assert_eq!(held(engine, &client, "magic_whistle"), 0);
        no_errors(engine);
    });
}

/// The Grail stays put until the land is restored; then King Arthur takes it: 2 quest
/// points, 11,000 Prayer xp and 15,300 Defence xp.
#[test]
fn the_grail_is_taken_to_king_arthur() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hg_grail", FINDING_PERCIVAL);
        stand_by(engine, &mut client, GRAIL, 2);
        let t = take(engine, &mut client, "holy_grail", GRAIL);
        says(&t, &["You feel that the Grail shouldn't be moved."]);
        assert_eq!(held(engine, &client, "holy_grail"), 0);

        set_var(engine, &client, "grail", GIVEN_WHISTLE);
        take(engine, &mut client, "holy_grail", GRAIL);
        settle(engine, &mut client, 3);
        assert_eq!(held(engine, &client, "holy_grail"), 1, "the Grail is taken");

        let (qp, prayer, defence) = (
            var(engine, &client, "qp"),
            xp(engine, &client, PRAYER),
            xp(engine, &client, DEFENCE),
        );
        let t = talk_near(engine, &mut client, "king_arthur", &[]);
        says(&t, &["I have retrieved the Grail!"]);
        settle(engine, &mut client, 3);
        assert_eq!(var(engine, &client, "grail"), COMPLETE);
        // The total is counted afresh: Merlin's Crystal's 6 and the Grail's 2.
        assert_eq!(var(engine, &client, "qp") - qp, 8);
        assert_eq!(
            xp(engine, &client, PRAYER) - prayer,
            110_000,
            "11,000 Prayer xp"
        );
        assert_eq!(
            xp(engine, &client, DEFENCE) - defence,
            153_000,
            "15,300 Defence xp"
        );
        assert_eq!(held(engine, &client, "holy_grail"), 0);
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        let lines = journal(engine, &mut client);
        assert!(lines.contains("QUEST COMPLETE!"), "{lines}");
        no_errors(engine);
    });
}
