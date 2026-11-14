//! Shades of Mort'ton at 647 (`quests/quest_mortton`, `minigames/game_mortton`): 289's quest and
//! temple. Cache 786 makes Razmire and Ulsquire multinpcs drawn by the varbits `razmire_visible`
//! and `ulsquire_visible`, and the catacombs' entrance one door; the rest stands on 289's tiles.

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

const BACKPACK: i32 = 149 << 16;
const CRAFTING: usize = 12;
const HERBLORE: usize = 15;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;
/// The quest list's rows, and Shades of Mort'ton's (`^questlist_mortton`).
const QUEST_LIST: i32 = 190 << 16 | 18;
const ROW: i32 = 96;
/// The book's Next arrow (cache 786: book_small com29, a pause button).
const NEXT_PAGE: i32 = 959 << 16 | 29;

/// `^mortton_read_diary` .. `^mortton_quest_complete`.
const READ_DIARY: i32 = 5;
const MADE_SERUM: i32 = 10;
const KILL_SHADES: i32 = 15;
const KILLED_5_SHADES: i32 = 40;
const SHADES_TO_RAZMIRE: i32 = 45;
const SHADES_TO_ULSQUIRE: i32 = 47;
const ULSQUIRE_TEMPLE: i32 = 50;
const REBUILD_TEMPLE: i32 = 55;
const CREATED_PYRE_LOGS: i32 = 70;
const LOGS_ON_PYRE: i32 = 75;
const LIT_PYRE: i32 = 80;
const COMPLETE: i32 = 85;

/// %morttonmulti's bits (289's constants; cache 786's varbits of the same names).
const USED_SERUM_ON_ULSQUIRE: u8 = 0;
const ULSQUIRE_VISIBLE: u8 = 1;
const USED_SERUM_ON_RAZMIRE: u8 = 2;
const RAZMIRE_VISIBLE: u8 = 3;
const RAZMIRE_PERM: u8 = 6;
const TABLE_SEARCHED: u8 = 8;
const UNLOCKED_SHADE_LAIR: u8 = 29;

const RAZMIRE: &str = "shades_razmire_multinpc";
const ULSQUIRE: &str = "shades_ulsquire_multinpc";

