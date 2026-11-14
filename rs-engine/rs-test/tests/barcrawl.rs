//! Alfred Grimhand's Barcrawl at 647 (`quests/quest_barcrawl`; the Barbarian Outpost's guard and
//! gate in `areas/area_barbarian_outpost`; the ten bars' keepers in their areas): 289's
//! miniquest. The ten drinks and their prices are the 2011 wiki's (Bar Crawl, 2011-05-15), 208
//! coins in all; a drink's hurt is in 2011's life points. 2011's Rising Sun has three barmaids
//! who say the same (2011 wiki, Rising Sun Inn), and the Rusty Anchor's goblin gossip no longer
//! starts Goblin Diplomacy (2011 wiki, Bartender (Rusty Anchor)).

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

/// `^barcrawl_started`, `^barcrawl_complete`, and the card's ten bars (bits 3 to 12).
const STARTED: i32 = 1;
const COMPLETE: i32 = 2;
const ALL_BARS: i32 = 0x3FF << 3;
const TOO_DRUNK: i32 = 1 << 2;

/// Each bar's keeper, the bit they sign and what their drink costs (2011 wiki, Bar Crawl).
const BARS: [(&str, i32, i32); 10] = [
    ("bluemoon_bartender", 3, 50),
    ("blurberry", 4, 10),
    ("deadmans_bartender", 5, 15),
    ("dragon_bartender", 6, 12),
    ("flyinghorse_bartender", 7, 8),
    ("foresters_bartender", 8, 18),
    ("jollyboar_bartender", 9, 10),
    ("zambo", 10, 7),
    ("risingsun_barmaid", 11, 70),
    ("rustyanchor_bartender", 12, 8),
];

/// The outpost's gate (cache 786's map: barbariangater).
const GATE: (u16, u16) = (2545, 3569);

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
    set_var(engine, &client, "barcrawl", stage);
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

/// The backpack's first button op on an obj (`[if_button1,inventory:inv]` runs `opheld1`):
/// Read.
fn use_item(engine: &mut Engine, client: &mut HarnessClient, obj: &str) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let obj = id(engine, "obj", obj);
    let mut p = ClientPacket::new();
    p.p4(BACKPACK).p2_alt3(obj as i32).p2_alt1(slot);
    client.send(ClientProt::IfButton1, &p.finish());
    shown(engine, client, &[])
}

/// The spawn of that name nearest the surface world's south (a bar's keeper stands a second
/// time in a far instance square): its nid, tile and level.
fn keeper(engine: &Engine, name: &str) -> (u16, u16, u16, u8) {
    npcs(engine, name)
        .into_iter()
        .min_by_key(|&(_, _, z, _)| z)
        .unwrap_or_else(|| panic!("{name} is spawned"))
}

/// Stands the player beside a keeper and uses the card on them (`OPNPCT`: `p2_alt2(obj)
/// p2_alt2(slot) p2_alt1(nid) p4_alt3(component) p1_alt2(ctrl)`), taking `choices`; then waits
/// out the drink (its lines come three ticks apart, with no box open) and what is said after.
fn card_on(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    choices: &[&str],
) -> Conversation {
    approach(engine, client, name, choices, true)
}

/// Talk-to a keeper whose first box is a choice, from beside them.
fn talk_over_the_bar(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    choices: &[&str],
) -> Conversation {
    approach(engine, client, name, choices, false)
}

