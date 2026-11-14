//! Plague City at 647 (`quests/quest_elena`, `areas/area_ardougne_west`): 289's quest on 2011's
//! map, where the sewer is in another map square and the dug hole, the rope, the grill, Edmond
//! and Elena are shown from varbits on `elenaquest_extra_bits` (cache 786). The rewards are the
//! 2011 wiki's as 289's.

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
const MINING: usize = 14;
/// The worn slot of a hat.
const HAT: u16 = 0;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `^quest_elena_started` .. `^elena_complete_read_scroll`.
const STARTED: i32 = 1;
const GASMASK: i32 = 2;
const STARTED_MUD_PATCH: i32 = 3;
const MUD_PATCH4: i32 = 7;
const OPENED_TUNNEL: i32 = 8;
const TIED_ROPE: i32 = 9;
const OPENED_PIPE: i32 = 10;
const SHOWN_PICTURE: i32 = 20;
const CURED_BRAVEK: i32 = 27;
const FREED_ELENA: i32 = 28;
const COMPLETE: i32 = 29;
const READ_SCROLL: i32 = 30;

/// Cache 786's varbits on `elenaquest_extra_bits`.
const EDMOND_BELOW: i32 = 1;
const ELENA_AT_HOME: i32 = 1 << 1;
const DUG: i32 = 1 << 2;
const ROPE_TIED: i32 = 1 << 4;
const GRILL_OFF: i32 = 2 << 4;

/// Cache 786's map.
const MUD_PATCH: (u16, u16) = (2566, 3332);
const GARDEN: (u16, u16) = (2566, 3331);
const BY_THE_PILE: (u16, u16) = (2518, 9759);
const MUD_PILE: (u16, u16) = (2519, 9759);
const PIPE: (u16, u16) = (2514, 9737);
const GRILL: (u16, u16) = (2514, 9739);
const BY_THE_PIPE: (u16, u16) = (2514, 9740);
const MANHOLE_EXIT: (u16, u16) = (2529, 3304);
const KEY_BARREL: (u16, u16) = (2534, 3268);
const STAIRS_DOWN: (u16, u16) = (2536, 3268);
const CELLAR: (u16, u16) = (2537, 9670);
const ELENAS_GATE: (u16, u16) = (2539, 9672);

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
    set_var(engine, &client, "elenaquest", stage);
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

fn extra(engine: &Engine, client: &HarnessClient) -> i32 {
    var(engine, client, "elenaquest_extra_bits")
}

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    var(engine, client, "elenaquest")
}

/// Uses the backpack's `obj` on a loc and takes the boxes that follow.
fn use_on_boxes(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    tile: (u16, u16),
) -> Conversation {
    use_on(engine, client, obj, loc, tile, 0);
    shown(engine, client, &[])
}

/// `OPLOC1` on a loc from each walkable tile about it in turn, until it is reached; then the
/// boxes that follow.
fn reach(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
) -> Conversation {
    let mut tiles: Vec<(i16, i16)> = (-2i16..=2)
        .flat_map(|dx| (-2i16..=2).map(move |dz| (dx, dz)))
        .filter(|&(dx, dz)| (dx, dz) != (0, 0))
        .collect();
    tiles.sort_by_key(|&(dx, dz)| dx.abs() + dz.abs());
    for (dx, dz) in tiles {
        let at = (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz));
        if !walkable(at.0, 0, at.1) {
            continue;
        }
        tele(engine, client, at, 0);
        let t = op_loc(engine, client, 1, loc, (x, z), &[]);
        if !t.messages.iter().any(|m| m == "I can't reach that!") {
            return t;
        }
    }
    panic!("{loc} at ({x}, {z}) was never reached");
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

/// Edmond asks for help finding Elena, and Alrena makes the gas mask from dwellberries.
#[test]
fn edmond_asks_and_alrena_makes_the_mask() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pc_start", 0);
        // Above ground until the tunnel is dug (cache 786: edmond_top shows Edmond at 0).
        let t = talk_near(
            engine,
            &mut client,
            "edmond_top",
            &["What's happened to her?", "Can I help find her?"],
        );
        says(
            &t,
            &["I've got to find my daughter.", "Okay, I'll go get some."],
        );
        assert_eq!(stage(engine, &client), STARTED);
        assert_eq!(extra(engine, &client), 0);

        give(engine, &client, 0, "dwellberries");
        let t = talk_near(engine, &mut client, "alrena", &[]);
        says(&t, &["Yes I've got some here."]);
        // She grinds the berries with no box open, then the talk goes on.
        let more = shown(engine, &mut client, &[]);
        says(&more, &["There we go, all done."]);
        assert_eq!(stage(engine, &client), GASMASK);
        assert_eq!(held(engine, &client, "gasmask"), 1);
        assert_eq!(held(engine, &client, "dwellberries"), 0);
        no_errors(engine);
    });
}

