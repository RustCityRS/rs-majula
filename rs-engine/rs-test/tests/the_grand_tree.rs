//! The Grand Tree at 647 (`quests/quest_grandtree`, and the gnome glider, gate and Hazelmere of
//! `areas/area_gnome`): 289's quest on 289's tiles. Cache 786 makes Hazelmere a multinpc, the
//! roots two new locs, and 289's one glider pilot five captains; its glider map plays 289's
//! journeys. The rewards are the 2011 wiki's, as 289's.

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

const ATTACK: usize = 0;
const MAGIC: usize = 6;
const AGILITY: usize = 16;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;
/// The quest list's rows, and The Grand Tree's (`^questlist_grandtree`).
const QUEST_LIST: i32 = 190 << 16 | 18;
const ROW: i32 = 56;
/// The glider map's mountains button (cache 786: glidermap com24).
const MOUNTAINS_BUTTON: i32 = 138 << 16 | 24;

/// `^grandtree_started` .. `^grandtree_complete`.
const STARTED: i32 = 10;
const SPOKEN_HAZELMERE: i32 = 20;
const RELAYED_MESSAGE: i32 = 30;
const RELEASED_PRISON: i32 = 80;
const OBTAINED_LUMBER_ORDER: i32 = 90;
const SEARCHING_DACONIA: i32 = 150;
const COMPLETE: i32 = 160;

/// Cache 786's spawns (289's tiles).
const KING: (u16, u16) = (2466, 3497);
const KING_BELOW: (u16, u16) = (2464, 9897);
const HAZELMERE: (u16, u16) = (2678, 3086);
const ERRDO: (u16, u16) = (2465, 3501);
const FOREMAN: (u16, u16) = (2999, 3043);
/// Where the glider comes down on Karamja, and the mountains' landing (`^sindarpos`).
const CRASH: (u16, u16) = (2917, 3058);
const SINDARPOS: (u16, u16) = (2850, 3497);
/// The third and the second root of 289's `daconia_coords`.
const ROOT_3: (u16, u16) = (2455, 9874);
const ROOT_2: (u16, u16) = (2457, 9881);

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
    set_var(engine, &client, "grandtree", stage);
    seasoned(engine, &client);
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
    by: (u16, u16),
    choices: &[&str],
) -> Conversation {
    let mut last = Conversation::default();
    for attempt in 0..12usize {
        let (nid, x, z, level) = npcs(engine, npc)
            .into_iter()
            .min_by_key(|n| n.1.abs_diff(by.0) + n.2.abs_diff(by.1))
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
    var(engine, client, "grandtree")
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

/// The npc's third op (2011's Glider), from each walkable tile near it in turn until one reaches it.
fn glider_op_near(
    engine: &mut Engine,
    client: &mut HarnessClient,
    npc: &str,
    by: (u16, u16),
    choices: &[&str],
) -> Conversation {
    let mut last = Conversation::default();
    for attempt in 0..12usize {
        let (nid, x, z, level) = npcs(engine, npc)
            .into_iter()
            .min_by_key(|n| n.1.abs_diff(by.0) + n.2.abs_diff(by.1))
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
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt2(nid as i32);
        client.send(ClientProt::OpNpc3, &p.finish());
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
            first.extend(more.texts);
            last.texts = first;
            last.messages.extend(more.messages);
            return last;
        }
    }
    panic!("{npc} was never reached: {:?}", last.messages);
}

/// A talk that closed its box to walk or fly somewhere goes on: waits up to `wait` ticks for
/// the next box, answers on from there, and adds what was said to `t`.
fn goes_on(
    engine: &mut Engine,
    client: &mut HarnessClient,
    t: &mut Conversation,
    choices: &[&str],
    wait: usize,
) {
    for _ in 0..wait {
        engine.cycle();
        let packets = client.packets();
        let texts: Vec<String> = settexts(&packets)
            .into_iter()
            .map(|(_, text)| text)
            .collect();
        t.messages.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
        if !texts.is_empty() {
            t.texts.extend(texts);
            let more = converse(engine, client, choices, 400);
            t.texts.extend(more.texts);
            t.messages.extend(more.messages);
            return;
        }
    }
}

/// No line of the talk is a script error (the harness shows one as a chat line).
fn clean(t: &Conversation) {
    assert!(
        !t.messages.iter().any(|m| m.contains("script error")),
        "{:?}",
        t.messages
    );
}

/// King Narnode takes the player below the tree and asks for help: a bark sample and a
/// translation book for Hazelmere.
#[test]
fn king_narnode_asks_for_help() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "gtstart", 0);
        let mut t = talk_near(
            engine,
            &mut client,
            "grandtree_narnode",
            KING,
            &["You seem worried, what's up?"],
        );
        // He walks to the trapdoor and the talk goes on below.
        goes_on(engine, &mut client, &mut t, &["I'd be happy to help!"], 80);
        clean(&t);
        says(
            &t,
            &[
                "Not here, follow me.",
                "The king has given you a sample of bark.",
                "The king has given you a translation book.",
            ],
        );
        assert_eq!(stage(engine, &client), STARTED);
        assert_eq!(held(engine, &client, "grandtree_barksample"), 1);
        assert_eq!(held(engine, &client, "grandtree_translationbook"), 1);
        no_errors(engine);
    });
}