/// Cache 786's map (289's tiles, but for the door).
const SHELF: (u16, u16) = (3481, 3279);
const TABLE: (u16, u16) = (3482, 3278);
const DOOR: (u16, u16) = (3483, 3320);
const CATACOMBS: (u16, u16) = (3493, 9725);
const TEMPLE_WALL: (u16, u16) = (3504, 3315);
const PYRE: (u16, u16) = (3462, 3282);

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
    set_var(engine, &client, "morttonquest", stage);
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
    } else if op == 5 {
        p.p2_alt2(loc.int())
            .p2_alt2(x as i32)
            .p1_alt1(0)
            .p2_alt1(z as i32);
        client.send(ClientProt::OpLoc5, &p.finish());
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
    var(engine, client, "morttonquest")
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

/// Uses a held obj on an npc, from each walkable tile near it in turn until one reaches it.
fn use_near(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
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
        let slot = slot_of(engine, client, obj);
        let mut p = ClientPacket::new();
        p.p2_alt2(id(engine, "obj", obj).int())
            .p2_alt2(slot)
            .p2_alt1(nid as i32)
            .p4_alt3(BACKPACK)
            .p1_alt2(0);
        client.send(ClientProt::OpNpcT, &p.finish());
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

fn bit(engine: &Engine, client: &HarnessClient, n: u8) -> bool {
    (var(engine, client, "morttonmulti") >> n) & 1 == 1
}

fn give_n(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str, count: u32) {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, o, count);
}

/// Waits without idling out.
fn wait(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) {
    for tick in 0..ticks {
        if tick % 20 == 0 {
            client.send(ClientProt::NoTimeout, &[]);
        }
        engine.cycle();
    }
    client.packets();
}

/// No line of the talk is a script error (the harness shows one as a chat line).
fn clean(t: &Conversation) {
    assert!(
        !t.messages.iter().any(|m| m.contains("script error")),
        "{:?}",
        t.messages
    );
}

/// The shelf in Herbi Flax's house has his diary, and its last page starts the quest; the table
/// has three herbs, once.
#[test]
fn the_diary_starts_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mortdiary", 0);
        // The shelf hangs on the wall of its own tile.
        tele(engine, &mut client, SHELF, 0);
        let t = op_loc(engine, &mut client, 1, "shades_experimentshelf", SHELF, &[]);
        clean(&t);
        says(&t, &["You find an interesting looking book on the shelf."]);
        assert_eq!(held(engine, &client, "serum_book"), 1);

        // The book is not a dialogue: its box is continued, then its Next arrow pressed page by
        // page. It is read in Lumbridge: a shade's attack in Mort'ton ends a reading.
        tele(engine, &mut client, (3222, 3218), 0);
        let slot = slot_of(engine, &client, "serum_book");
        let book = id(engine, "obj", "serum_book").int();
        let mut pages: Vec<String> = Vec::new();
        for _ in 0..2 {
            let mut p = ClientPacket::new();
            p.p4(BACKPACK).p2_alt3(book).p2_alt1(slot);
            client.send(ClientProt::IfButton1, &p.finish());
            let t = watch(engine, &mut client, 2);
            clean(&t);
            pages.extend(t.texts);
            let more = converse(engine, &mut client, &[], 2);
            pages.extend(more.texts);
            for _ in 0..14 {
                let v = NEXT_PAGE;
                client.send(
                    ClientProt::ResumePauseButton,
                    &[
                        0xFF,
                        0xFF,
                        (v >> 8) as u8,
                        v as u8,
                        (v >> 24) as u8,
                        (v >> 16) as u8,
                    ],
                );
                let page = watch(engine, &mut client, 1);
                clean(&page);
                pages.extend(page.texts);
            }
            if stage(engine, &client) == READ_DIARY {
                break;
            }
        }
        assert!(
            pages
                .iter()
                .any(|p| p.contains("You prepare to read the book.")),
            "289's box"
        );
        assert!(
            pages.iter().any(|p| p.contains("Index of Solution")),
            "the last page: {:?}",
            &pages[pages.len().saturating_sub(6)..]
        );
        assert_eq!(stage(engine, &client), READ_DIARY);

        let mut t = Conversation::default();
        for at in about(TABLE, 0) {
            tele(engine, &mut client, at, 0);
            t = op_loc(engine, &mut client, 5, "shades_experimenttable", TABLE, &[]);
            if !t.messages.iter().any(|m| m == "I can't reach that!") {
                break;
            }
        }
        clean(&t);
        says(&t, &["You find a selection of herbs."]);
        assert!(bit(engine, &client, TABLE_SEARCHED));
        assert_eq!(held(engine, &client, "unidentified_tarromin"), 2);
        assert_eq!(held(engine, &client, "unidentified_rogues_purse"), 1);
        no_errors(engine);
    });
}

