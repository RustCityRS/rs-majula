//! Waterfall Quest at 647 (`quests/quest_waterfall`): 289's quest on 2011's map. The rewards are
//! the 2011 wiki's and Sal's Realm's as 289's (1 quest point, 13,750 Attack and Strength
//! experience, 2 diamonds, 2 gold bars, 40 mithril seeds); the falls' and the boulder's damage is
//! in 2011's life points.

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
const STRENGTH: usize = 2;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `^waterfall_started` .. `^waterfall_complete`.
const STARTED: i32 = 1;
const SPOKEN_TO_HUDON: i32 = 2;
const OPENED_BOOK: i32 = 3;
const ENTERED_TOMB: i32 = 4;
const ENTERED_WATERFALL: i32 = 5;
const ENTERED_PUZZLE_ROOM: i32 = 6;
const PLACED_AMULET: i32 = 8;
const COMPLETE: i32 = 10;

/// Cache 786's map (the tiles are 289's but where noted in the scripts).
const RAFT: (u16, u16) = (2509, 3493);
const MOUND: (u16, u16) = (2512, 3481);
const ROCK: (u16, u16) = (2512, 3468);
const TREE: (u16, u16) = (2512, 3465);
const LEDGE: (u16, u16) = (2511, 3463);
const LEDGE_DOOR: (u16, u16) = (2511, 3464);
const WASHED_UP: (u16, u16) = (2527, 3413);
const INSIDE: (u16, u16) = (2575, 9861);
const WAY_OUT: (u16, u16) = (2574, 9860);
const BOOKCASE: (u16, u16) = (2520, 3426);
const GOLRIE_CRATE: (u16, u16) = (2548, 9565);
const GOLRIE_GATE: (u16, u16) = (2515, 9575);
const TOMBSTONE: (u16, u16) = (2558, 3444);
const TOMB: (u16, u16) = (2554, 9844);
const TOMB_CHEST: (u16, u16) = (2530, 9844);
const COFFIN: (u16, u16) = (2542, 9811);
const KEY_CRATE: (u16, u16) = (2589, 9888);
const GROUND_KEY: (u16, u16) = (2569, 9897);
const PUZZLE_DOOR: (u16, u16) = (2566, 9901);
const PILLARS: [(u16, u16); 6] = [
    (2562, 9910),
    (2562, 9912),
    (2562, 9914),
    (2569, 9910),
    (2569, 9912),
    (2569, 9914),
];
const STATUE: (u16, u16) = (2565, 9916);
const RAISED: (u16, u16) = (2603, 9914);
const RAISED_STATUE: (u16, u16) = (2603, 9915);
const RAISED_CHALICE: (u16, u16) = (2603, 9910);

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
    set_var(engine, &client, "waterfall_quest", stage);
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

fn life(engine: &Engine, client: &HarnessClient) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(LIFE)
        .as_int()
        >> 1
}

fn xp(engine: &Engine, client: &HarnessClient, skill: usize) -> i32 {
    engine.harness_player(client.pid).unwrap().stats.xp[skill]
}

/// Almera's raft is left alone until she asks for help; then it runs aground by Hudon, who will
/// not leave the falls.
#[test]
fn almera_sends_the_player_down_the_river() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wf_start", 0);
        stand_by(engine, &mut client, RAFT, 0);
        let t = op_loc(engine, &mut client, 1, "lograft_waterfall_quest", RAFT, &[]);
        says(&t, &["You're not sure if the raft is safe to use."]);

        let t = talk_near(
            engine,
            &mut client,
            "almera_waterfall_quest",
            &["How can I help?"],
        );
        says(&t, &["You can use the small raft out back if you wish,"]);
        assert_eq!(var(engine, &client, "waterfall_quest"), STARTED);

        stand_by(engine, &mut client, RAFT, 0);
        let boarded = search(engine, &mut client, "lograft_waterfall_quest", RAFT, 8);
        tells(&boarded, "You board the small raft");
        let t = shown(engine, &mut client, &[]);
        says(
            &t,
            &["Hello son, are you okay? You need help?", "I'm fine alone."],
        );
        tells(&t, "Hudon is refusing to leave the waterfall");
        assert_eq!(coord(engine, &client), MOUND);
        assert_eq!(var(engine, &client, "waterfall_quest"), SPOKEN_TO_HUDON);
        no_errors(engine);
    });
}