/// Hazelmere (2011's multinpc, speaking as Hazelmere) takes the bark sample and writes his
/// answer on a scroll.
#[test]
fn hazelmere_writes_his_answer_down() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "gtmage", STARTED);
        assert_eq!(
            npcs(engine, "grandtree_hazelmere_multi").len(),
            1,
            "one Hazelmere"
        );
        let t = talk_near(
            engine,
            &mut client,
            "grandtree_hazelmere_multi",
            HAZELMERE,
            &[],
        );
        clean(&t);
        says(&t, &["You still can't understand the mage."]);
        assert_eq!(stage(engine, &client), STARTED);

        give(engine, &client, 0, "grandtree_barksample");
        let t = talk_near(
            engine,
            &mut client,
            "grandtree_hazelmere_multi",
            HAZELMERE,
            &[],
        );
        clean(&t);
        says(
            &t,
            &[
                "Blah. Blah, blah, blah, blah...blah!",
                "You give the bark sample to Hazelmere.",
                "Hazelmere has given you the scroll.",
            ],
        );
        assert_eq!(stage(engine, &client), SPOKEN_HAZELMERE);
        assert_eq!(held(engine, &client, "grandtree_barksample"), 0);
        assert_eq!(held(engine, &client, "grandtree_scroll"), 1);
        no_errors(engine);
    });
}

/// The King is told Hazelmere's three lines (the 2011 wiki's: the first is in the third set).
#[test]
fn the_message_is_relayed_to_the_king() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "gtmessage", SPOKEN_HAZELMERE);
        let t = talk_near(
            engine,
            &mut client,
            "grandtree_narnode",
            KING,
            &[
                "I think so!",
                "None of the above.",
                "None of the above.",
                "A man came to me with the King's seal.",
                "I gave the man Daconia rocks.",
                "And Daconia rocks will kill the tree!",
            ],
        );
        clean(&t);
        says(&t, &["Of course! I should've known!"]);
        assert_eq!(stage(engine, &client), RELAYED_MESSAGE);
        no_errors(engine);
    });
}

/// Captain Errdo (289's gnome pilot on the Grand Tree) flies the released prisoner to Karamja,
/// where the glider comes down.
#[test]
fn the_glider_comes_down_on_karamja() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "gtcrash", RELEASED_PRISON);
        let mut t = talk_near(
            engine,
            &mut client,
            "captain_errdo",
            ERRDO,
            &["Take me to Karamja please!"],
        );
        // The flight, and the talk goes on where the glider comes down.
        goes_on(engine, &mut client, &mut t, &[], 30);
        clean(&t);
        says(
            &t,
            &[
                "Hi, the King said that you need to leave?",
                "Hold on tight, it'll be a rough ride!",
                "Sorry about that.",
            ],
        );
        assert_eq!(coord(engine, &client), CRASH);
        no_errors(engine);
    });
}

