//! Eadgar's Ruse at 647 (`quests/quest_eadgar`): 289's quest. Cache 786's map has every loc it
//! operates on 289's tiles (they are indoors or at the zoo); the rewards are the 2011 wiki's as
//! 289's.

#![cfg(all(since_647, before_742))]

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

const BACKPACK: i32 = 149 << 16;
const HERBLORE: usize = 15;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `^eadgar_started` .. `^eadgar_complete`.
const STARTED: i32 = 10;
const SPOKEN_EADGAR_FIRST: i32 = 15;
const SPOKEN_BURNTMEAT_SECOND: i32 = 25;
const NEEDS_PARROT: i32 = 30;
const GOT_FAKE_MAN: i32 = 87;
const GOT_BURNT_MEAT: i32 = 90;
const UNLOCKED_STOREROOM: i32 = 100;
const COMPLETE: i32 = 110;

/// Cache 786's map (289's tiles).
const HATCH: (u16, u16) = (2611, 3287);
const STOREROOM_DOOR: (u16, u16) = (2869, 10085);
const GOUTWEED_CRATE: (u16, u16) = (2856, 10074);
const KNOCKED_OUT_TO: (u16, u16) = (2865, 10088);
/// The five tiles the troll thistle moves between (289's), and Eadgar's at home (level 2).
const THISTLE: [(u16, u16); 5] = [
    (2891, 3671),
    (2897, 3688),
    (2898, 3681),
    (2914, 3665),
    (2914, 3667),
];
const EADGAR_AT_HOME: (u16, u16) = (2890, 10086);

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
    set_var(engine, &client, "eadgar_quest", stage);
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
            .max_by_key(|n| n.3)
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
        .p2_alt1(loc.int())
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
    p.p2_alt2(obj.int())
        .p2_alt2(slot)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(loc.int());
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
    var(engine, client, "eadgar_quest")
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

/// With Death Plateau and Troll Stronghold done and 31 Herblore, Sanfew asks for goutweed.
#[test]
fn sanfew_asks_for_goutweed() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "er_start", 0);
        seasoned(engine, &client);
        set_var(engine, &client, "druidquest", 4);
        set_var(engine, &client, "death_equiproom", 80);
        set_var(engine, &client, "troll_quest", 50);
        let t = talk_near(
            engine,
            &mut client,
            "sanfew",
            &[
                "Have you any more work for me, to help reclaim the circle?",
                "I'll do it.",
            ],
        );
        says(&t, &["Thank you, adventurer!"]);
        assert_eq!(stage(engine, &client), STARTED);
        no_errors(engine);
    });
}

/// Eadgar at home takes up the quest's talk (289's two ways into it), and the troll cook is
/// asked about goutweed.
#[test]
fn eadgar_and_the_troll_cook_are_asked() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "er_eadgar", STARTED);
        seasoned(engine, &client);
        let home = npcs(engine, "eadgar")
            .into_iter()
            .find(|n| n.3 == 2)
            .expect("Eadgar is spawned at home");
        assert!(
            home.1.abs_diff(EADGAR_AT_HOME.0) <= 5 && home.2.abs_diff(EADGAR_AT_HOME.1) <= 5,
            "he wanders about his home tile: {home:?}"
        );
        let t = talk_near(
            engine,
            &mut client,
            "eadgar",
            &["I need to find some goutweed.", "No thanks, Eadgar."],
        );
        assert_eq!(stage(engine, &client), SPOKEN_EADGAR_FIRST, "{:?}", t.texts);
        talk_near(
            engine,
            &mut client,
            "eadgar_troll_chief_cook",
            &["So, where can I get some goutweed?", "I'll be going now."],
        );
        assert_eq!(stage(engine, &client), SPOKEN_BURNTMEAT_SECOND);
        no_errors(engine);
    });
}

/// Alco-chunks through the aviary's hatch catch a drunk parrot.
#[test]
fn alco_chunks_catch_a_parrot() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "er_parrot", NEEDS_PARROT);
        seasoned(engine, &client);
        give(engine, &client, 0, "eadgar_alco_chunks");
        let t = use_from(
            engine,
            &mut client,
            "eadgar_alco_chunks",
            "eadgar_aviary_wall_hatch",
            HATCH,
            0,
        );
        says(
            &t,
            &[
                "You manage to attract a parrot and catch it.",
                "Sqwaawk...*hic*",
                "It looks drunk! You should NEVER feed alcohol to a parrot!",
            ],
        );
        settle(engine, &mut client, 3);
        assert_eq!(held(engine, &client, "eadgar_drunk_parrot"), 1);
        assert_eq!(held(engine, &client, "eadgar_alco_chunks"), 0);
        no_errors(engine);
    });
}