/// Four buckets of water soften the mud patch, a spade opens the tunnel, and from then the patch
/// is 2011's dug hole and Edmond is below (cache 786's varbits).
#[test]
fn the_mud_patch_is_softened_and_dug() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pc_dig", STARTED_MUD_PATCH);
        seasoned(engine, &client);
        give(engine, &client, 0, "spade");
        for slot in 1..5 {
            give(engine, &client, slot, "bucket_water");
        }
        tele(engine, &mut client, GARDEN, 0);
        let t = use_on_boxes(engine, &mut client, "spade", "plaguemudpatch2", MUD_PATCH);
        says(&t, &["The ground is rather hard."]);
        for n in 1..=4 {
            let t = use_on_boxes(
                engine,
                &mut client,
                "bucket_water",
                "plaguemudpatch2",
                MUD_PATCH,
            );
            if n < 4 {
                says(&t, &["The soil softens slightly."]);
            } else {
                says(&t, &["The soil is now soft enough to dig into."]);
            }
        }
        assert_eq!(stage(engine, &client), MUD_PATCH4);
        assert_eq!(held(engine, &client, "bucket_empty"), 4);
        assert_eq!(extra(engine, &client), 0, "no hole yet");

        let t = use_on_boxes(engine, &mut client, "spade", "plaguemudpatch2", MUD_PATCH);
        says(
            &t,
            &[
                "Suddenly it crumbles away!",
                "Edmond follows you down the hole.",
            ],
        );
        assert_eq!(stage(engine, &client), OPENED_TUNNEL);
        assert_eq!(
            coord(engine, &client),
            BY_THE_PILE,
            "in 2011's sewer, by the mud pile"
        );
        assert_eq!(
            extra(engine, &client),
            DUG | EDMOND_BELOW,
            "the dug hole, and Edmond below"
        );

        // The mud pile leads back up, and the dug hole (op Climb-down) down again.
        let t = search(engine, &mut client, "plaguemudpile", MUD_PILE, 5);
        tells(&t, "You climb up the mud pile.");
        assert_eq!(coord(engine, &client), GARDEN);
        search(engine, &mut client, "plaguemudpatch2", MUD_PATCH, 5);
        assert_eq!(coord(engine, &client), BY_THE_PILE, "down the dug hole");
        no_errors(engine);
    });
}

/// The grill is too secure alone; a rope tied to it (the rope shows, cache 786's
/// plaguecity_pipe 1) and Edmond's pull take it off (2), and the pipe leads to West Ardougne
/// for whoever wears the gas mask.
#[test]
fn the_grill_comes_off_and_the_pipe_leads_west() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pc_pipe", 0);
        seasoned(engine, &client);
        set_var(engine, &client, "elenaquest", OPENED_TUNNEL);
        set_var(engine, &client, "elenaquest_extra_bits", DUG | EDMOND_BELOW);
        give(engine, &client, 0, "rope");
        tele(engine, &mut client, BY_THE_PIPE, 0);
        let t = op_loc(engine, &mut client, 1, "plague_grill", GRILL, &[]);
        says(&t, &["The grill is too secure."]);
        let t = use_on_boxes(engine, &mut client, "rope", "plague_grill", GRILL);
        says(
            &t,
            &["You tie the end of the rope to the sewer pipe's grill"],
        );
        assert_eq!(stage(engine, &client), TIED_ROPE);
        assert_eq!(held(engine, &client, "rope"), 0);
        assert_eq!(
            extra(engine, &client),
            DUG | EDMOND_BELOW | ROPE_TIED,
            "the rope hangs from the grill"
        );

        // Edmond is below now (cache 786: edmond_bottom shows Edmond at 1).
        let t = talk_near(engine, &mut client, "edmond_bottom", &[]);
        says(
            &t,
            &[
                "I've tied the other end of this rope to the grill.",
                "That's done the job.",
            ],
        );
        assert!(
            t.texts.iter().any(|l| l == "Edmond"),
            "he speaks as Edmond: {:?}",
            t.texts
        );
        assert_eq!(stage(engine, &client), OPENED_PIPE);
        assert_eq!(
            extra(engine, &client),
            DUG | EDMOND_BELOW | GRILL_OFF,
            "the grill is gone"
        );

        // Not without the mask.
        tele(engine, &mut client, BY_THE_PIPE, 0);
        search(engine, &mut client, "plaguesewerpipe_open", PIPE, 8);
        assert!(
            coord(engine, &client).1 > 9000,
            "no way through without the mask"
        );
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        wear(engine, &client, HAT, "gasmask");
        tele(engine, &mut client, BY_THE_PIPE, 0);
        let t = search(engine, &mut client, "plaguesewerpipe_open", PIPE, 14);
        tells(&t, "You climb through the sewer pipe.");
        assert_eq!(
            coord(engine, &client),
            MANHOLE_EXIT,
            "out of the manhole in West Ardougne"
        );
        no_errors(engine);
    });
}