/// From the mound: a swim is washed downstream, a rope reaches the rock, a rope on the tree lets
/// the player down to the ledge, and the door there floods without Glarial's amulet (80 life
/// points, 289's 8 hitpoints).
#[test]
fn the_rope_the_tree_and_the_ledge() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wf_ledge", ENTERED_TOMB);
        seasoned(engine, &client);
        give(engine, &client, 0, "rope");
        tele(engine, &mut client, (MOUND.0, MOUND.1 - 5), 0);
        let t = search(
            engine,
            &mut client,
            "crossing_rock_waterfall_quest",
            ROCK,
            12,
        );
        tells(
            &t,
            "It looks like a long distance, but you swim out into the water.",
        );
        tells(&t, "You are washed downstream but feel lucky to be alive.");
        assert_eq!(coord(engine, &client), WASHED_UP);

        tele(engine, &mut client, (MOUND.0, MOUND.1 - 5), 0);
        use_on(
            engine,
            &mut client,
            "rope",
            "crossing_rock_waterfall_quest",
            ROCK,
            24,
        );
        assert_eq!(
            coord(engine, &client),
            (ROCK.0 + 1, ROCK.1),
            "across by the rope"
        );

        let t = use_on(
            engine,
            &mut client,
            "rope",
            "overhanging_tree1_waterfall_quest",
            TREE,
            8,
        );
        tells(&t, "You tie the rope to the tree");
        assert_eq!(coord(engine, &client), LEDGE);
        assert_eq!(held(engine, &client, "rope"), 1, "the rope is kept");

        let before = life(engine, &client);
        let t = search(
            engine,
            &mut client,
            "gl_waterfall_ledge_door",
            LEDGE_DOOR,
            12,
        );
        tells(&t, "Suddenly the corridor floods...");
        tells(&t, "and are washed up by the riverside.");
        assert_eq!(coord(engine, &client), WASHED_UP);
        let lost = before - life(engine, &client);
        assert!((70..=80).contains(&lost), "the fall: {lost}");
        assert_eq!(var(engine, &client, "waterfall_quest"), ENTERED_TOMB);

        // With the amulet the door lets the player in, and the way out leads back to the ledge.
        give(engine, &client, 1, "glarials_amulet_waterfall_quest");
        engine.harness_telejump(client.pid, LEDGE.0, 0, LEDGE.1);
        settle(engine, &mut client, 3);
        let t = search(
            engine,
            &mut client,
            "gl_waterfall_ledge_door",
            LEDGE_DOOR,
            8,
        );
        tells(&t, "You walk through the door.");
        assert_eq!(coord(engine, &client), INSIDE);
        assert_eq!(var(engine, &client, "waterfall_quest"), ENTERED_WATERFALL);
        let t = search(engine, &mut client, "di2_cave_wall_door_exit", WAY_OUT, 6);
        tells(&t, "You open the door and walk through.");
        assert_eq!(coord(engine, &client), LEDGE);
        no_errors(engine);
    });
}

