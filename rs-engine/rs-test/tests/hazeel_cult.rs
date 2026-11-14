//! Hazeel Cult at 647 (`quests/quest_hazeelcult`): 289's quest in 2011's rebuilt Carnillean
//! mansion (cache 786's map moves the wardrobe, the chest and the range, and makes the hollow
//! wall `carnilleanbookcase`). The rewards are the 2011 wiki's and Sal's Realm's as 289's.

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
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `^hazeelcult_started` .. `^hazeelcult_complete`, and the two sides.
const STARTED: i32 = 2;
const CLIVET_DECISION: i32 = 4;
const POURED_POISON: i32 = 5;
const FINISHED_SIDE_TASK: i32 = 6;
const GIVEN_ARMOUR_OR_SCROLL: i32 = 7;
const COMPLETE: i32 = 9;
const GOOD: i32 = 0;
const EVIL: i32 = 1;

/// Cache 786's map.
const CAVE: (u16, u16) = (2585, 3233);
const IN_THE_CAVE: (u16, u16) = (2570, 9682);
const RAFT: (u16, u16) = (2567, 9679);
const SEWERS_END: (u16, u16) = (2606, 9692);
/// The five sewer valves above ground, and the op that turns each the quest's way (the third
/// is turned left, the others right).
const VALVES: [(&str, (u16, u16), u8); 5] = [
    ("sewervalve1", (2562, 3247), 2),
    ("sewervalve2", (2572, 3263), 2),
    ("sewervalve3", (2585, 3245), 1),
    ("sewervalve4", (2597, 3263), 2),
    ("sewervalve5", (2611, 3242), 2),
];
const RANGE: (u16, u16) = (2564, 9671);
/// Upstairs in the mansion: the wardrobe and the hollow wall (level 1), the chest (level 2).
const WARDROBE: (u16, u16) = (2572, 3267);
const HOLLOW_WALL: (u16, u16) = (2566, 3274);
const CHEST: (u16, u16) = (2565, 3272);

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
    set_var(engine, &client, "hazeelcultquest", stage);
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
    var(engine, client, "hazeelcultquest")
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

/// An op on a loc from each tile about it in turn, until it is reached; then its boxes.
fn reach(
    engine: &mut Engine,
    client: &mut HarnessClient,
    op: u8,
    loc: &str,
    tile: (u16, u16),
    level: u8,
) -> Conversation {
    for at in about(tile, level) {
        tele(engine, client, at, level);
        let t = op_loc(engine, client, op, loc, tile, &[]);
        if !t.messages.iter().any(|m| m == "I can't reach that!") {
            return t;
        }
    }
    panic!("{loc} at {tile:?} on level {level} was never reached");
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

/// Ceril Carnillean asks for help, and Clivet in the cave offers the other side: turned down,
/// the player is on Ceril's; taken up, on Hazeel's, with poison for the family's dinner.
#[test]
fn ceril_asks_and_clivet_offers_a_side() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hz_start", 0);
        seasoned(engine, &client);
        let t = talk_near(
            engine,
            &mut client,
            "sir_ceril_carnillean",
            &["What's wrong?", "Yes, of course, I'd be happy to help."],
        );
        assert!(!t.texts.is_empty(), "Ceril talks");
        assert_eq!(stage(engine, &client), STARTED);

        // (a plain op: the cave's music line would be taken for a box)
        for at in about(CAVE, 0) {
            tele(engine, &mut client, at, 0);
            let t = search(engine, &mut client, "hazeelcultcave", CAVE, 6);
            if t.messages.iter().any(|m| m == "You enter the cave.") {
                break;
            }
        }
        assert_eq!(coord(engine, &client), IN_THE_CAVE);

        let t = talk_near(
            engine,
            &mut client,
            "clivet_hazeel_cultist",
            &["What do you mean?", "You're crazy, I'd never help you."],
        );
        assert!(!t.texts.is_empty(), "Clivet talks");
        assert_eq!(stage(engine, &client), CLIVET_DECISION);
        assert_eq!(var(engine, &client, "hazeelcult_side"), GOOD);

        // The other answer, from the start.
        set_var(engine, &client, "hazeelcultquest", STARTED);
        let t = talk_near(
            engine,
            &mut client,
            "clivet_hazeel_cultist",
            &[
                "What do you mean?",
                "So what would I have to do?",
                "Ok, count me in.",
            ],
        );
        assert!(!t.texts.is_empty(), "Clivet talks");
        assert_eq!(stage(engine, &client), CLIVET_DECISION);
        assert_eq!(var(engine, &client, "hazeelcult_side"), EVIL);
        assert_eq!(held(engine, &client, "poison"), 1);
        no_errors(engine);
    });
}

/// The raft goes nowhere against the current; the five sewer valves turned the right way carry
/// it to the end of the sewer.
#[test]
fn the_valves_turn_the_sewers_current() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hz_valves", CLIVET_DECISION);
        seasoned(engine, &client);
        tele(engine, &mut client, IN_THE_CAVE, 0);
        let t = reach(engine, &mut client, 1, "hazeelsewerraft", RAFT, 0);
        tells(
            &t,
            "The current is flowing against the raft. It will not move.",
        );

        for (n, (valve, tile, op)) in VALVES.into_iter().enumerate() {
            let t = reach(engine, &mut client, op, valve, tile, 0);
            assert!(
                t.said("Beneath your feet you can hear the sudden sound of rushing water"),
                "{valve}: {:?}",
                t.texts
            );
            assert_eq!(
                var(engine, &client, "hazeelcult_valves"),
                (1 << (n + 1)) - 1,
                "{valve} sets its bit"
            );
        }

        tele(engine, &mut client, IN_THE_CAVE, 0);
        let t = reach(engine, &mut client, 1, "hazeelsewerraft", RAFT, 0);
        says(&t, &["until it reaches the end of the sewer passage."]);
        assert_eq!(coord(engine, &client), SEWERS_END);
        no_errors(engine);
    });
}