/// The shipyard's foreman hands over Glough's lumber order to one who knows Glough.
#[test]
fn the_foreman_hands_over_the_lumber_order() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "gtforeman", RELEASED_PRISON);
        let mut t = talk_near(engine, &mut client, "grandtree_foreman", FOREMAN, &[]);
        // He leads the way to his office and the talk goes on there.
        goes_on(
            engine,
            &mut client,
            &mut t,
            &[
                "Sadly his wife is no longer with us!",
                "He loves worm holes.",
                "Anita.",
            ],
            60,
        );
        clean(&t);
        says(
            &t,
            &[
                "Follow me.",
                "Well, well, you do know Glough.",
                "The foreman has given you the lumber order.",
            ],
        );
        assert_eq!(stage(engine, &client), OBTAINED_LUMBER_ORDER);
        assert_eq!(held(engine, &client, "grandtree_order"), 1);
        no_errors(engine);
    });
}

/// The last Daconia rock is under the root the King's talk picked (2011's root locs on 289's
/// tiles), and the King ends the quest for it: 5 quest points, 7,900 Agility, 18,400 Attack and
/// 2,150 Magic experience (the 2011 wiki).
#[test]
fn the_daconia_rock_is_found_and_the_quest_ends() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "gtroots", SEARCHING_DACONIA);
        set_var(engine, &client, "daconia_rock_root", 3);
        let t = reach(engine, &mut client, 1, "di2_largeroot_gnome", ROOT_2, 0);
        clean(&t);
        assert_eq!(
            held(engine, &client, "grandtree_daconiarock"),
            0,
            "not this root"
        );
        let t = reach(engine, &mut client, 1, "di2_largeroot_gnome", ROOT_3, 0);
        clean(&t);
        says(&t, &["You've found a Daconia Rock!"]);
        assert_eq!(held(engine, &client, "grandtree_daconiarock"), 1);

        let (agility, attack, magic) = (
            xp(engine, &client, AGILITY),
            xp(engine, &client, ATTACK),
            xp(engine, &client, MAGIC),
        );
        let points = var(engine, &client, "qp");
        let t = talk_near(engine, &mut client, "grandtree_narnode", KING_BELOW, &[]);
        clean(&t);
        says(&t, &["All the best Traveller and thanks again!"]);
        settle(engine, &mut client, 3);
        assert_eq!(stage(engine, &client), COMPLETE);
        assert_eq!(held(engine, &client, "grandtree_daconiarock"), 0);
        assert_eq!(xp(engine, &client, AGILITY) - agility, 79_000);
        assert_eq!(xp(engine, &client, ATTACK) - attack, 184_000);
        assert_eq!(xp(engine, &client, MAGIC) - magic, 21_500);
        assert_eq!(var(engine, &client, "qp") - points, 5);
        no_errors(engine);
    });
}

/// After the quest the captains fly: 2011's Glider op opens the map, and its mountains button
/// lands by the White Wolf Mountain's glider.
#[test]
fn the_glider_flies_after_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "gtglider", COMPLETE);
        let t = glider_op_near(engine, &mut client, "captain_errdo", ERRDO, &[]);
        clean(&t);
        says(&t, &["Can you take me on the glider?", "Of course!"]);

        let mut p = ClientPacket::new();
        p.p4(MOUNTAINS_BUTTON).p2_alt3(-1).p2_alt1(-1);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 8);
        clean(&t);
        let (x, z) = coord(engine, &client);
        assert!(
            x.abs_diff(SINDARPOS.0) <= 1 && z.abs_diff(SINDARPOS.1) <= 1,
            "landed by the mountains' glider: ({x}, {z})"
        );
        assert_eq!(
            var(engine, &client, "pilot_journey"),
            -1,
            "the journey is over"
        );
        no_errors(engine);
    });
}

/// View on the quest list's row opens 289's journal.
#[test]
fn the_journal() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "gtjournal", STARTED);
        let mut p = ClientPacket::new();
        p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(ROW);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 3);
        clean(&t);
        assert!(
            t.texts.iter().any(|l| l.contains("Hazelmere")),
            "289's journal at the first stage: {:?}",
            t.texts
        );
        no_errors(engine);
    });
}
