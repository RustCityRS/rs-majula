//! Merlin's Crystal at 647 (`quests/quest_arthur`; Camelot's knights in `areas/area_camelot`,
//! Arhein and the candle maker in `areas/area_catherby`, the beehives in `areas/area_seers`, the
//! Lady of the Lake in `areas/area_taverly`): 289's quest. 2011's crystal is each player's own
//! (cache 786: a multiloc on the quest's varp) and has a Smash op that wants Excalibur carried
//! (2011 wiki, Merlin's Crystal, 2011-05-16). Reward: 6 quest points.

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
const MERLINS_CRYSTAL: i32 = 72;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;
const BACKPACK: i32 = 149 << 16;

/// `^arthur_started` .. `^arthur_complete`.
const STARTED: i32 = 1;
const SPOKEN_GAWAIN: i32 = 2;
const SPOKEN_LANCELOT: i32 = 3;
const SPOKEN_MORGAN: i32 = 4;
const EXCALIBUR_BOUND: i32 = 5;
const FREED_MERLIN: i32 = 6;
const COMPLETE: i32 = 7;

/// Bits of `%excalibur_components_progress`.
const ALTAR_READ: i32 = 1 << 0;
const CANDLE_ASKED: i32 = 1 << 1;
const EXCALIBUR_STARTED: i32 = 1 << 2;
const EXCALIBUR_REWARDED: i32 = 1 << 4;

/// Cache 786's map: the crate behind Catherby's candle shop, 289's tiles out of the crates,
/// a beehive, the jeweller's door in Port Sarim, Varrock's chaos altar, the magic symbol in
/// Camelot's grounds and the crystal at the top of the south-east tower.
const CATHERBY_CRATE: (u16, u16) = (2801, 3441);
const OUT_AT_CATHERBY: (u16, u16) = (2802, 3441);
const OUT_AT_THE_KEEP: (u16, u16) = (2778, 3401);
const BEEHIVE: (u16, u16) = (2755, 3440);
const JEWELLERS_DOOR: (u16, u16) = (3016, 3246);
const CHAOS_ALTAR: (u16, u16) = (3259, 3381);
const MAGIC_SYMBOL: (u16, u16) = (2780, 3515);
const CRYSTAL: (u16, u16) = (2767, 3493);

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
    set_var(engine, &client, "arthur", stage);
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

/// Every npc of that name spawned in the quest's own lands: its nid, tile and level.
fn npcs(engine: &Engine, name: &str) -> Vec<(u16, u16, u16, u8)> {
    let want = id(engine, "npc", name);
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        // Keep Le Faye stands a second time in a far square (26_66), Mordred with it.
        .filter(|n| n.npc.uid.id() == want && n.npc.pathing.coord.x() > 2700)
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
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(MERLINS_CRYSTAL);
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

/// `OPLOCT`: a backpack item used on a loc.
fn use_on_loc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    (x, z): (u16, u16),
) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let obj = id(engine, "obj", obj);
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt2(obj as i32)
        .p2_alt2(slot)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(loc as i32);
    client.send(ClientProt::OpLocT, &p.finish());
    shown(engine, client, &[])
}

/// Drop, the backpack's eighth button op (`[if_button8,inventory:inv]` runs `opheld5`).
fn drop_item(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    choices: &[&str],
) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let obj = id(engine, "obj", obj);
    let mut p = ClientPacket::new();
    p.p4(BACKPACK).p2_alt3(obj as i32).p2_alt1(slot);
    client.send(ClientProt::IfButton8, &p.finish());
    shown(engine, client, choices)
}

/// King Arthur sets the quest; Gawain names Morgan Le Faye and Lancelot the way in by sea.
#[test]
fn king_arthur_sets_the_quest_and_his_knights_point_the_way() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mc_start", 0);
        let lines = journal(engine, &mut client);
        assert!(
            lines.contains("I can start this quest by speaking to King Arthur"),
            "{lines}"
        );

        let t = talk_near(
            engine,
            &mut client,
            "king_arthur",
            &["I want to become a Knight of the Round Table!"],
        );
        says(
            &t,
            &[
                "Welcome to my court. I am King Arthur.",
                "Unfortunately, our current quest is to rescue Merlin.",
                "Talk to my knights if you need any help.",
            ],
        );
        assert_eq!(var(engine, &client, "arthur"), STARTED);

        let t = talk_near(
            engine,
            &mut client,
            "sir_gawain",
            &[
                "Do you know how Merlin got trapped?",
                "Thank you for the information.",
            ],
        );
        says(
            &t,
            &["I would guess this is the work of the evil Morgan Le Faye!"],
        );
        assert_eq!(var(engine, &client, "arthur"), SPOKEN_GAWAIN);

        let t = talk_near(
            engine,
            &mut client,
            "sir_lancelot",
            &["Any ideas on how to get into Morgan Le Faye's stronghold?"],
        );
        says(
            &t,
            &["They get all their deliveries by boat from Catherby."],
        );
        assert_eq!(var(engine, &client, "arthur"), SPOKEN_LANCELOT);
        let lines = journal(engine, &mut client);
        assert!(
            lines.contains("Mordred's Fortress is not completely impenetrable."),
            "{lines}"
        );
        no_errors(engine);
    });
}

