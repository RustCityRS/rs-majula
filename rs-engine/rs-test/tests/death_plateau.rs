//! Death Plateau at 647 (`quests/quest_death`): 289's quest on 2011's redrawn mountain. Cache
//! 786's map gives Saba's cave new entrance and exit locs, makes both of Tenzing's doors one loc,
//! and its obj table has a new pair of quest climbing boots beside 289's (now "Rock climbing
//! boots"). The rewards are the 2011 wiki's and Sal's Realm's as 289's.

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
const ATTACK: usize = 0;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `^death_started` .. `^death_complete` (`%death_equiproom`).
const STARTED: i32 = 10;
const GIVEN_IOU: i32 = 55;
const FOUND_COMBO: i32 = 60;
const UNLOCKED_DOOR: i32 = 70;
const COMPLETE: i32 = 80;
/// `^death_spoken_saba` .. `^death_scouted_area` (the low four bits of `%death_map`).
const SPOKEN_SABA: i32 = 1;
const SPOKEN_TENZING: i32 = 2;
const GOT_ENTRANCECERT: i32 = 4;
const GIVEN_CERT: i32 = 5;
const GIVEN_SUPPLIES: i32 = 6;
const GOT_MAP: i32 = 7;
const SCOUTED_AREA: i32 = 8;

/// Cache 786's map.
const CAVE_ENTRANCE: (u16, u16) = (2857, 3578);
const IN_THE_CAVE: (u16, u16) = (2269, 4752);
const CAVE_EXIT: (u16, u16) = (2268, 4750);
const OUT_OF_THE_CAVE: (u16, u16) = (2858, 3577);
const TENZINGS_DOOR: (u16, u16) = (2823, 3555);
const CASTLE_DOOR: (u16, u16) = (2894, 3566);
/// The stone mechanism's five tiles, with the ball the combination puts on each.
const MECHANISM: [(&str, &str, (u16, u16)); 5] = [
    (
        "death_cannonball_blue",
        "death_stone_mechanism_corner",
        (2894, 3562),
    ),
    (
        "death_cannonball_yellow",
        "death_stone_mechanism_corner",
        (2895, 3562),
    ),
    (
        "death_cannonball_red",
        "death_stone_mechanism_side",
        (2894, 3563),
    ),
    (
        "death_cannonball_purple",
        "death_stone_mechanism_side",
        (2895, 3563),
    ),
    (
        "death_cannonball_green",
        "death_stone_mechanism_corner",
        (2895, 3564),
    ),
];
/// The zone (`0_44_56_48_24`) from which the secret path is seen to be clear.
const LOOKOUT: (u16, u16) = (2864, 3608);

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
    set_var(engine, &client, "death_equiproom", stage);
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
    var(engine, client, "death_equiproom")
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

/// The backpack's `obj` on a loc from each tile about it in turn, until it is reached.
fn use_from(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    tile: (u16, u16),
    level: u8,
) -> Conversation {
    for at in about(tile, level) {
        tele(engine, client, at, level);
        use_on(engine, client, obj, loc, tile, 0);
        let t = shown(engine, client, &[]);
        if !t.messages.iter().any(|m| m == "I can't reach that!") {
            return t;
        }
    }
    panic!("{loc} at {tile:?} on level {level} was never reached with {obj}");
}

fn map(engine: &Engine, client: &HarnessClient) -> i32 {
    var(engine, client, "death_map") & 0xf
}

/// A plain `OPLOC1` from each tile about a loc in turn, until `done` holds.
fn plain_until(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    tile: (u16, u16),
    ticks: usize,
    done: impl Fn(&Engine, &HarnessClient, &Conversation) -> bool,
) -> Conversation {
    let mut last = Conversation::default();
    for dx in -1i16..=4 {
        for dz in -1i16..=4 {
            let at = (
                tile.0.wrapping_add_signed(dx),
                tile.1.wrapping_add_signed(dz),
            );
            if !walkable(at.0, 0, at.1) {
                continue;
            }
            tele(engine, client, at, 0);
            last = search(engine, client, loc, tile, ticks);
            if done(engine, client, &last) {
                return last;
            }
        }
    }
    panic!("{loc} at {tile:?}: {:?}", last.messages);
}

