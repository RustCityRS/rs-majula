//! Priest in Peril at 647 (`quests/quest_priestperil`, `areas/area_mausoleum`, King Roald in
//! `areas/area_varrock`): 289's quest in 2011's rebuilt temple (cache 786's map: other doors,
//! another coffin, the cell door moved). The rewards are the 2011 wiki's and Sal's Realm's as
//! 289's; the monuments' holy power hurts in 2011's life points.

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
const PRAYER: usize = 5;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `^priestperil_started` .. `^priestperil_end_bring_essence`.
const STARTED: i32 = 1;
const AGREE_TO_KILL_DOG: i32 = 2;
const KILLED_DOG: i32 = 3;
const RETURN_TO_DREZEL: i32 = 4;
const FIND_DREZEL_KEY: i32 = 5;
const UNLOCKED_DREZEL: i32 = 6;
const POURED_BLESSED_WATER: i32 = 7;
const MEET_IN_MAUSOLEUM: i32 = 8;
const END_BRING_ESSENCE: i32 = 60;

/// Cache 786's map: the temple's left door, and on its top floor (level 2) the cell door and
/// the coffin; the seven monuments under the mausoleum.
const TEMPLE_DOOR: (u16, u16) = (3406, 3489);
const CELL_DOOR: (u16, u16) = (3413, 3487);
const COFFIN: (u16, u16) = (3410, 3488);
const MONUMENTS: [(&str, (u16, u16)); 7] = [
    ("priestperil_grave_base1", (3416, 9890)),
    ("priestperil_grave_base2", (3423, 9895)),
    ("priestperil_grave_base3", (3428, 9890)),
    ("priestperil_grave_base4", (3422, 9884)),
    ("priestperil_grave_base5", (3427, 9894)),
    ("priestperil_grave_base6", (3427, 9885)),
    ("priestperil_grave_base7", (3418, 9894)),
];

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
    set_var(engine, &client, "priestperil", stage);
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
    var(engine, client, "priestperil")
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

/// King Roald sends the player to the temple, where a voice behind the locked doors asks for a
/// dog to be killed.
#[test]
fn the_king_sends_the_player_to_the_temple() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pip_start", 0);
        seasoned(engine, &client);
        let t = talk_near(engine, &mut client, "king_roald", &["Sure."]);
        says(
            &t,
            &[
                "I'm looking for a quest!",
                "Sure. I don't have anything better to do right now.",
            ],
        );
        assert_eq!(stage(engine, &client), STARTED);

        // 2011's doors (large_door_30707 and 30708): locked, and knocked at.
        let t = reach(engine, &mut client, 1, "large_door_30707", TEMPLE_DOOR, 0);
        tells(&t, "This door is securely locked from inside.");
        let before = coord(engine, &client);
        let t = op_loc(
            engine,
            &mut client,
            2,
            "large_door_30707",
            TEMPLE_DOOR,
            &["Roald sent me to check on Drezel.", "Sure."],
        );
        says(
            &t,
            &[
                "Roald sent me to check on Drezel.",
                "Okey-dokey, one dead dog coming up.",
            ],
        );
        assert_eq!(stage(engine, &client), AGREE_TO_KILL_DOG);
        assert_eq!(coord(engine, &client), before, "still outside");
        no_errors(engine);
    });
}

/// The temple guardian under the mausoleum is killed for the voice at the door.
#[test]
fn the_temple_guardian_is_killed() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pip_dog", AGREE_TO_KILL_DOG);
        seasoned(engine, &client);
        let (nid, ..) = npcs(engine, "priestperilguarddog")
            .into_iter()
            .next()
            .expect("the temple guardian is spawned");
        for tick in 0..200 {
            let Some(n) = engine.get_npc_mut(nid) else {
                break;
            };
            let c = n.npc.pathing.coord;
            let stats = &mut n.npc.stats.levels;
            stats[rs_pack::types::NpcStat::Hitpoints as usize] =
                stats[rs_pack::types::NpcStat::Hitpoints as usize].min(1);
            stats[rs_pack::types::NpcStat::Defence as usize] = 1;
            if tick % 6 == 0 {
                let (px, pz) = coord(engine, &client);
                if px.abs_diff(c.x()) > 1 || pz.abs_diff(c.z()) > 1 {
                    if let Some(at) = about((c.x(), c.z()), 0).into_iter().next() {
                        engine.harness_telejump(client.pid, at.0, 0, at.1);
                    }
                }
                let mut p = ClientPacket::new();
                p.p1_alt2(0).p2_alt3(nid as i32);
                client.send(ClientProt::OpNpc2, &p.finish());
            }
            engine.cycle();
            client.packets();
            if stage(engine, &client) == KILLED_DOG {
                break;
            }
        }
        settle(engine, &mut client, 4);
        assert_eq!(
            stage(engine, &client),
            KILLED_DOG,
            "the dog's death moves the quest on"
        );
        no_errors(engine);
    });
}