/// Ceril's side: the armour brought back earns five coins and a scroll that the quest is "kind
/// of" complete; the wardrobe upstairs then gives the butler away, and Ceril rewards the
/// player: 1 quest point, 1,500 Thieving experience and 2,000 coins (2011 wiki and Sal's
/// Realm, as 289).
#[test]
fn ceril_is_shown_the_butlers_wardrobe() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hz_good", FINISHED_SIDE_TASK);
        seasoned(engine, &client);
        set_var(engine, &client, "hazeelcult_side", GOOD);
        give(engine, &client, 0, "carnillean_armour");
        // Downstairs he only calls the player up; the armour is handed over upstairs.
        // (he wanders: from each tile about him in turn, until one reaches him)
        let mut t = Conversation::default();
        for attempt in 0..12 {
            let (nid, x, z, _) = npcs(engine, "sir_ceril_carnillean")
                .into_iter()
                .find(|&(.., level)| level == 1)
                .expect("Ceril upstairs");
            let tiles = about((x, z), 1);
            let at = tiles[attempt % tiles.len()];
            tele(engine, &mut client, at, 1);
            talk_to(&mut client, nid);
            t = shown(engine, &mut client, &[]);
            if !t.texts.is_empty() {
                break;
            }
        }
        says(&t, &["Look! I've recovered your armour!"]);
        settle(engine, &mut client, 3);
        assert_eq!(stage(engine, &client), GIVEN_ARMOUR_OR_SCROLL);
        assert_eq!(held(engine, &client, "carnillean_armour"), 0);
        assert_eq!(
            held(engine, &client, "coins"),
            5,
            "five coins for the armour"
        );
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        // The wardrobe (2011's hazeelcbshut, a tile west of 289's) is opened, then searched.
        let thieving = xp(engine, &client, THIEVING);
        reach(engine, &mut client, 1, "hazeelcbshut", WARDROBE, 1);
        settle(engine, &mut client, 3);
        let t = reach(engine, &mut client, 1, "hazeelcbopen", WARDROBE, 1);
        says(
            &t,
            &[
                "You find a bottle of poison and a mysterious amulet.",
                "Look what I've found in the cupboard!",
                "No, no, thank YOU! Feel free to stop by anytime adventurer!",
            ],
        );
        settle(engine, &mut client, 3);
        assert_eq!(stage(engine, &client), COMPLETE);
        assert_eq!(
            xp(engine, &client, THIEVING) - thieving,
            15_000,
            "1,500 Thieving xp"
        );
        assert_eq!(held(engine, &client, "coins"), 2_005, "2,000 coins more");
        no_errors(engine);
    });
}

/// Hazeel's side: poison goes into the family's range; later the hollow wall upstairs (2011's
/// carnilleanbookcase) lets the player through, and the chest behind it, unlocked with the
/// key, has the Scroll of Hazeel.
#[test]
fn hazeels_side_poisons_the_range_and_finds_the_scroll() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "hz_evil", CLIVET_DECISION);
        seasoned(engine, &client);
        set_var(engine, &client, "hazeelcult_side", EVIL);
        give(engine, &client, 0, "poison");
        use_from(
            engine,
            &mut client,
            "poison",
            "cooking_range_2859",
            RANGE,
            0,
        );
        assert_eq!(stage(engine, &client), POURED_POISON);
        assert_eq!(held(engine, &client, "poison"), 0);

        set_var(engine, &client, "hazeelcultquest", FINISHED_SIDE_TASK);
        let before = coord(engine, &client);
        let mut through = false;
        for at in about(HOLLOW_WALL, 1) {
            tele(engine, &mut client, at, 1);
            let t = op_loc(
                engine,
                &mut client,
                1,
                "carnilleanbookcase",
                HOLLOW_WALL,
                &[],
            );
            if t.messages
                .iter()
                .any(|m| m == "You find a secret passageway.")
            {
                settle(engine, &mut client, 3);
                through = coord(engine, &client) != at;
                break;
            }
        }
        assert!(
            through,
            "through the hollow wall (from {before:?} to {:?})",
            coord(engine, &client)
        );

        give(engine, &client, 1, "carnilleanchestkey");
        let t = reach(engine, &mut client, 1, "carnilleanshutchest", CHEST, 2);
        tells(&t, "This chest is locked shut.");
        let t = use_from(
            engine,
            &mut client,
            "carnilleanchestkey",
            "carnilleanshutchest",
            CHEST,
            2,
        );
        tells(&t, "You unlock the chest.");
        settle(engine, &mut client, 3);
        let t = reach(engine, &mut client, 1, "carnilleanopenchest", CHEST, 2);
        says(&t, &["Inside the chest you find the Scroll of Hazeel."]);
        assert_eq!(held(engine, &client, "hazeel_scroll"), 1);
        assert_eq!(stage(engine, &client), GIVEN_ARMOUR_OR_SCROLL);
        no_errors(engine);
    });
}
