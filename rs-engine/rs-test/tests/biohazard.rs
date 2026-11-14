//! Biohazard at 647 (`quests/quest_biohazard`; Kilron and Omart in `areas/area_ardougne_west`;
//! King Lathas in `areas/area_ardougne_east`; Varrock's south-east gate): 289's quest. The
//! rewards are the 2011 wiki's and Sal's Realm's as 289's (3 quest points, 1,250 Thieving
//! experience); starting it fills Plague City's tunnel in (2011's dug hole is a patch again).

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
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const BACKPACK: i32 = 149 << 16;
const THIEVING: usize = 17;
/// Worn slots.
const BODY: u16 = 4;
const LEGS: u16 = 7;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `^biohazard_started` .. `^biohazard_complete`.
const STARTED: i32 = 1;
const SPOKEN_JERICO: i32 = 2;
const USED_BIRDFEED: i32 = 3;
const RELEASED_PIGEONS: i32 = 4;
const CLIMBED_LADDER: i32 = 5;
const POISONED_STEW: i32 = 6;
const FOUND_DISTILLATOR: i32 = 7;
const SPOKEN_CHEMIST: i32 = 12;
const FOUND_SECRET: i32 = 14;
const REPORTED_ELENA: i32 = 15;
const COMPLETE: i32 = 16;

/// Plague City: `^elena_complete`, and cache 786's varbits on `elenaquest_extra_bits`.
const ELENA_COMPLETE: i32 = 29;
const ELENA_AT_HOME: i32 = 1 << 1;
const DUG: i32 = 1 << 2;
const GRILL_OFF: i32 = 2 << 4;

/// Cache 786's map.
/// The watchtower's wall pieces (it is a ring of them; some face the wall's far side).
const WATCHTOWER: [(u16, u16); 6] = [
    (2559, 3301),
    (2560, 3301),
    (2561, 3301),
    (2562, 3301),
    (2563, 3302),
    (2563, 3303),
];
const OVER_THE_WALL: (u16, u16) = (2556, 3267);
const BACK_OVER: (u16, u16) = (2559, 3267);
const CAULDRON: (u16, u16) = (2543, 3332);
/// Upstairs in the mourners' headquarters (level 1).
const MOURNERS_GATE: (u16, u16) = (2552, 3325);
const DISTILLATOR_CRATE: (u16, u16) = (2554, 3327);
const GUIDORS_DOOR: (u16, u16) = (3282, 3382);

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
    set_var(engine, &client, "biohazard", stage);
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

/// Stands the player on a walkable tile within two of a loc's tile, nearest first.
fn stand_by(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16), level: u8) {
    let mut tiles: Vec<(i16, i16)> = (-2i16..=2)
        .flat_map(|dx| (-2i16..=2).map(move |dz| (dx, dz)))
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
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
    watch(engine, client, ticks)
}

/// Uses the backpack's `obj` on a loc (`OPLOCT`: `p2_alt2(obj) p2_alt2(slot) p2(z)
/// p4_alt2(component) p2_alt2(x) p1_alt1(ctrl) p2_alt1(loc)`), and what the chat is told over
/// `ticks`.
fn use_on(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    (x, z): (u16, u16),
    ticks: usize,
) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let (obj, loc) = (id(engine, "obj", obj), id(engine, "loc", loc));
    let mut p = ClientPacket::new();
    p.p2_alt2(obj as i32)
        .p2_alt2(slot)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(loc as i32);
    client.send(ClientProt::OpLocT, &p.finish());
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
    var(engine, client, "biohazard")
}

fn wear(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str) {
    let o = id(engine, "obj", name);
    let worn = *engine.cache.invs.debugnames.get("worn").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&worn)
        .unwrap()
        .set(slot, o, 1);
}

/// The walkable tiles within two of a tile on `level`, nearest first.
fn about((x, z): (u16, u16), level: u8) -> Vec<(u16, u16)> {
    let mut tiles: Vec<(i16, i16)> = (-2i16..=2)
        .flat_map(|dx| (-2i16..=2).map(move |dz| (dx, dz)))
        .filter(|&(dx, dz)| (dx, dz) != (0, 0))
        .collect();
    tiles.sort_by_key(|&(dx, dz)| dx.abs() + dz.abs());
    tiles
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .filter(|&(ax, az)| walkable(ax, level, az))
        .collect()
}

/// Uses the backpack's `obj` on a loc from each tile about it in turn, until the chat has `line`.
fn use_until(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    tile: (u16, u16),
    level: u8,
    line: &str,
) -> Conversation {
    let mut last = Conversation::default();
    for at in about(tile, level) {
        tele(engine, client, at, level);
        last = use_on(engine, client, obj, loc, tile, 8);
        if last.messages.iter().any(|m| m == line) {
            return last;
        }
    }
    panic!(
        "289 has {line:?} for {obj} on {loc}; chat {:?}",
        last.messages
    );
}