fn approach(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    choices: &[&str],
    card: bool,
) -> Conversation {
    let (nid, x, z, level) = keeper(engine, name);
    let mut all = Conversation::default();
    for attempt in 0..10usize {
        let tiles: Vec<(u16, u16)> = (1i16..=2)
            .flat_map(|r| [(r, 0), (-r, 0), (0, r), (0, -r)])
            .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
            .filter(|&(ax, az)| walkable(ax, level, az))
            .collect();
        assert!(!tiles.is_empty(), "a tile near {name}");
        let at = tiles[attempt % tiles.len()];
        engine.harness_telejump(client.pid, at.0, level, at.1);
        settle(engine, client, 2);
        if card {
            let slot = slot_of(engine, client, "barcrawl_card");
            let obj = id(engine, "obj", "barcrawl_card");
            let mut p = ClientPacket::new();
            p.p2_alt2(obj as i32)
                .p2_alt2(slot)
                .p2_alt1(nid as i32)
                .p4_alt3(BACKPACK)
                .p1_alt2(0);
            client.send(ClientProt::OpNpcT, &p.finish());
        } else {
            talk_to(client, nid);
        }
        // Straight into the talk: the Rising Sun's first box is its choice, whose options a
        // first look would swallow. Forty ticks at most when nothing opens.
        let t = converse(engine, client, choices, 40);
        assert!(t.unanswered.is_none(), "{name}: {:?}", t.unanswered);
        // Behind a counter: not every tile beside them reaches.
        if (t.texts.is_empty() && t.messages.is_empty())
            || t.messages.iter().any(|m| m == "I can't reach that!")
        {
            continue;
        }
        all = t;
        break;
    }
    for _ in 0..3 {
        let more = converse(engine, client, &[], 12);
        all.texts.extend(more.texts);
        all.messages.extend(more.messages);
    }
    engine.harness_close_modal(client.pid);
    all
}

/// Read on the card: the scroll's lines (a screen, not a talk to follow).
fn read_card(engine: &mut Engine, client: &mut HarnessClient) -> Vec<String> {
    let slot = slot_of(engine, client, "barcrawl_card");
    let obj = id(engine, "obj", "barcrawl_card");
    let mut p = ClientPacket::new();
    p.p4(BACKPACK).p2_alt3(obj as i32).p2_alt1(slot);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut lines = Vec::new();
    for _ in 0..4 {
        engine.cycle();
        lines.extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
    }
    engine.harness_close_modal(client.pid);
    lines
}

fn coins(engine: &Engine, client: &HarnessClient) -> i32 {
    held(engine, client, "coins")
}

fn give_n(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str, n: u32) {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, o, n);
}

/// 99 Hitpoints and full life points: the drinks hurt.
fn hardy(engine: &mut Engine, client: &HarnessClient) {
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.stats.levels[3] = 99;
    player.stats.base_levels[3] = 99;
    let bit0 = player.vars.get(1240).as_int() & 1;
    player.vars.set(1240, VarValue::Int((990 << 1) | bit0));
}

/// The gate is the guard's until the barcrawl is done, and he hands out the card.
#[test]
fn the_guard_hands_out_the_card() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "bc_start", 0);
        tele(engine, &mut client, (GATE.0 - 1, GATE.1), 0);
        let before = coord(engine, &client);
        let t = op_loc(
            engine,
            &mut client,
            1,
            "barbariangater",
            GATE,
            &[
                "I want to come through this gate.",
                "Looks can be deceiving, I am in fact a barbarian.",
            ],
        );
        says(
            &t,
            &[
                "Oi, whaddya want?",
                "The Alfred Grimhand Barcrawl!",
                "The guard hands you a Barcrawl card",
            ],
        );
        assert_eq!(var(engine, &client, "barcrawl"), STARTED);
        assert_eq!(held(engine, &client, "barcrawl_card"), 1);
        assert!(
            coord(engine, &client).0 <= GATE.0,
            "the gate stays shut: from {before:?} to {:?}",
            coord(engine, &client)
        );

        // The card lists the ten bars, none done.
        let lines = read_card(engine, &mut client).join(" | ");
        for line in [
            "The Official Alfred Grimhand Barcrawl!",
            "BlueMoon Inn - Not Completed...",
            "Rusty Anchor Inn - Not Completed...",
        ] {
            assert!(lines.contains(line), "289 has {line:?}; {lines}");
        }
        settle(engine, &mut client, 2);

        // Asked again before the bars are done.
        let t = talk_near(engine, &mut client, "barbguard1", &[]);
        says(
            &t,
            &[
                "So, how's the Barcrawl coming along?",
                "Well come back when you have, you lightweight.",
            ],
        );
        no_errors(engine);
    });
}