/// Denulth asks for another way up Death Plateau and for the equipment room's combination.
#[test]
fn denulth_starts_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "dp_start", 0);
        let t = talk_near(
            engine,
            &mut client,
            "denulth",
            &[
                "Do you have any quests for me?",
                "No but perhaps I could try and find one?",
            ],
        );
        says(
            &t,
            &["Citizen you would be well rewarded!", "A stone what...?!"],
        );
        assert_eq!(stage(engine, &client), STARTED);
        no_errors(engine);
    });
}

/// Harold's IOU is written on the back of the combination.
#[test]
fn the_iou_is_the_combination() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "dp_iou", GIVEN_IOU);
        give(engine, &client, 0, "death_iou");
        let iou = id(engine, "obj", "death_iou");
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(iou as i32).p2_alt1(0);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = shown(engine, &mut client, &[]);
        says(&t, &["The IOU is written on the back of the combination!"]);
        assert_eq!(stage(engine, &client), FOUND_COMBO);
        assert_eq!(held(engine, &client, "death_iou"), 0);
        assert_eq!(held(engine, &client, "death_combination"), 1);
        no_errors(engine);
    });
}

/// The five stone balls on the mechanism in the combination's order unlock the equipment room.
#[test]
fn the_stone_balls_unlock_the_equipment_room() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "dp_balls", FOUND_COMBO);
        let t = plain_until(
            engine,
            &mut client,
            "death_castledoor",
            CASTLE_DOOR,
            4,
            |_, _, t| t.messages.iter().any(|m| m == "The door is locked."),
        );
        tells(&t, "The door is locked.");
        for (slot, (ball, _, _)) in MECHANISM.into_iter().enumerate() {
            give(engine, &client, slot as u16, ball);
        }
        for (n, (ball, loc, tile)) in MECHANISM.into_iter().enumerate() {
            let t = use_from(engine, &mut client, ball, loc, tile, 0);
            assert_eq!(
                held(engine, &client, ball),
                0,
                "{ball} is put down: {:?}",
                t.messages
            );
            let unlocked = stage(engine, &client) == UNLOCKED_DOOR;
            assert_eq!(unlocked, n == 4, "after {ball}");
        }
        plain_until(
            engine,
            &mut client,
            "death_castledoor",
            CASTLE_DOOR,
            4,
            |_, _, t| {
                !t.messages
                    .iter()
                    .any(|m| m == "The door is locked." || m == "I can't reach that!")
            },
        );
        no_errors(engine);
    });
}

/// 2011's cave locs lead into Saba's cave and out again, and Tenzing answers his door.
#[test]
fn sabas_cave_and_tenzings_door() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "dp_cave", STARTED);
        plain_until(
            engine,
            &mut client,
            "trollheim_mountain_cave_entrance_4x4_1a",
            CAVE_ENTRANCE,
            4,
            |e, c, _| coord(e, c) == IN_THE_CAVE,
        );
        plain_until(
            engine,
            &mut client,
            "di2_cave_wall_exit_light_grey",
            CAVE_EXIT,
            4,
            |e, c, _| coord(e, c) == OUT_OF_THE_CAVE,
        );

        // Before Saba has told of him, Tenzing sends the caller away.
        let mut answered = false;
        for at in about(TENZINGS_DOOR, 0) {
            tele(engine, &mut client, at, 0);
            let t = op_loc(
                engine,
                &mut client,
                1,
                "death_sherpa_door",
                TENZINGS_DOOR,
                &[],
            );
            if t.said("No milk today! Thank you!") {
                answered = true;
                assert!(!t.said("I'm not the milkman"), "{:?}", t.texts);
                break;
            }
        }
        assert!(answered, "Tenzing answers a knock");
        set_var(engine, &client, "death_map", SPOKEN_SABA);
        let mut let_in = false;
        for at in about(TENZINGS_DOOR, 0) {
            tele(engine, &mut client, at, 0);
            let t = op_loc(
                engine,
                &mut client,
                1,
                "death_sherpa_door",
                TENZINGS_DOOR,
                &[],
            );
            if t.said("Oh...OK. You'd better come in then.") {
                let_in = true;
                break;
            }
        }
        assert!(
            let_in,
            "Tenzing lets the player in once Saba has told of him"
        );
        no_errors(engine);
    });
}