/// The tourist centre's bookcase has the Book on Baxtorian once Hudon is spoken to; reading it
/// moves the quest on. Golrie's key is in the crate, opens his gate, and he gives up Glarial's
/// pebble.
#[test]
fn the_book_and_golries_pebble() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wf_book", SPOKEN_TO_HUDON);
        stand_by(engine, &mut client, BOOKCASE, 1);
        let t = search(engine, &mut client, "bookcase_waterfall_quest", BOOKCASE, 8);
        tells(&t, "You find a book named 'Book on Baxtorian'");
        assert_eq!(held(engine, &client, "baxtorian_book_waterfall_quest"), 1);

        let slot = slot_of(engine, &client, "baxtorian_book_waterfall_quest");
        let book = id(engine, "obj", "baxtorian_book_waterfall_quest");
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(book as i32).p2_alt1(slot);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 4);
        tells(&t, "The book is old with many pages missing,");
        assert_eq!(var(engine, &client, "waterfall_quest"), OPENED_BOOK);
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        // Golrie has locked himself in and lost the key.
        // He answers through the gate when he is within five tiles of it (he wanders), and the
        // cave's hobgoblins can pull the player off the gate: tried until he does.
        seasoned(engine, &client);
        let mut answered = false;
        for _ in 0..10 {
            tele(engine, &mut client, GOLRIE_GATE, 0);
            let t = op_loc(
                engine,
                &mut client,
                1,
                "golrie_gate_waterfall_quest",
                GOLRIE_GATE,
                &[],
            );
            if t.said("I locked myself in here for protection, but I've left the key somewhere.") {
                answered = true;
                break;
            }
            settle(engine, &mut client, 5);
        }
        assert!(answered, "Golrie answers through the gate");

        stand_by(engine, &mut client, GOLRIE_CRATE, 0);
        let t = search(
            engine,
            &mut client,
            "golrie_crate_waterfall_quest",
            GOLRIE_CRATE,
            8,
        );
        tells(&t, "and find a large key");
        assert_eq!(held(engine, &client, "golrie_key_waterfall_quest"), 1);

        tele(engine, &mut client, GOLRIE_GATE, 0);
        let t = use_on(
            engine,
            &mut client,
            "golrie_key_waterfall_quest",
            "golrie_gate_waterfall_quest",
            GOLRIE_GATE,
            8,
        );
        tells(&t, "The key fits the gate");
        assert_eq!(
            coord(engine, &client),
            (GOLRIE_GATE.0, GOLRIE_GATE.1 + 1),
            "through the gate"
        );

        // He lets the player look through his junk (no box is open meanwhile), then the talk
        // goes on, and stops once more as the key is handed over.
        let t = talk_near(engine, &mut client, "golrie_waterfall_quest", &[]);
        says(&t, &["No, of course not."]);
        tells(
            &t,
            "mixed with the junk on the floor you find Glarial's pebble.",
        );
        let t = shown(engine, &mut client, &[]);
        says(&t, &["Could I take this old pebble?"]);
        let more = shown(engine, &mut client, &[]);
        assert!(
            t.messages
                .iter()
                .chain(&more.messages)
                .any(|m| m == "You give Golrie the key."),
            "the key is handed over: {:?} {:?}",
            t.messages,
            more.messages
        );
        assert!(
            t.said("Thanks a lot for the key traveller.")
                || more.said("Thanks a lot for the key traveller."),
            "{:?} {:?}",
            t.texts,
            more.texts
        );
        assert_eq!(held(engine, &client, "glarials_pebble_waterfall_quest"), 1);
        assert_eq!(held(engine, &client, "golrie_key_waterfall_quest"), 0);
        assert_eq!(var(engine, &client, "waterfall_golrie_and_puzzle") & 1, 1);

        let mut locked = false;
        for _ in 0..5 {
            tele(engine, &mut client, GOLRIE_GATE, 0);
            let t = search(
                engine,
                &mut client,
                "golrie_gate_waterfall_quest",
                GOLRIE_GATE,
                4,
            );
            if t.messages
                .iter()
                .any(|m| m == "Golrie has locked himself in.")
            {
                locked = true;
                break;
            }
        }
        assert!(locked, "the gate stays locked once he has the key");
        no_errors(engine);
    });
}