/// The crate behind the candle shop: nothing to hide for before Lancelot; after him it is
/// carried aboard Arhein's ship and set down at Keep Le Faye.
#[test]
fn the_crate_is_shipped_to_the_keep() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mc_crate", STARTED);
        assert!(walkable(OUT_AT_CATHERBY.0, 0, OUT_AT_CATHERBY.1));
        assert!(walkable(OUT_AT_THE_KEEP.0, 0, OUT_AT_THE_KEEP.1));
        tele(engine, &mut client, OUT_AT_CATHERBY, 0);
        let t = op_loc(
            engine,
            &mut client,
            2,
            "merlincrate_empty",
            CATHERBY_CRATE,
            &[],
        );
        tells(&t, "You have no reason to do that...");
        assert_eq!(coord(engine, &client), OUT_AT_CATHERBY);

        set_var(engine, &client, "arthur", SPOKEN_LANCELOT);
        let t = op_loc(
            engine,
            &mut client,
            2,
            "merlincrate_empty",
            CATHERBY_CRATE,
            &["Yes.", "Yes."],
        );
        says(
            &t,
            &[
                "The crate is empty. It's just about big enough to hide inside.",
                "You climb inside the crate and wait.",
                "You feel the ship start to move.",
                "You feel the crate being put down.",
                "You climb out of the crate.",
            ],
        );
        settle(engine, &mut client, 3);
        assert_eq!(coord(engine, &client), OUT_AT_THE_KEEP);
        assert_eq!(level_of(engine, &client), 0);
        no_errors(engine);
    });
}

/// Sir Mordred, beaten, is begged for by Morgan Le Faye, who tells how Merlin is freed.
#[test]
fn morgan_le_faye_buys_her_sons_life() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mc_mordred", SPOKEN_LANCELOT);
        {
            let player = engine.harness_player_mut(client.pid).unwrap();
            for skill in [0, 1, 2, 3] {
                player.stats.levels[skill] = 99;
                player.stats.base_levels[skill] = 99;
            }
            let bit0 = player.vars.get(1240).as_int() & 1;
            player.vars.set(1240, VarValue::Int((990 << 1) | bit0));
        }
        let nid = stand_near(engine, &mut client, "sir_mordred", 2);
        let level = level_of(engine, &client);
        let mut texts: Vec<String> = Vec::new();
        for tick in 0..600 {
            if let Some(n) = engine.get_npc_mut(nid) {
                // 1 life point, and defence 1 so a swing lands: the case is his death's
                // trigger, not the fight's dice.
                let stats = &mut n.npc.stats.levels;
                stats[NpcStat::Hitpoints as usize] = stats[NpcStat::Hitpoints as usize].min(1);
                stats[NpcStat::Defence as usize] = 1;
            }
            if tick % 8 == 0 {
                // He wanders and gives chase: stand beside him again before each swing.
                if let Some(n) = engine.get_npc(nid) {
                    let (x, z) = (n.npc.pathing.coord.x(), n.npc.pathing.coord.z());
                    let (px, pz) = coord(engine, &client);
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
            texts.extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
            if texts.iter().any(|t| t.contains("spare my son")) {
                break;
            }
        }
        assert!(
            texts
                .iter()
                .any(|t| t.contains("STOP! Please... spare my son.")),
            "{texts:?}"
        );
        assert_eq!(npcs(engine, "morgan_le_faye").len(), 1, "she appears");
        let t = converse(
            engine,
            &mut client,
            &[
                "Tell me how to untrap Merlin and I might.",
                "OK I will go do all that.",
            ],
            400,
        );
        assert!(t.unanswered.is_none(), "{:?}", t.unanswered);
        says(
            &t,
            &[
                "This will summon a mighty spirit named Thrantax.",
                "Morgan Le Faye vanishes.",
            ],
        );
        assert_eq!(var(engine, &client, "arthur"), SPOKEN_MORGAN);
        assert!(npcs(engine, "morgan_le_faye").is_empty(), "she vanishes");
        assert!(engine.get_npc(nid).is_some(), "Mordred lives");
        no_errors(engine);
    });
}