/// Elena asks for her distillator back: the mourners found Plague City's tunnel and filled it
/// in, so 2011's dug hole is a mud patch again.
#[test]
fn elena_asks_for_her_distillator() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "bio_start", 0);
        set_var(engine, &client, "elenaquest", ELENA_COMPLETE);
        set_var(
            engine,
            &client,
            "elenaquest_extra_bits",
            DUG | GRILL_OFF | ELENA_AT_HOME,
        );
        let t = talk_near(
            engine,
            &mut client,
            "elena2",
            &["I'll try to retrieve it for you."],
        );
        says(
            &t,
            &["Unfortunately they discovered the tunnel and filled it in."],
        );
        assert_eq!(stage(engine, &client), STARTED);
        assert_eq!(
            var(engine, &client, "elenaquest_extra_bits"),
            GRILL_OFF | ELENA_AT_HOME,
            "the dug hole is filled in"
        );
        no_errors(engine);
    });
}

/// Bird feed on the watchtower and a cage of pigeons distract the mourners; Omart's rope
/// ladder then crosses the wall, and Kilron's crosses back.
#[test]
fn the_pigeons_and_the_rope_ladder() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "bio_ladder", SPOKEN_JERICO);
        seasoned(engine, &client);
        give(engine, &client, 0, "birdfeed");
        give(engine, &client, 1, "pigeons");
        let mut thrown = false;
        'pieces: for piece in WATCHTOWER {
            for at in about(piece, 0) {
                tele(engine, &mut client, at, 0);
                let t = use_on(engine, &mut client, "birdfeed", "biowatchtower", piece, 8);
                if t.messages
                    .iter()
                    .any(|m| m == "You throw a hand full of seeds onto the watch tower")
                {
                    thrown = true;
                    break 'pieces;
                }
            }
        }
        assert!(thrown, "the bird feed is thrown onto the watchtower");
        settle(engine, &mut client, 3);
        assert_eq!(stage(engine, &client), USED_BIRDFEED);
        assert_eq!(held(engine, &client, "birdfeed"), 0);

        let slot = slot_of(engine, &client, "pigeons");
        let cage = id(engine, "obj", "pigeons");
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(cage as i32).p2_alt1(slot);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 8);
        tells(&t, "The pigeons fly towards the watch tower");
        tells(&t, "The mourners frantically try to scare them away");
        assert_eq!(stage(engine, &client), RELEASED_PIGEONS);
        assert_eq!(held(engine, &client, "pigeoncage"), 1);

        // Omart calls Kilron and throws the ladder over with no box open between his lines.
        let go = ["Ok, lets do it."];
        let first = talk_near(engine, &mut client, "omart", &go);
        let mut told = first.messages.clone();
        let mut said = first.texts.clone();
        for _ in 0..2 {
            let more = shown(engine, &mut client, &go);
            told.extend(more.messages);
            said.extend(more.texts);
        }
        told.extend(watch(engine, &mut client, 8).messages);
        assert!(
            said.iter().any(|l| l == "Kilron!"),
            "Omart calls Kilron: {said:?}"
        );
        for line in [
            "Omart calls to his associate.",
            "He throws one end of the rope ladder over the wall.",
            "You climb up the rope ladder...",
            "and drop down on the other side.",
        ] {
            assert!(
                told.iter().any(|m| m == line),
                "289 has {line:?}; chat {told:?}"
            );
        }
        assert_eq!(coord(engine, &client), OVER_THE_WALL, "in West Ardougne");
        assert_eq!(stage(engine, &client), CLIMBED_LADDER);

        let t = talk_near(engine, &mut client, "kilron", &["Yes I do."]);
        says(&t, &["Okay, quickly now!"]);
        let mut told = t.messages;
        told.extend(watch(engine, &mut client, 8).messages);
        assert!(
            told.iter().any(|m| m == "and drop down on the other side."),
            "back over the ladder: {told:?}"
        );
        assert_eq!(coord(engine, &client), BACK_OVER, "back in East Ardougne");
        no_errors(engine);
    });
}