/// Jethick knows Elena from her picture and sends the player to the Rehnisons with a book.
#[test]
fn jethick_knows_the_picture() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pc_jethick", OPENED_PIPE);
        seasoned(engine, &client);
        give(engine, &client, 0, "elena_picture");
        let t = talk_near(
            engine,
            &mut client,
            "jethick",
            &["I'm looking for a woman from East Ardougne."],
        );
        says(
            &t,
            &[
                "Yes, a lady called Elena.",
                "Ah yes, I recognise her.",
                "I think she is staying over with the Rehnison family.",
            ],
        );
        assert_eq!(stage(engine, &client), SHOWN_PICTURE);
        assert_eq!(held(engine, &client, "turnip_book"), 1);
        no_errors(engine);
    });
}

/// The key from the barrel opens Elena's cell under the plague house; she goes home (cache
/// 786's plaguecity_elena_at_home), and Edmond rewards the player: 1 quest point, 2,425 Mining
/// experience and the Ardougne scroll (2011 wiki, as 289).
#[test]
fn elena_is_freed_and_edmond_rewards() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "pc_elena", 0);
        seasoned(engine, &client);
        set_var(engine, &client, "elenaquest", CURED_BRAVEK);
        set_var(
            engine,
            &client,
            "elenaquest_extra_bits",
            DUG | EDMOND_BELOW | GRILL_OFF,
        );
        let t = reach(engine, &mut client, "plaguekeybarrel", KEY_BARREL);
        says(&t, &["You find a small key in the barrel."]);
        assert_eq!(held(engine, &client, "elenakey"), 1);

        let t = search(engine, &mut client, "plaguehousestairsdown", STAIRS_DOWN, 6);
        tells(&t, "You walk down the stairs...");
        assert_eq!(coord(engine, &client), CELLAR);

        let t = op_loc(engine, &mut client, 1, "elenagateshut", ELENAS_GATE, &[]);
        says(&t, &["You unlock the door."]);
        settle(engine, &mut client, 4);
        let t = talk_near(engine, &mut client, "elenap", &[]);
        says(&t, &["Hi, you're free to go!", "Go and see my father,"]);
        assert_eq!(stage(engine, &client), FREED_ELENA);
        assert_eq!(
            extra(engine, &client),
            DUG | EDMOND_BELOW | GRILL_OFF | ELENA_AT_HOME,
            "Elena is home"
        );

        let mining = xp(engine, &client, MINING);
        let t = talk_near(engine, &mut client, "edmond_bottom", &[]);
        says(&t, &["Thank you, thank you!"]);
        settle(engine, &mut client, 3);
        assert_eq!(stage(engine, &client), COMPLETE);
        assert_eq!(
            xp(engine, &client, MINING) - mining,
            24_250,
            "2,425 Mining xp"
        );
        assert_eq!(held(engine, &client, "ardougnescroll"), 1);
        assert_eq!(
            held(engine, &client, "elenakey"),
            0,
            "the quest's things are taken back"
        );
        assert_eq!(
            extra(engine, &client),
            DUG | GRILL_OFF | ELENA_AT_HOME,
            "Edmond is above ground again"
        );
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        // The scroll teaches the Ardougne teleport.
        let slot = slot_of(engine, &client, "ardougnescroll");
        let scroll = id(engine, "obj", "ardougnescroll");
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(scroll as i32).p2_alt1(slot);
        client.send(ClientProt::IfButton1, &p.finish());
        shown(engine, &mut client, &[]);
        assert_eq!(stage(engine, &client), READ_SCROLL);
        assert_eq!(held(engine, &client, "ardougnescroll"), 0);
        no_errors(engine);
    });
}