/// The black candle: the candle maker wants a bucket of wax, the bees want repelling first.
#[test]
fn the_candle_maker_trades_wax_for_a_black_candle() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mc_candle", SPOKEN_MORGAN);
        let t = talk_near(
            engine,
            &mut client,
            "candle_maker",
            &["Have you got any black candles?"],
        );
        says(&t, &["IF you can bring me a bucket FULL of wax."]);
        assert_eq!(
            var(engine, &client, "excalibur_components_progress"),
            CANDLE_ASKED
        );

        give(engine, &client, 0, "bucket_empty");
        give(engine, &client, 1, "insect_repellent");
        stand_by(engine, &mut client, BEEHIVE, 0);
        // Take-from before the repellent: the bees sting.
        let t = op_loc(engine, &mut client, 1, "merlin_beehive", BEEHIVE, &[]);
        tells(&t, "Suddenly bees fly out of the hive and sting you.");
        assert_eq!(held(engine, &client, "bucket_wax"), 0);
        settle(engine, &mut client, 3);

        let t = use_on_loc(
            engine,
            &mut client,
            "insect_repellent",
            "merlin_beehive",
            BEEHIVE,
        );
        says(&t, &["You pour insect repellent on the beehive."]);
        let t = op_loc(engine, &mut client, 1, "merlin_beehive", BEEHIVE, &[]);
        says(&t, &["You get some wax from the hive."]);
        assert_eq!(held(engine, &client, "bucket_wax"), 1);
        assert_eq!(held(engine, &client, "bucket_empty"), 0);

        let t = talk_near(engine, &mut client, "candle_maker", &[]);
        says(
            &t,
            &["You exchange the wax with the candle maker for a black candle."],
        );
        assert_eq!(held(engine, &client, "unlit_black_candle"), 1);
        assert_eq!(held(engine, &client, "bucket_wax"), 0);
        assert_eq!(var(engine, &client, "excalibur_components_progress"), 0);
        no_errors(engine);
    });
}

/// Excalibur: the Lady of the Lake sends the player to Port Sarim's jeweller, where a beggar
/// at the door asks for bread and turns out to be her.
#[test]
fn the_lady_of_the_lake_tests_with_a_beggar() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mc_lady", SPOKEN_MORGAN);
        let t = talk_near(
            engine,
            &mut client,
            "the_lady_of_the_lake",
            &["I seek the sword Excalibur."],
        );
        says(
            &t,
            &["Then go to the upstairs room of the jeweller's shop there."],
        );
        assert_eq!(
            var(engine, &client, "excalibur_components_progress"),
            EXCALIBUR_STARTED
        );

        give(engine, &client, 0, "bread");
        stand_by(engine, &mut client, JEWELLERS_DOOR, 0);
        let t = op_loc(
            engine,
            &mut client,
            1,
            "jewellersdoor",
            JEWELLERS_DOOR,
            &["Yes certainly."],
        );
        says(
            &t,
            &[
                "Could you find it in your heart to spare me a simple loaf of bread?",
                "You give bread to the beggar.",
                "The beggar has turned into the Lady of the Lake!",
                "Here is Excalibur, guard it well.",
            ],
        );
        assert_eq!(held(engine, &client, "excalibur"), 1);
        assert_eq!(held(engine, &client, "bread"), 0);
        assert_eq!(
            var(engine, &client, "excalibur_components_progress") & EXCALIBUR_REWARDED,
            EXCALIBUR_REWARDED
        );
        no_errors(engine);
    });
}