/// Razmire is the afflicted form until serum is used on him: the bit `razmire_visible` (which
/// 2011's client draws him by) is set for 200 ticks, or for good by the permanent serum.
#[test]
fn serum_cures_razmire_for_a_while() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mortserum", READ_DIARY);
        assert_eq!(npcs(engine, RAZMIRE).len(), 1, "one Razmire");
        let t = talk_near(engine, &mut client, RAZMIRE, &[]);
        clean(&t);
        says(&t, &["This person doesn't make any sense at all."]);

        // Somebody else's serum: a line, and he is afflicted again.
        give(engine, &client, 0, "mort_serum4");
        let t = use_near(engine, &mut client, "mort_serum4", RAZMIRE, &[]);
        clean(&t);
        says(
            &t,
            &[
                "The serum it works...but it's not too fresh",
                "This person doesn't make any sense at all.",
            ],
        );
        assert_eq!(held(engine, &client, "mort_serum3"), 1);
        assert!(!bit(engine, &client, RAZMIRE_VISIBLE));

        // The player's own.
        set_var(engine, &client, "morttonquest", MADE_SERUM);
        let t = use_near(engine, &mut client, "mort_serum3", RAZMIRE, &[]);
        clean(&t);
        says(&t, &["Ah, excellent, you've made your own serum"]);
        assert_eq!(held(engine, &client, "mort_serum2"), 1);
        assert!(bit(engine, &client, RAZMIRE_VISIBLE));
        assert!(bit(engine, &client, USED_SERUM_ON_RAZMIRE));
        // (From here his talk ends without a menu: an open menu holds the reset's queue.)
        set_var(engine, &client, "morttonquest", KILL_SHADES);
        let t = talk_near(engine, &mut client, RAZMIRE, &[]);
        clean(&t);
        says(&t, &["Ah, it's you, how's it going?"]);

        wait(engine, &mut client, 215);
        assert!(!bit(engine, &client, RAZMIRE_VISIBLE), "200 ticks on");
        let t = talk_near(engine, &mut client, RAZMIRE, &[]);
        says(&t, &["This person doesn't make any sense at all."]);

        // The permanent serum: coins, and the form stays.
        give(engine, &client, 5, "mort_serum_perm4");
        let t = use_near(engine, &mut client, "mort_serum_perm4", RAZMIRE, &[]);
        clean(&t);
        says(
            &t,
            &["Well done - I feel this serum is really working on me."],
        );
        assert!(bit(engine, &client, RAZMIRE_PERM));
        assert!(bit(engine, &client, RAZMIRE_VISIBLE));
        let coins = held(engine, &client, "coins");
        assert!((180..=220).contains(&coins), "180 to 220 coins: {coins}");
        let t = use_near(engine, &mut client, "mort_serum2", RAZMIRE, &[]);
        clean(&t);
        says(&t, &["You've already used the permanent serum on me"]);
        wait(engine, &mut client, 215);
        assert!(bit(engine, &client, RAZMIRE_VISIBLE), "still cured");
        no_errors(engine);
    });
}

/// Razmire asks for five shades killed, and takes two of the five remains.
#[test]
fn razmire_wants_five_shades() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mortshades", MADE_SERUM);
        let cured = (1 << RAZMIRE_VISIBLE) | (1 << USED_SERUM_ON_RAZMIRE);
        set_var(engine, &client, "morttonmulti", cured);
        let t = talk_near(
            engine,
            &mut client,
            RAZMIRE,
            &[
                "Is there anything worth doing around here?",
                "Yes, I'll dispatch those dark and evil creatures.",
            ],
        );
        clean(&t);
        says(&t, &["Great, that's what I wanted to hear."]);
        assert_eq!(stage(engine, &client), KILL_SHADES);

        let t = talk_near(engine, &mut client, RAZMIRE, &[]);
        says(
            &t,
            &["Hey, have you killed the five shades yet?", "No, not yet!"],
        );

        set_var(engine, &client, "morttonquest", KILLED_5_SHADES);
        for slot in 0..4 {
            give(engine, &client, slot, "shade_bones1");
        }
        let t = talk_near(engine, &mut client, RAZMIRE, &[]);
        clean(&t);
        says(&t, &["you don't have the five shade remains"]);
        assert_eq!(stage(engine, &client), KILLED_5_SHADES);

        give(engine, &client, 4, "shade_bones1");
        let t = talk_near(engine, &mut client, RAZMIRE, &[]);
        clean(&t);
        says(&t, &["Razmire takes two Shade remains."]);
        assert_eq!(stage(engine, &client), SHADES_TO_RAZMIRE);
        assert_eq!(held(engine, &client, "shade_bones1"), 3);
        no_errors(engine);
    });
}