/// Each of the ten keepers serves their drink for the 2011 wiki's price and signs the card:
/// 208 coins in all. With every bar signed the card cannot be read.
#[test]
fn the_ten_bars_sign_the_card() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "bc_bars", STARTED);
        give(engine, &client, 0, "barcrawl_card");
        give_n(engine, &client, 1, "coins", 1000);
        let mut spent = 0;
        for (name, bit, price) in BARS {
            hardy(engine, &client);
            let before = coins(engine, &client);
            // The Rising Sun's barmaid asks which: the ales or the barcrawl.
            let t = card_on(
                engine,
                &mut client,
                name,
                &["I'm doing Alfred Grimhand's Barcrawl."],
            );
            let signed = var(engine, &client, "barcrawl") & (1 << bit) != 0;
            assert!(
                signed,
                "{name} signs the card: {:?} {:?}",
                t.texts, t.messages
            );
            assert_eq!(before - coins(engine, &client), price, "{name}'s drink");
            assert!(
                t.messages.len() >= 3,
                "{name}'s drink is told: {:?}",
                t.messages
            );
            spent += price;
            // Signed once: the card does nothing more here.
            let again = coins(engine, &client);
            card_on(engine, &mut client, name, &[]);
            assert_eq!(coins(engine, &client), again, "{name} is paid once");
        }
        assert_eq!(spent, 208);
        assert_eq!(var(engine, &client, "barcrawl") & ALL_BARS, ALL_BARS);

        let t = use_item(engine, &mut client, "barcrawl_card");
        tells(&t, "You are too drunk to be able to read the barcrawl card");
        assert_eq!(var(engine, &client, "barcrawl") & TOO_DRUNK, TOO_DRUNK);
        no_errors(engine);
    });
}

/// A drink is not served to a player who cannot pay.
#[test]
fn a_drink_wants_its_price() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "bc_poor", STARTED);
        give(engine, &client, 0, "barcrawl_card");
        give_n(engine, &client, 1, "coins", 49);
        let t = card_on(engine, &mut client, "bluemoon_bartender", &[]);
        says(&t, &["I don't have 50 coins."]);
        assert_eq!(coins(engine, &client), 49);
        assert_eq!(var(engine, &client, "barcrawl"), STARTED);
        no_errors(engine);
    });
}

/// The card, every bar signed, is the way through the gate.
#[test]
fn the_signed_card_opens_the_gate() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "bc_done", STARTED | ALL_BARS);
        give(engine, &client, 0, "barcrawl_card");
        let t = talk_near(engine, &mut client, "barbguard1", &[]);
        says(
            &t,
            &[
                "I tink I jusht 'bout done dem all... but I losht count...",
                "You give the card to the barbarian",
                "Yep that seems fine, you can come in now.",
            ],
        );
        assert_eq!(var(engine, &client, "barcrawl"), COMPLETE);
        assert_eq!(held(engine, &client, "barcrawl_card"), 0);

        tele(engine, &mut client, (GATE.0 - 1, GATE.1), 0);
        op_loc(engine, &mut client, 1, "barbariangater", GATE, &[]);
        settle(engine, &mut client, 3);
        assert_eq!(
            coord(engine, &client),
            (GATE.0 + 1, GATE.1),
            "through the gate"
        );
        let t = talk_near(engine, &mut client, "barbguard1", &[]);
        says(&t, &["'Ello friend."]);
        no_errors(engine);
    });
}

/// 2011: Kaylee and Tina serve as Emily does, and the Rusty Anchor's goblin gossip is only
/// gossip.
#[test]
fn the_other_barmaids_serve_and_the_gossip_starts_nothing() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "bc_2011", STARTED);
        give(engine, &client, 0, "barcrawl_card");
        give_n(engine, &client, 1, "coins", 200);
        hardy(engine, &client);
        let t = card_on(
            engine,
            &mut client,
            "risingsun_barmaid2",
            &["I'm doing Alfred Grimhand's Barcrawl."],
        );
        says(&t, &["You'll be after our Hand of Death cocktail, then."]);
        assert_eq!(coins(engine, &client), 130);
        assert_ne!(var(engine, &client, "barcrawl") & (1 << 11), 0);

        let quest = var(engine, &client, "goblinquest");
        let t = talk_over_the_bar(
            engine,
            &mut client,
            "rustyanchor_bartender",
            &["Not very busy in here today, is it?"],
        );
        says(
            &t,
            &["Knowing the goblins, it could easily turn into a full blown war."],
        );
        assert_eq!(var(engine, &client, "goblinquest"), quest);
        no_errors(engine);
    });
}