/// The words on Varrock's chaos altar bind Thrantax on the magic symbol; the wrong words
/// cost the candle. The bat bones are not dropped either way (2011 wiki, as 289).
#[test]
fn thrantax_is_bound_with_the_altars_words() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mc_spirit", SPOKEN_MORGAN);
        stand_by(engine, &mut client, CHAOS_ALTAR, 0);
        let t = op_loc(engine, &mut client, 2, "thrantaxaltar", CHAOS_ALTAR, &[]);
        says(&t, &["It reads: 'Snarthon Candtrick Termanto'."]);
        assert_eq!(
            var(engine, &client, "excalibur_components_progress"),
            ALTAR_READ
        );

        give(engine, &client, 0, "bat_bones");
        give(engine, &client, 1, "lit_black_candle");
        tele(engine, &mut client, MAGIC_SYMBOL, 0);
        let t = drop_item(
            engine,
            &mut client,
            "bat_bones",
            &["Snarthtrick Candanto Termon"],
        );
        says(&t, &["Suddenly a mighty spirit appears!", "GRAAAAAARGH!"]);
        assert_eq!(var(engine, &client, "arthur"), SPOKEN_MORGAN);
        assert_eq!(
            held(engine, &client, "lit_black_candle"),
            0,
            "the candle is spent"
        );
        assert_eq!(held(engine, &client, "bat_bones"), 1);
        no_errors(engine);

        // Again, a new candle and the right words. The first spirit may still be about.
        let spirits = npcs(engine, "thrantax").len();
        let mut client = login(engine, "mc_spirit2", SPOKEN_MORGAN);
        set_var(engine, &client, "excalibur_components_progress", ALTAR_READ);
        give(engine, &client, 0, "bat_bones");
        give(engine, &client, 1, "lit_black_candle");
        tele(engine, &mut client, MAGIC_SYMBOL, 0);
        let t = drop_item(
            engine,
            &mut client,
            "bat_bones",
            &["Snarthon Candtrick Termanto"],
        );
        says(
            &t,
            &[
                "Suddenly a mighty spirit appears!",
                "I wish to free Merlin from his giant crystal!",
            ],
        );
        assert_eq!(var(engine, &client, "arthur"), EXCALIBUR_BOUND);
        assert_eq!(held(engine, &client, "bat_bones"), 1, "the bones stay");
        assert_eq!(held(engine, &client, "lit_black_candle"), 1);
        assert_eq!(
            npcs(engine, "thrantax").len(),
            spirits,
            "the spirit is gone"
        );
        no_errors(engine);
    });
}

/// Smash on the crystal wants Excalibur carried and Thrantax bound; then Merlin is free and
/// King Arthur knights the player: 6 quest points.
#[test]
fn excalibur_smashes_the_crystal_and_arthur_knights_the_player() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "mc_crystal", EXCALIBUR_BOUND);
        stand_by(engine, &mut client, CRYSTAL, 2);
        // No Excalibur.
        let t = op_loc(engine, &mut client, 1, "merlins_crystal", CRYSTAL, &[]);
        tells(&t, "... but you fail to have any effect on it.");
        assert_eq!(var(engine, &client, "arthur"), EXCALIBUR_BOUND);

        // Excalibur before the spirit is bound.
        give(engine, &client, 0, "excalibur");
        set_var(engine, &client, "arthur", SPOKEN_MORGAN);
        let t = op_loc(engine, &mut client, 1, "merlins_crystal", CRYSTAL, &[]);
        tells(&t, "... but it seems to be protected by a dark force.");

        set_var(engine, &client, "arthur", EXCALIBUR_BOUND);
        let t = op_loc(engine, &mut client, 1, "merlins_crystal", CRYSTAL, &[]);
        tells(&t, "... and it shatters under the force of Excalibur!");
        says(
            &t,
            &[
                "Thank you! Thank you! Thank you!",
                "You have set Merlin free. Now talk to King Arthur.",
            ],
        );
        assert_eq!(var(engine, &client, "arthur"), FREED_MERLIN);
        // The shards have no op (cache 786); Excalibur used on them tells the rest.
        let t = use_on_loc(engine, &mut client, "excalibur", "merlins_crystal", CRYSTAL);
        says(&t, &["You have already freed Merlin from the crystal."]);

        let before = var(engine, &client, "qp");
        let t = talk_near(engine, &mut client, "king_arthur", &[]);
        says(&t, &["I dub thee a Knight Of The Round Table."]);
        settle(engine, &mut client, 3);
        assert_eq!(var(engine, &client, "arthur"), COMPLETE);
        assert_eq!(var(engine, &client, "qp") - before, 6);
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        let lines = journal(engine, &mut client);
        assert!(lines.contains("QUEST COMPLETE!"), "{lines}");
        no_errors(engine);
    });
}