/// Ulsquire studies a shade's remains, tells of the temple, and ends the quest once a shade is
/// laid to rest: 3 quest points, 2,000 Herblore and 2,000 Crafting experience (the 2011 wiki).
#[test]
fn ulsquire_studies_the_remains_and_ends_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mortpriest", SHADES_TO_RAZMIRE);
        assert_eq!(npcs(engine, ULSQUIRE).len(), 1, "one Ulsquire");
        let t = talk_near(engine, &mut client, ULSQUIRE, &[]);
        says(&t, &["This person doesn't make any sense at all."]);

        let cured = (1 << ULSQUIRE_VISIBLE) | (1 << USED_SERUM_ON_ULSQUIRE);
        set_var(engine, &client, "morttonmulti", cured);
        give(engine, &client, 0, "shade_bones1");
        let t = talk_near(engine, &mut client, ULSQUIRE, &[]);
        clean(&t);
        says(&t, &["You show the shade remains to the priest."]);
        assert_eq!(stage(engine, &client), SHADES_TO_ULSQUIRE);
        assert_eq!(held(engine, &client, "shade_bones1"), 0);

        let t = talk_near(
            engine,
            &mut client,
            ULSQUIRE,
            &["What can you tell me about that temple?", "Ok thanks"],
        );
        clean(&t);
        says(&t, &["How can you rebuild a totally destroyed temple?"]);
        assert_eq!(stage(engine, &client), ULSQUIRE_TEMPLE);

        set_var(engine, &client, "morttonquest", LIT_PYRE);
        let (herblore, crafting) = (xp(engine, &client, HERBLORE), xp(engine, &client, CRAFTING));
        let points = var(engine, &client, "qp");
        let t = talk_near(engine, &mut client, ULSQUIRE, &[]);
        clean(&t);
        says(
            &t,
            &[
                "I've put the Shade's spirit to rest!",
                "Great! Well done my friend!",
            ],
        );
        settle(engine, &mut client, 3);
        assert_eq!(stage(engine, &client), COMPLETE);
        assert_eq!(xp(engine, &client, HERBLORE) - herblore, 20_000);
        assert_eq!(xp(engine, &client, CRAFTING) - crafting, 20_000);
        assert_eq!(var(engine, &client, "qp") - points, 3);
        no_errors(engine);
    });
}

/// 2011's one catacomb door: locked until Razmire has told of the lair and a shade key is held.
#[test]
fn the_catacombs_door_wants_a_shade_key() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mortdoor", COMPLETE);
        let t = reach(engine, &mut client, 1, "wooden_doors_31294", DOOR, 0);
        clean(&t);
        tells(&t, "The door seems locked. Perhaps you need a key.");

        give(engine, &client, 0, "shadekey_bronze_bloodred");
        let t = reach(engine, &mut client, 1, "wooden_doors_31294", DOOR, 0);
        tells(&t, "The door seems locked. Perhaps you need a key.");

        set_var(engine, &client, "morttonmulti", 1 << UNLOCKED_SHADE_LAIR);
        // (Watched tick by tick: the catacombs' music line is not a dialogue to answer.)
        let mut t = Conversation::default();
        for at in about(DOOR, 0) {
            tele(engine, &mut client, at, 0);
            t = search(engine, &mut client, "wooden_doors_31294", DOOR, 8);
            if !t.messages.iter().any(|m| m == "I can't reach that!") {
                break;
            }
        }
        clean(&t);
        tells(&t, "You use your Shade key to unlock the door.");
        assert_eq!(coord(engine, &client), CATACOMBS);
        no_errors(engine);
    });
}

/// The catacombs' shades are 289's spawns (cache 786 has none for the square): each stands on a
/// walkable tile of 2011's map.
#[test]
fn the_catacombs_shades_stand_on_the_floor() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let _client = login(engine, "mortspawns", 0);
        let spawns = include_str!("../../../content/647/maps/n54_151.jm2");
        let mut count = 0;
        let mut off = Vec::new();
        for line in spawns.lines().filter(|l| l.contains(':')) {
            let (tile, name) = line.split_once(": ").unwrap();
            let mut parts = tile.split(' ').map(|n| n.parse::<u16>().unwrap());
            let (level, x, z) = (
                parts.next().unwrap() as u8,
                54 * 64 + parts.next().unwrap(),
                151 * 64 + parts.next().unwrap(),
            );
            if !walkable(x, level, z) {
                off.push((name, x - 54 * 64, z - 151 * 64));
            }
            count += 1;
        }
        assert!(off.is_empty(), "on the floor: {off:?}");
        assert_eq!(count, 65);
        let spawned: usize = (2..=5)
            .map(|n| npcs(engine, &format!("shadeshadow_level{n}")).len())
            .sum();
        assert_eq!(spawned, 65, "289's 65 shades");
    });
}