/// The fake man buys burnt meat from the troll cook, and the storeroom's key opens its door.
#[test]
fn the_fake_man_and_the_storeroom_key() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "er_key", GOT_FAKE_MAN);
        seasoned(engine, &client);
        give(engine, &client, 0, "eadgar_fake_man");
        talk_near(
            engine,
            &mut client,
            "eadgar_troll_chief_cook",
            &["So, where can I get some goutweed?", "I'll be going now."],
        );
        assert_eq!(stage(engine, &client), GOT_BURNT_MEAT);
        assert_eq!(held(engine, &client, "eadgar_fake_man"), 0);
        assert_eq!(held(engine, &client, "burnt_meat"), 1);

        let mut refused = false;
        for at in about(STOREROOM_DOOR, 0) {
            tele(engine, &mut client, at, 0);
            let t = search(
                engine,
                &mut client,
                "eadgar_storeroomdoor",
                STOREROOM_DOOR,
                4,
            );
            if t.messages
                .iter()
                .any(|m| m == "You need to find the right key to open this door.")
            {
                refused = true;
                break;
            }
        }
        assert!(refused, "the door wants its key");
        give(engine, &client, 5, "eadgar_troll_storeroom_key");
        for at in about(STOREROOM_DOOR, 0) {
            if stage(engine, &client) == UNLOCKED_STOREROOM {
                break;
            }
            tele(engine, &mut client, at, 0);
            search(
                engine,
                &mut client,
                "eadgar_storeroomdoor",
                STOREROOM_DOOR,
                4,
            );
        }
        assert_eq!(stage(engine, &client), UNLOCKED_STOREROOM);
        assert_eq!(held(engine, &client, "eadgar_troll_storeroom_key"), 0);
        no_errors(engine);
    });
}

/// The crate has goutweed, and its guard knocks the thief out and has him carried off.
#[test]
fn the_goutweed_crate_and_its_guard() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "er_crate", UNLOCKED_STOREROOM);
        seasoned(engine, &client);
        let mut found = false;
        for at in about(GOUTWEED_CRATE, 0) {
            tele(engine, &mut client, at, 0);
            let t = op_loc(
                engine,
                &mut client,
                1,
                "eadgar_crate_goutweed",
                GOUTWEED_CRATE,
                &[],
            );
            if t.said("You've found some goutweed!") {
                found = true;
                break;
            }
        }
        assert!(found, "goutweed is found in the crate");
        assert_eq!(held(engine, &client, "eadgar_goutweed_herb"), 1);
        settle(engine, &mut client, 8);
        assert_eq!(
            coord(engine, &client),
            KNOCKED_OUT_TO,
            "carried out by the guard"
        );
        no_errors(engine);
    });
}

/// Sanfew takes the goutweed: 1 quest point and 11,000 Herblore experience (2011 wiki, as 289).
#[test]
fn sanfew_rewards_the_goutweed() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "er_done", UNLOCKED_STOREROOM);
        seasoned(engine, &client);
        set_var(engine, &client, "druidquest", 4);
        give(engine, &client, 0, "eadgar_goutweed_herb");
        let herblore = xp(engine, &client, HERBLORE);
        let t = talk_near(engine, &mut client, "sanfew", &[]);
        says(&t, &["I have some goutweed!"]);
        settle(engine, &mut client, 6);
        assert_eq!(stage(engine, &client), COMPLETE);
        assert_eq!(
            xp(engine, &client, HERBLORE) - herblore,
            110_000,
            "11,000 Herblore xp"
        );
        assert_eq!(held(engine, &client, "eadgar_goutweed_herb"), 0);
        no_errors(engine);
    });
}

/// The troll thistle's five tiles (289's) are open ground on 2011's redrawn mountain.
#[test]
fn the_thistles_tiles_are_open_ground() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|_engine| {
        for (x, z) in THISTLE {
            assert!(walkable(x, 0, z), "the thistle's tile ({x},{z}) is open");
        }
    });
}