/// Glarial's tombstone takes the pebble only from a visitor of peaceful intent (no runes); in
/// the tomb the chest has her amulet and the coffin her urn.
#[test]
fn glarials_tomb_has_the_amulet_and_the_urn() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wf_tomb", OPENED_BOOK);
        // The tomb's dead attack on sight: a new player does not always live through the search.
        seasoned(engine, &client);
        give(engine, &client, 0, "glarials_pebble_waterfall_quest");
        give(engine, &client, 1, "airrune");
        stand_by(engine, &mut client, TOMBSTONE, 0);
        let t = search(
            engine,
            &mut client,
            "glarials_tombstone_waterfall_quest",
            TOMBSTONE,
            18,
        );
        tells(&t, "Here lies Glarial, wife of Baxtorian,");
        tells(&t, "only visitors with peaceful intent can enter.");

        let t = use_on(
            engine,
            &mut client,
            "glarials_pebble_waterfall_quest",
            "glarials_tombstone_waterfall_quest",
            TOMBSTONE,
            8,
        );
        tells(&t, "But nothing happens.");
        assert_eq!(var(engine, &client, "waterfall_quest"), OPENED_BOOK);

        give(engine, &client, 1, "bread");
        let t = use_on(
            engine,
            &mut client,
            "glarials_pebble_waterfall_quest",
            "glarials_tombstone_waterfall_quest",
            TOMBSTONE,
            10,
        );
        tells(&t, "The stone slab slides back revealing a ladder down.");
        assert_eq!(coord(engine, &client), TOMB);
        assert_eq!(var(engine, &client, "waterfall_quest"), ENTERED_TOMB);
        assert_eq!(
            held(engine, &client, "glarials_pebble_waterfall_quest"),
            1,
            "the pebble is kept"
        );

        // 2011's chest is a plain tomb chest: opened as any, then searched.
        stand_by(engine, &mut client, TOMB_CHEST, 0);
        let t = search(engine, &mut client, "di2_chestclosed_dark", TOMB_CHEST, 10);
        assert!(
            t.messages.is_empty(),
            "the chest opens: {:?} from {:?}",
            t.messages,
            coord(engine, &client)
        );
        let t = search(engine, &mut client, "di2_chestopen_dark", TOMB_CHEST, 8);
        tells(&t, "You search the chest and find a small amulet.");
        assert_eq!(held(engine, &client, "glarials_amulet_waterfall_quest"), 1);
        let t = search(engine, &mut client, "di2_chestopen_dark", TOMB_CHEST, 4);
        tells(&t, "You search the chest but find nothing.");

        stand_by(engine, &mut client, COFFIN, 0);
        let t = search(
            engine,
            &mut client,
            "di2_glarials_tomb_waterfall_quest",
            COFFIN,
            8,
        );
        tells(&t, "Inside you find an urn full of ashes.");
        assert_eq!(
            held(engine, &client, "glarials_urn_full_waterfall_quest"),
            1
        );
        let t = search(
            engine,
            &mut client,
            "di2_glarials_tomb_waterfall_quest",
            COFFIN,
            8,
        );
        tells(&t, "You find nothing of interest.");
        assert_eq!(
            held(engine, &client, "glarials_urn_full_waterfall_quest"),
            1
        );
        no_errors(engine);
    });
}