/// The temple's walls take a plank, a limestone brick and five swamp paste, and the first repair
/// moves the quest on.
#[test]
fn repairing_the_temple() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "morttemple", SHADES_TO_ULSQUIRE);
        let t = reach(engine, &mut client, 1, "templewall_base", TEMPLE_WALL, 0);
        clean(&t);
        says(&t, &["You're not sure how you would go about fixing this."]);

        set_var(engine, &client, "morttonquest", ULSQUIRE_TEMPLE);
        give(engine, &client, 0, "hammer");
        give(engine, &client, 1, "woodplank");
        give(engine, &client, 2, "limestonebrick");
        give_n(engine, &client, 3, "swamppaste", 5);
        // One Repair: it repeats on the wall's hidden op (289's op3=hidden) until a repair lands.
        let mut seen: Vec<String> = Vec::new();
        for at in about(TEMPLE_WALL, 0) {
            tele(engine, &mut client, at, 0);
            let t = search(engine, &mut client, "templewall_base", TEMPLE_WALL, 14);
            clean(&t);
            let unreached = t.messages.iter().any(|m| m == "I can't reach that!");
            seen.extend(t.messages);
            if !unreached {
                break;
            }
        }
        assert_eq!(stage(engine, &client), REBUILD_TEMPLE, "{seen:?}");
        assert_eq!(held(engine, &client, "woodplank"), 0);
        assert_eq!(held(engine, &client, "limestonebrick"), 0);
        assert_eq!(held(engine, &client, "swamppaste"), 0);
        no_errors(engine);
    });
}

/// Pyre logs go on a funeral pyre (the 24 pyres stand on 289's tiles), a shade's remains on the
/// logs, and a tinderbox lights it: the lighting repeats on the pyre's hidden op (289's op4).
#[test]
fn a_shade_is_burnt_on_a_pyre() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mortpyre", CREATED_PYRE_LOGS);
        give(engine, &client, 0, "logs_pyre");
        let t = use_from(engine, &mut client, "logs_pyre", "temple_pyre", PYRE, 0);
        clean(&t);
        tells(&t, "You put some logs on the pyre.");
        assert_eq!(stage(engine, &client), LOGS_ON_PYRE);
        assert_eq!(held(engine, &client, "logs_pyre"), 0);

        give(engine, &client, 1, "shade_bones1");
        give(engine, &client, 2, "tinderbox");
        let t = use_on(
            engine,
            &mut client,
            "shade_bones1",
            "temple_pyre_logs",
            PYRE,
            4,
        );
        clean(&t);
        tells(&t, "You place the shade's remains on the logs.");
        assert_eq!(held(engine, &client, "shade_bones1"), 0);
        let t = use_on(
            engine,
            &mut client,
            "tinderbox",
            "temple_pyre_bones_logs",
            PYRE,
            14,
        );
        clean(&t);
        tells(&t, "You attempt to light the funeral pyre.");
        assert_eq!(stage(engine, &client), LIT_PYRE, "{:?}", t.messages);
        no_errors(engine);
    });
}

/// View on the quest list's row opens 289's journal.
#[test]
fn the_journal() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mortjournal", 0);
        let mut p = ClientPacket::new();
        p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(ROW);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 3);
        clean(&t);
        assert!(
            t.texts.iter().any(|l| l.contains("exploring the town of")),
            "289's first line: {:?}",
            t.texts
        );
        no_errors(engine);
    });
}