/// A rotten apple spoils the mourners' stew; upstairs in their headquarters the key opens the
/// gate and the crate has Elena's distillator.
#[test]
fn the_stew_is_spoiled_and_the_distillator_found() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "bio_stew", CLIMBED_LADDER);
        seasoned(engine, &client);
        give(engine, &client, 0, "rottenapples");
        let t = use_until(
            engine,
            &mut client,
            "rottenapples",
            "mournercauldron",
            CAULDRON,
            0,
            "and it quickly dissolves into the stew.",
        );
        tells(&t, "You place the rotten apple in the pot...");
        assert_eq!(stage(engine, &client), POISONED_STEW);
        assert_eq!(held(engine, &client, "rottenapples"), 0);

        // The gate upstairs wants the mourners' key.
        give(engine, &client, 1, "mournerkeytw");
        let west = (MOURNERS_GATE.0 - 1, MOURNERS_GATE.1);
        tele(engine, &mut client, west, 1);
        use_on(
            engine,
            &mut client,
            "mournerkeytw",
            "mournerquaters_gatel",
            MOURNERS_GATE,
            6,
        );
        assert!(
            coord(engine, &client).0 >= MOURNERS_GATE.0,
            "through the mourners' gate: {:?}",
            coord(engine, &client)
        );
        stand_by(engine, &mut client, DISTILLATOR_CRATE, 1);
        let t = search(engine, &mut client, "mournercrateup", DISTILLATOR_CRATE, 6);
        tells(&t, "and find Elena's distillator.");
        assert_eq!(held(engine, &client, "distillator"), 1);
        assert_eq!(stage(engine, &client), FOUND_DISTILLATOR);
        no_errors(engine);
    });
}

/// Guidor's wife lets only a priest in; Guidor finds the plague is a hoax; King Lathas admits it
/// and gives the reward: 3 quest points and 1,250 Thieving experience (2011 wiki and Sal's
/// Realm, as 289). Underground Pass, which 289's king begins next, waits to be ported.
#[test]
fn guidor_finds_the_hoax_and_the_king_rewards() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "bio_guidor", SPOKEN_CHEMIST);
        seasoned(engine, &client);
        for (slot, obj) in [
            "plaguesample",
            "sulphuric_broline",
            "liquid_honey",
            "ethenea",
            "touch_paper",
        ]
        .into_iter()
        .enumerate()
        {
            give(engine, &client, slot as u16, obj);
        }
        // Unrobed, his wife (289's spawn; the 2011 wiki still has her at the door) turns the
        // player away.
        let mut turned_away = None;
        for at in about(GUIDORS_DOOR, 0) {
            tele(engine, &mut client, at, 0);
            let t = op_loc(engine, &mut client, 1, "guidordoor", GUIDORS_DOOR, &[]);
            if t.messages
                .iter()
                .any(|m| m == "Guidor's wife refuses to let you enter.")
            {
                turned_away = Some((at, t));
                break;
            }
        }
        let (outside, t) = turned_away.expect("Guidor's door is reached from outside");
        says(&t, &["Please leave my husband alone."]);

        wear(engine, &client, BODY, "priest_gown");
        wear(engine, &client, LEGS, "priest_robe");
        tele(engine, &mut client, outside, 0);
        let t = op_loc(engine, &mut client, 1, "guidordoor", GUIDORS_DOOR, &[]);
        tells(&t, "Guidor's wife allows you to go in.");

        let t = talk_near(
            engine,
            &mut client,
            "guidor",
            &[
                "I've come to ask your assistance in stopping a plague.",
                "I've been sent by your old pupil Elena.",
                "So what does that mean exactly?",
            ],
        );
        assert!(!t.texts.is_empty(), "Guidor talks");
        assert_eq!(stage(engine, &client), FOUND_SECRET);
        for obj in [
            "plaguesample",
            "sulphuric_broline",
            "liquid_honey",
            "ethenea",
            "touch_paper",
        ] {
            assert_eq!(held(engine, &client, obj), 0, "{obj} is used in the test");
        }

        set_var(engine, &client, "biohazard", REPORTED_ELENA);
        let thieving = xp(engine, &client, THIEVING);
        let t = talk_near(
            engine,
            &mut client,
            "king_lathas",
            &["I don't understand..."],
        );
        says(
            &t,
            &[
                "I give you permission to use my training area.",
                "He was my brother.",
            ],
        );
        settle(engine, &mut client, 3);
        assert_eq!(stage(engine, &client), COMPLETE);
        assert_eq!(
            xp(engine, &client, THIEVING) - thieving,
            12_500,
            "1,250 Thieving xp"
        );
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        // Underground Pass is not ported: the king does not begin it.
        let (nid, x, z, level) = npcs(engine, "king_lathas").into_iter().next().unwrap();
        let at = about((x, z), level)
            .into_iter()
            .next()
            .expect("a tile by the king");
        tele(engine, &mut client, at, level);
        talk_to(&mut client, nid);
        let t = watch(engine, &mut client, 6);
        tells(&t, "The king is too busy to talk.");
        assert_eq!(var(engine, &client, "ibanmulti"), 0);
        no_errors(engine);
    });
}