/// Under the falls: the key from the crate (and 289's on the floor), the puzzle room, a boulder
/// for an amulet hung too soon (200 life points, 289's 20 hitpoints), the eighteen runes, the
/// raised room, and the ashes for the treasure (2011 wiki and Sal's Realm, as 289).
#[test]
fn the_treasure_of_baxtorian() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "wf_treasure", ENTERED_WATERFALL);
        seasoned(engine, &client);
        give(engine, &client, 0, "glarials_amulet_waterfall_quest");
        give(engine, &client, 1, "glarials_urn_full_waterfall_quest");
        {
            let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
            let runes = ["airrune", "earthrune", "waterrune"].map(|r| id(engine, "obj", r));
            let player = engine.harness_player_mut(client.pid).unwrap();
            for (slot, rune) in runes.into_iter().enumerate() {
                player
                    .invs
                    .get_mut(&inv)
                    .unwrap()
                    .set(2 + slot as u16, rune, 6);
            }
        }

        // 289's key on the floor of the first room.
        tele(engine, &mut client, (GROUND_KEY.0 - 1, GROUND_KEY.1), 0);
        let key = id(engine, "obj", "baxtorian_key_waterfall_quest");
        let mut p = ClientPacket::new();
        p.p2_alt1(key as i32)
            .p2_alt2(GROUND_KEY.1 as i32)
            .p1(0)
            .p2(GROUND_KEY.0 as i32);
        client.send(ClientProt::OpObj3, &p.finish());
        settle(engine, &mut client, 4);
        assert_eq!(
            held(engine, &client, "baxtorian_key_waterfall_quest"),
            1,
            "the key on the floor"
        );

        stand_by(engine, &mut client, KEY_CRATE, 0);
        let t = search(
            engine,
            &mut client,
            "baxtorian_crate_waterfall_quest",
            KEY_CRATE,
            8,
        );
        tells(&t, "but find nothing");

        // The door is locked without the key in hand, and opens to it.
        tele(engine, &mut client, PUZZLE_DOOR, 0);
        let t = search(
            engine,
            &mut client,
            "baxtorian_door_2_waterfall_quest",
            PUZZLE_DOOR,
            4,
        );
        tells(&t, "The door is locked.");
        let t = use_on(
            engine,
            &mut client,
            "baxtorian_key_waterfall_quest",
            "baxtorian_door_2_waterfall_quest",
            PUZZLE_DOOR,
            6,
        );
        tells(&t, "You open the door and walk through.");
        assert_eq!(coord(engine, &client), (PUZZLE_DOOR.0, PUZZLE_DOOR.1 + 1));
        assert_eq!(var(engine, &client, "waterfall_quest"), ENTERED_PUZZLE_ROOM);

        // The amulet before the runes: a boulder.
        stand_by(engine, &mut client, STATUE, 0);
        let before = life(engine, &client);
        let t = use_on(
            engine,
            &mut client,
            "glarials_amulet_waterfall_quest",
            "statue_queen_waterfall_quest",
            STATUE,
            8,
        );
        tells(&t, "A large boulder hits you on the head!");
        let lost = before - life(engine, &client);
        assert!((190..=200).contains(&lost), "the boulder: {lost}");
        assert_eq!(held(engine, &client, "glarials_amulet_waterfall_quest"), 1);

        for pillar in PILLARS {
            stand_by(engine, &mut client, pillar, 0);
            for rune in ["airrune", "earthrune", "waterrune"] {
                let t = use_on(
                    engine,
                    &mut client,
                    rune,
                    "stonepillar_small_waterfall_quest",
                    pillar,
                    4,
                );
                tells(&t, "The rune stone disappears in a puff of smoke.");
            }
        }
        assert_eq!(
            var(engine, &client, "waterfall_golrie_and_puzzle") >> 1,
            (1 << 18) - 1,
            "a rune of each kind on each pillar"
        );
        for rune in ["airrune", "earthrune", "waterrune"] {
            assert_eq!(held(engine, &client, rune), 0, "six of each are used");
        }

        stand_by(engine, &mut client, STATUE, 0);
        let t = use_on(
            engine,
            &mut client,
            "glarials_amulet_waterfall_quest",
            "statue_queen_waterfall_quest",
            STATUE,
            10,
        );
        tells(&t, "The ground raises up before you!");
        assert_eq!(coord(engine, &client), RAISED);
        assert_eq!(var(engine, &client, "waterfall_quest"), PLACED_AMULET);
        assert_eq!(held(engine, &client, "glarials_amulet_waterfall_quest"), 0);
        assert!(
            RAISED_STATUE.1 > RAISED.1,
            "the raised room's statue is behind the player"
        );

        let (attack, strength) = (xp(engine, &client, ATTACK), xp(engine, &client, STRENGTH));
        let t = use_on(
            engine,
            &mut client,
            "glarials_urn_full_waterfall_quest",
            "baxtorian_chalice_waterfall_quest",
            RAISED_CHALICE,
            14,
        );
        tells(&t, "You carefully pour the ashes into the chalice");
        tells(&t, "two diamonds and two gold bars.");
        assert_eq!(var(engine, &client, "waterfall_quest"), COMPLETE);
        assert_eq!(held(engine, &client, "diamond"), 2);
        assert_eq!(held(engine, &client, "gold_bar"), 2);
        assert_eq!(held(engine, &client, "mithril_seed"), 40);
        assert_eq!(
            held(engine, &client, "glarials_urn_empty_waterfall_quest"),
            1
        );
        assert_eq!(
            held(engine, &client, "glarials_urn_full_waterfall_quest"),
            0
        );
        assert_eq!(
            xp(engine, &client, ATTACK) - attack,
            137_500,
            "13,750 Attack xp"
        );
        assert_eq!(
            xp(engine, &client, STRENGTH) - strength,
            137_500,
            "13,750 Strength xp"
        );
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        let t = search(
            engine,
            &mut client,
            "baxtorian_chalice_waterfall_quest",
            RAISED_CHALICE,
            6,
        );
        tells(&t, "The chalice only contains some old ashes.");
        no_errors(engine);
    });
}