/// Tenzing lends 2011's quest climbing boots, Dunstan spikes that pair for the certificate and an
/// iron bar, and Tenzing takes the spiked boots, ten loaves and ten trout for his map; the path
/// is then seen to be clear from the lookout.
#[test]
fn the_boots_the_supplies_and_the_secret_way() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "dp_boots", STARTED);
        set_var(engine, &client, "death_map", SPOKEN_SABA);
        let t = talk_near(
            engine,
            &mut client,
            "death_sherpa",
            &["OK, I'll get those for you."],
        );
        assert_eq!(map(engine, &client), SPOKEN_TENZING, "{:?}", t.texts);
        assert_eq!(
            held(engine, &client, "death_climbingboots_quest"),
            1,
            "2011's quest pair"
        );
        assert_eq!(
            held(engine, &client, "death_climbingboots"),
            0,
            "not the rock climbing boots"
        );

        set_var(engine, &client, "death_map", GOT_ENTRANCECERT);
        give(engine, &client, 5, "death_entrancecert");
        give(engine, &client, 6, "iron_bar");
        let t = talk_near(engine, &mut client, "death_smithy", &[]);
        assert_eq!(
            held(engine, &client, "death_spikedboots"),
            1,
            "{:?}",
            t.texts
        );
        assert_eq!(held(engine, &client, "death_climbingboots_quest"), 0);
        assert_eq!(held(engine, &client, "death_entrancecert"), 0);
        assert_eq!(map(engine, &client), GIVEN_CERT);

        for n in 0..10u16 {
            give(engine, &client, 7 + n, "bread");
            give(engine, &client, 17 + n, "trout");
        }
        let t = talk_near(engine, &mut client, "death_sherpa", &[]);
        assert!(map(engine, &client) >= GIVEN_SUPPLIES, "{:?}", t.texts);
        if map(engine, &client) == GIVEN_SUPPLIES {
            talk_near(engine, &mut client, "death_sherpa", &[]);
        }
        assert_eq!(map(engine, &client), GOT_MAP);
        assert_eq!(held(engine, &client, "death_secretwaymap"), 1);
        assert_eq!(held(engine, &client, "bread"), 0);
        assert_eq!(held(engine, &client, "trout"), 0);
        assert_eq!(held(engine, &client, "death_spikedboots"), 0);

        // Into the lookout's zone on foot from just outside it.
        let inside = (0..8u16)
            .flat_map(|dx| (0..8u16).map(move |dz| (LOOKOUT.0 + dx, LOOKOUT.1 + dz)))
            .find(|&(x, z)| walkable(x, 0, z))
            .expect("a walkable tile in the lookout's zone");
        tele(engine, &mut client, inside, 0);
        let t = shown(engine, &mut client, &[]);
        assert_eq!(
            map(engine, &client),
            SCOUTED_AREA,
            "{:?} {:?}",
            t.messages,
            t.texts
        );
        no_errors(engine);
    });
}

/// Denulth takes the map and the combination and rewards the player: 1 quest point, 3,000
/// Attack experience and steel claws (2011 wiki and Sal's Realm, as 289).
#[test]
fn denulth_rewards_the_secret_way_and_the_combination() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "dp_done", UNLOCKED_DOOR);
        seasoned(engine, &client);
        set_var(engine, &client, "death_map", SCOUTED_AREA);
        give(engine, &client, 0, "death_secretwaymap");
        give(engine, &client, 1, "death_combination");
        let attack = xp(engine, &client, ATTACK);
        let t = talk_near(engine, &mut client, "denulth", &[]);
        says(
            &t,
            &["You are now an honorary member of the Imperial Guard!"],
        );
        settle(engine, &mut client, 6);
        assert_eq!(stage(engine, &client), COMPLETE);
        assert_eq!(
            xp(engine, &client, ATTACK) - attack,
            30_000,
            "3,000 Attack xp"
        );
        assert_eq!(held(engine, &client, "steel_claws"), 1);
        assert_eq!(held(engine, &client, "death_secretwaymap"), 0);
        assert_eq!(held(engine, &client, "death_combination"), 0);
        no_errors(engine);
    });
}