/// Drezel, locked in on the top floor, tells of the river Salve; the iron key unlocks his cell,
/// blessed water quiets the vampire's coffin (2011's morytania_coffin), and he goes below.
#[test]
fn drezel_is_freed_and_the_coffin_blessed() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pip_cell", RETURN_TO_DREZEL);
        seasoned(engine, &client);
        // Talk-through is the cell door's second op.
        let mut told = Conversation::default();
        for at in about(CELL_DOOR, 2) {
            tele(engine, &mut client, at, 2);
            told = op_loc(
                engine,
                &mut client,
                2,
                "pip_prisondoor",
                CELL_DOOR,
                &["Tell me anyway.", "Yes."],
            );
            if !told.texts.is_empty() {
                break;
            }
        }
        says(
            &told,
            &["Oh! You do not appear to be one of those Zamorakians who imprisoned me here!"],
        );
        assert_eq!(stage(engine, &client), FIND_DREZEL_KEY);

        give(engine, &client, 0, "pipkey_gold");
        let t = use_from(
            engine,
            &mut client,
            "pipkey_gold",
            "pip_prisondoor",
            CELL_DOOR,
            2,
        );
        says(
            &t,
            &["The key is a similar size to the lock, but does not fit."],
        );
        give(engine, &client, 0, "pipkey_iron");
        let t = use_from(
            engine,
            &mut client,
            "pipkey_iron",
            "pip_prisondoor",
            CELL_DOOR,
            2,
        );
        says(&t, &["Oh! Thank you! You have found the key!"]);
        assert_eq!(stage(engine, &client), UNLOCKED_DREZEL);
        assert_eq!(held(engine, &client, "pipkey_iron"), 0);

        give(engine, &client, 1, "bucket_blessedwater");
        for at in about(COFFIN, 2) {
            tele(engine, &mut client, at, 2);
            let t = use_on(
                engine,
                &mut client,
                "bucket_blessedwater",
                "morytania_coffin",
                COFFIN,
                6,
            );
            if t.messages
                .iter()
                .any(|m| m == "You pour the blessed water over the coffin...")
            {
                break;
            }
        }
        assert_eq!(stage(engine, &client), POURED_BLESSED_WATER);
        assert_eq!(held(engine, &client, "bucket_empty"), 1);

        let t = talk_near(engine, &mut client, "priestperiltrappedmonk", &[]);
        assert!(!t.texts.is_empty(), "Drezel talks");
        assert_eq!(stage(engine, &client), MEET_IN_MAUSOLEUM);
        no_errors(engine);
    });
}

/// The seven monuments: a holy power hurts whoever tries to take from one (10 to 50 life
/// points, 289's 1 to 5 hitpoints), and the golden key is swapped for the iron one at the
/// monument that holds it.
#[test]
fn the_monuments_guard_the_iron_key() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pip_monuments", FIND_DREZEL_KEY);
        seasoned(engine, &client);
        let life = |engine: &Engine, client: &HarnessClient| {
            engine
                .harness_player(client.pid)
                .unwrap()
                .vars
                .get(LIFE)
                .as_int()
                >> 1
        };
        let before = life(engine, &client);
        let (first, tile) = MONUMENTS[0];
        let t = reach(engine, &mut client, 2, first, tile, 0);
        tells(&t, "A holy power prevents you stealing from the monument!");
        let lost = before - life(engine, &client);
        assert!((1..=50).contains(&lost), "the holy power: {lost}");

        // Study one (it lays the monuments out), then offer the golden key to each in turn.
        // (a plain op: the monument's screen would be taken for a box)
        search(engine, &mut client, first, tile, 6);
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        give(engine, &client, 0, "pipkey_gold");
        for (monument, tile) in MONUMENTS {
            use_from(engine, &mut client, "pipkey_gold", monument, tile, 0);
            engine.harness_close_modal(client.pid);
            settle(engine, &mut client, 1);
            if held(engine, &client, "pipkey_iron") == 1 {
                break;
            }
        }
        assert_eq!(
            held(engine, &client, "pipkey_iron"),
            1,
            "one monument holds the iron key"
        );
        assert_eq!(
            held(engine, &client, "pipkey_gold"),
            0,
            "the golden key is left in its place"
        );
        no_errors(engine);
    });
}

/// The last rune essence brought to Drezel under the mausoleum completes the quest: 1 quest
/// point, 1,406 Prayer experience and the Wolfbane dagger (2011 wiki and Sal's Realm, as 289).
#[test]
fn the_last_essence_completes_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pip_done", END_BRING_ESSENCE - 1);
        seasoned(engine, &client);
        give(engine, &client, 0, "blankrune");
        let prayer = xp(engine, &client, PRAYER);
        let t = talk_near(engine, &mut client, "drezel", &[]);
        assert!(!t.texts.is_empty(), "Drezel talks");
        settle(engine, &mut client, 3);
        assert_eq!(held(engine, &client, "blankrune"), 0);
        assert_eq!(
            held(engine, &client, "dagger_wolfbane"),
            1,
            "the Wolfbane dagger"
        );
        assert_eq!(
            xp(engine, &client, PRAYER) - prayer,
            14_060,
            "1,406 Prayer xp"
        );
        assert!(
            stage(engine, &client) >= END_BRING_ESSENCE,
            "the quest is complete"
        );
        no_errors(engine);
    });
}
