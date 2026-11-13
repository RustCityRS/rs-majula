//! Sheep Herder at 647 (`quests/quest_sheepherder`): 289's quest on cache 786's enclosure,
//! where a herded sheep is the player's own: the pen's `herder_plaguesheep_N_enclosure` npcs
//! show as the sick-looking sheep while the sheep's varbit (bits of `%sheepherdervar`) is 1.
//! The 2011 wiki (Sheep Herder, 2011-04-20) gives the reward, 3100 coins and 4 quest points.

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
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const BACKPACK: i32 = 149 << 16;
/// View on the quest list's row (questlist_v4:quests) and the journal's first line (qj1).
const QUEST_LIST: i32 = 190 << 16 | 18;
const SHEEP_HERDER: i32 = 98;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;
/// The worn inventory's slots (`^wearpos_rhand`, `_torso`, `_legs`).
const RHAND: u16 = 3;
const TORSO: u16 = 4;
const LEGS: u16 = 7;

/// `^sheepherder_tasked_with_talking_to_dr_orbon` .. `^sheepherder_complete`.
const SENT_TO_ORBON: i32 = 1;
const HERDING: i32 = 2;
const COMPLETE: i32 = 3;

/// maps/l40_52.jm2: the gate's north leaf, and the tile east of it inside the enclosure.
const GATE: (u16, u16) = (2594, 3362);
/// 289's sheepherder_pen_gate: where a prodded sheep jumps the gate.
const GATE_ZONE: ((u16, u16), (u16, u16)) = ((2592, 3360), (2594, 3363));
/// 289's sheepherder_in_pen.
const PEN_ZONE: ((u16, u16), (u16, u16)) = ((2595, 3351), (2609, 3364));
const INCINERATOR: (u16, u16) = (2606, 3359);
/// maps/o40_52.jm2.
const PROD: (u16, u16) = (2604, 3357);
/// maps/n40_52.jm2: where cache 786 stands each sheep's enclosure npc.
const PEN: [(u16, u16); 4] = [(2597, 3362), (2598, 3361), (2597, 3360), (2596, 3359)];

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

/// The sheep's three bits of %sheepherdervar (sheep 1 is bits 1-3).
fn sheep(engine: &Engine, client: &HarnessClient, n: usize) -> i32 {
    (var(engine, client, "sheepherdervar") >> (1 + 3 * (n - 1))) & 7
}

fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "sheepherderquest", stage);
    client
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

/// The plague suit on and the prod in hand.
fn dressed(engine: &mut Engine, client: &HarnessClient) {
    wear(engine, client, TORSO, "plague_jacket");
    wear(engine, client, LEGS, "plague_trousers");
    wear(engine, client, RHAND, "cattleprod");
}

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

/// Talks to the npc of that name, taking `choices`.
fn talk(
    engine: &mut Engine,
    client: &mut HarnessClient,
    npc: &str,
    choices: &[&str],
) -> Conversation {
    let (nid, x, z) = npcs(engine, npc)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{npc} is spawned"));
    // A tile beside the npc with no wall between them.
    let at = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dz)| {
            (
                x.wrapping_add_signed(dx as i16),
                z.wrapping_add_signed(dz as i16),
                dx,
                dz,
            )
        })
        .find(|&(ax, az, dx, dz)| {
            walkable(ax, 0, az)
                && rsmod::can_travel(0, ax, az, -dx, -dz, 1, 0, CollisionType::Normal)
        })
        .map(|(ax, az, ..)| (ax, az))
        .expect("a tile beside the npc");
    tele(engine, client, at);
    talk_to(client, nid);
    let talk = converse(engine, client, choices, 300);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    talk
}

/// Prod (`OPNPC1`) on a sheep, and what it shows.
fn prod(engine: &mut Engine, client: &mut HarnessClient, nid: u16) -> Conversation {
    talk_to(client, nid);
    converse(engine, client, &[], 6)
}

/// Take (`OPOBJ3`) on a ground obj.
fn take(engine: &mut Engine, client: &mut HarnessClient, obj: &str, (x, z): (u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt1(id(engine, "obj", obj) as i32)
        .p2_alt2(z as i32)
        .p1(0)
        .p2(x as i32);
    client.send(ClientProt::OpObj3, &p.finish());
    settle(engine, client, 10);
}

/// The journal's lines once View is clicked on the quest's row, their colour tags taken off.
fn journal(engine: &mut Engine, client: &mut HarnessClient) -> String {
    let mut p = ClientPacket::new();
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(SHEEP_HERDER);
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

type Tile = (u16, u16);
type Step = (i8, i8);

fn in_gate_zone((x, z): (u16, u16)) -> bool {
    let ((x0, z0), (x1, z1)) = GATE_ZONE;
    (x0..=x1).contains(&x) && (z0..=z1).contains(&z)
}

/// The prods that take a sheep from `from` to the gate: each step one tile along an axis the
/// sheep can walk, with a walkable tile behind it for the player to prod from.
fn route(from: (u16, u16)) -> Option<Vec<(i8, i8)>> {
    let mut came: HashMap<Tile, (Tile, Step)> = HashMap::new();
    let mut queue = VecDeque::from([from]);
    let mut end = None;
    while let Some(at) = queue.pop_front() {
        if in_gate_zone(at) {
            end = Some(at);
            break;
        }
        for (dx, dz) in [(-1i8, 0i8), (0, 1), (0, -1), (1, 0)] {
            let next = (
                at.0.wrapping_add_signed(dx as i16),
                at.1.wrapping_add_signed(dz as i16),
            );
            let behind = (
                at.0.wrapping_add_signed(-dx as i16),
                at.1.wrapping_add_signed(-dz as i16),
            );
            if next.0.abs_diff(from.0) > 60 || next.1.abs_diff(from.1) > 60 {
                continue;
            }
            // Not through 289's sheepherder_in_pen: a sheep there is not prodded.
            if (PEN_ZONE.0.0..=PEN_ZONE.1.0).contains(&next.0)
                && (PEN_ZONE.0.1..=PEN_ZONE.1.1).contains(&next.1)
            {
                continue;
            }
            if next == from || came.contains_key(&next) || !walkable(behind.0, 0, behind.1) {
                continue;
            }
            if !rsmod::can_travel(0, at.0, at.1, dx, dz, 1, 0, CollisionType::Normal) {
                continue;
            }
            came.insert(next, (at, (dx, dz)));
            queue.push_back(next);
        }
    }
    let mut at = end?;
    let mut steps = Vec::new();
    while at != from {
        let (prev, step) = came[&at];
        steps.push(step);
        at = prev;
    }
    steps.reverse();
    Some(steps)
}

fn npc_at(engine: &Engine, nid: u16) -> Option<(u16, u16)> {
    engine
        .get_npc(nid)
        .map(|n| (n.npc.pathing.coord.x(), n.npc.pathing.coord.z()))
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

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

fn tele(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16)) {
    assert!(walkable(x, 0, z), "({x}, {z}) is walkable");
    engine.harness_telejump(client.pid, x, 0, z);
    settle(engine, client, 3);
}

/// Every spawned npc of that name: its nid and tile.
fn npcs(engine: &Engine, name: &str) -> Vec<(u16, u16, u16)> {
    let want = id(engine, "npc", name);
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == want)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .collect()
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

/// `OPLOC1`: `p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`, and what it shows.
fn op_loc1(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
    converse(engine, client, &[], 12)
}

fn slot_of(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let bag = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap();
    (0..28)
        .find(|&slot| bag.has_at(slot, o))
        .unwrap_or_else(|| panic!("{name} is held")) as i32
}

/// An item used on a loc (`OPLOCT`), and what it shows.
fn use_on_loc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    (x, z): (u16, u16),
) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let mut p = ClientPacket::new();
    p.p2_alt2(id(engine, "obj", obj) as i32)
        .p2_alt2(slot)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(id(engine, "loc", loc) as i32);
    client.send(ClientProt::OpLocT, &p.finish());
    converse(engine, client, &[], 12)
}

/// An item used on an npc (`OPNPCT`), and what it shows.
fn use_on_npc(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    nid: u16,
) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let mut p = ClientPacket::new();
    p.p2_alt2(id(engine, "obj", obj) as i32)
        .p2_alt2(slot)
        .p2_alt1(nid as i32)
        .p4_alt3(BACKPACK)
        .p1_alt2(0);
    client.send(ClientProt::OpNpcT, &p.finish());
    converse(engine, client, &[], 12)
}

/// Councillor Halgrive gives the job and the feed, and Doctor Orbon sells the suit for 100
/// coins, with 289's lines.
#[test]
fn halgrive_gives_the_job_and_orbon_sells_the_suit() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "sh_start", 0);
        let lines = journal(engine, &mut client);
        assert!(
            lines.contains("I can start this quest by speaking to Councillor Halgrive"),
            "{lines}"
        );

        let t = talk(
            engine,
            &mut client,
            "councillor_halgrive",
            &["That's life for you."],
        );
        says(&t, &["I've been better.", "That's what concerns me..."]);
        let t = talk(
            engine,
            &mut client,
            "councillor_halgrive",
            &["What's wrong?", "That's not a job for me."],
        );
        says(&t, &["They believe that the sheep have"]);
        assert_eq!(var(engine, &client, "sheepherderquest"), 0);

        let t = talk(
            engine,
            &mut client,
            "councillor_halgrive",
            &["What's wrong?", "I can do that for you."],
        );
        says(
            &t,
            &[
                "The councillor gives you some poisoned sheep feed.",
                "How will I know which sheep are infected?",
            ],
        );
        assert_eq!(var(engine, &client, "sheepherderquest"), SENT_TO_ORBON);
        assert_eq!(held(engine, &client, "poisoned_feed"), 1);

        // The feed lost, he gives more.
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let feed = id(engine, "obj", "poisoned_feed");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .delete(feed, 1);
        let t = talk(engine, &mut client, "councillor_halgrive", &[]);
        says(
            &t,
            &[
                "Some more sheep poison would be appreciated...",
                "The councillor gives you some more poisoned sheep feed.",
            ],
        );
        assert_eq!(held(engine, &client, "poisoned_feed"), 1);

        // Without the coins the doctor keeps his suit.
        let t = talk(engine, &mut client, "doctor_orbon", &["Ok, I'll take it."]);
        says(&t, &["Oops, sorry, I don't have enough money."]);
        assert_eq!(var(engine, &client, "sheepherderquest"), SENT_TO_ORBON);

        let coins = id(engine, "obj", "coins");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .set(5, coins, 150);
        let t = talk(engine, &mut client, "doctor_orbon", &["Ok, I'll take it."]);
        says(
            &t,
            &[
                "You give Doctor Orbon 100 coins.",
                "These should protect you from infection.",
            ],
        );
        assert_eq!(var(engine, &client, "sheepherderquest"), HERDING);
        assert_eq!(held(engine, &client, "coins"), 50);
        assert_eq!(held(engine, &client, "plague_jacket"), 1);
        assert_eq!(held(engine, &client, "plague_trousers"), 1);

        let lines = journal(engine, &mut client);
        for line in [
            "I bought some protective clothing from Dr Orbon",
            "I must find the first sheep and herd it to the pen.",
            "I must find the fourth sheep and herd it to the pen.",
        ] {
            assert!(lines.contains(line), "{line:?}: {lines}");
        }
        no_errors(engine);
    });
}

/// The enclosure's gate lets in a player on the quest in the plague suit, and lets anyone
/// out. Cache 786 has the gate where 289 does: the enclosure is east of it.
#[test]
fn the_gate_wants_the_plague_suit() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let inside = (GATE.0 + 1, GATE.1);
        let mut client = login(engine, "sh_gate", 0);
        tele(engine, &mut client, GATE);
        let t = op_loc1(engine, &mut client, "plaguesheep_gatel", GATE);
        says(&t, &["This is a restricted area."]);
        assert_eq!(coord(engine, &client), GATE);

        set_var(engine, &client, "sheepherderquest", HERDING);
        let t = op_loc1(engine, &mut client, "plaguesheep_gatel", GATE);
        says(
            &t,
            &[
                "It doesn't look very safe in there. I'm not going in without decent protective clothing.",
            ],
        );
        assert_eq!(coord(engine, &client), GATE);

        wear(engine, &client, TORSO, "plague_jacket");
        let t = op_loc1(engine, &mut client, "plaguesheep_gatel", GATE);
        assert_eq!(
            coord(engine, &client),
            GATE,
            "the jacket alone: {:?}",
            t.texts
        );

        wear(engine, &client, LEGS, "plague_trousers");
        op_loc1(engine, &mut client, "plaguesheep_gatel", GATE);
        assert_eq!(coord(engine, &client), inside, "let in");
        settle(engine, &mut client, 6);

        // The prod lies where 289 has it, by the incinerator.
        assert!(walkable(PROD.0, 0, PROD.1), "the prod's tile");
        take(engine, &mut client, "cattleprod", PROD);
        assert_eq!(held(engine, &client, "cattleprod"), 1, "the prod is taken");

        // Out again, by the south leaf.
        let south = (GATE.0, GATE.1 - 1);
        tele(engine, &mut client, (south.0 + 1, south.1));
        op_loc1(engine, &mut client, "plaguesheep_gater", south);
        assert_eq!(coord(engine, &client), south, "let out");
        settle(engine, &mut client, 6);
        no_errors(engine);
    });
}

/// A sheep is not prodded by a player off the quest, out of the suit or without the prod.
#[test]
fn a_sheep_is_prodded_in_the_suit_with_the_prod() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "sh_prod", 0);
        // A sheep of the fourth flock: the herding case takes one of the first.
        let (nid, x, z) = npcs(engine, "plaguesheep_4")[0];
        let beside = |engine: &Engine| {
            let (x, z) = npc_at(engine, nid).unwrap_or((x, z));
            [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
                .find(|&(x, z)| walkable(x, 0, z))
                .expect("a tile beside the sheep")
        };
        let at = beside(engine);
        tele(engine, &mut client, at);
        let t = prod(engine, &mut client, nid);
        tells(
            &t,
            "The sheep looks extremely ill and you haven't any reason to interfere with it.",
        );

        set_var(engine, &client, "sheepherderquest", HERDING);
        let at = beside(engine);
        tele(engine, &mut client, at);
        let t = prod(engine, &mut client, nid);
        says(&t, &["The sheep looks extremely unwell."]);

        wear(engine, &client, TORSO, "plague_jacket");
        wear(engine, &client, LEGS, "plague_trousers");
        let at = beside(engine);
        tele(engine, &mut client, at);
        let t = prod(engine, &mut client, nid);
        says(
            &t,
            &[
                "I'm not prodding a sickly-looking sheep with my hands!",
                "I'll need to wield an appropriate tool.",
            ],
        );

        // Fed outside the enclosure, the sheep is left alone.
        give(engine, &client, 0, "poisoned_feed");
        let at = beside(engine);
        tele(engine, &mut client, at);
        let t = use_on_npc(engine, &mut client, "poisoned_feed", nid);
        says(&t, &["I don't think I should kill the sheep outside the"]);

        set_var(engine, &client, "sheepherderquest", COMPLETE);
        let at = beside(engine);
        tele(engine, &mut client, at);
        let t = prod(engine, &mut client, nid);
        says(&t, &["There's no need for you to do that."]);
        no_errors(engine);
    });
}

/// The whole of one sheep: prodded a tile at a time to the gate, where it jumps into the
/// enclosure as the player's own; fed there, it dies and leaves its bones; the bones burn in
/// the incinerator.
#[test]
fn a_sheep_is_herded_fed_and_burned() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // Each sheep's enclosure npc stands where cache 786 spawns it.
        for (n, at) in PEN.iter().enumerate() {
            let name = format!("herder_plaguesheep_{}_enclosure", n + 1);
            let spawned = npcs(engine, &name);
            assert_eq!(spawned.len(), 1, "{name}");
            assert_eq!((spawned[0].1, spawned[0].2), *at, "{name}");
        }

        let mut client = login(engine, "sh_herd", HERDING);
        dressed(engine, &client);
        give(engine, &client, 0, "poisoned_feed");

        // The first sheep, by number, that has a way to the gate from where it stands.
        let (n, nid) = (1..=4)
            .find_map(|n| {
                npcs(engine, &format!("plaguesheep_{n}"))
                    .into_iter()
                    .find(|&(_, x, z)| route((x, z)).is_some())
                    .map(|(nid, ..)| (n, nid))
            })
            .expect("a sheep with a way to the gate");
        let bones = ["sheepbonesa", "sheepbonesb", "sheepbonesc", "sheepbonesd"][n - 1];
        let nth = ["first", "second", "third", "fourth"][n - 1];
        let mut prods = 0;
        let mut jumped = false;
        // Another sheep of its flock in the way holds it up: plan again, up to 80 times.
        'herd: for _ in 0..200 {
            let from = npc_at(engine, nid).expect("the sheep");
            let Some(mut steps) = route(from) else {
                settle(engine, &mut client, 2);
                continue;
            };
            // In the gate's zone it takes one more prod, from any side.
            if steps.is_empty() {
                steps.push((0, 1));
            }
            let mut at = from;
            for (dx, dz) in steps {
                let behind = (
                    at.0.wrapping_add_signed(-dx as i16),
                    at.1.wrapping_add_signed(-dz as i16),
                );
                // Prodded every other tick: left alone, the sheep's timer turns it for home.
                engine.harness_telejump(client.pid, behind.0, 0, behind.1);
                talk_to(&mut client, nid);
                prods += 1;
                let mut said = Vec::new();
                for _ in 0..2 {
                    engine.cycle();
                    said.extend(HarnessClient::game_messages(&client.packets()));
                }
                let in_pen = npc_at(engine, nid).is_none_or(|(x, z)| {
                    (PEN_ZONE.0.0..=PEN_ZONE.1.0).contains(&x)
                        && (PEN_ZONE.0.1..=PEN_ZONE.1.1).contains(&z)
                });
                if in_pen {
                    // Over the gate: the jump takes two ticks more.
                    for _ in 0..4 {
                        engine.cycle();
                        said.extend(HarnessClient::game_messages(&client.packets()));
                    }
                }
                if said.iter().any(|(_, m)| {
                    m == "The sheep obligingly jumps over the gate and into the enclosure."
                }) {
                    jumped = true;
                    break 'herd;
                }
                let next = (
                    at.0.wrapping_add_signed(dx as i16),
                    at.1.wrapping_add_signed(dz as i16),
                );
                match npc_at(engine, nid) {
                    Some(now) if now == next => at = now,
                    // It wandered or was held up: plan again from where it is.
                    _ => continue 'herd,
                }
            }
        }
        assert!(
            jumped,
            "the sheep reached the gate and jumped ({prods} prods)"
        );
        assert!(prods > 5, "it was herded a tile at a time: {prods} prods");
        assert_eq!(sheep(engine, &client, n), 1, "herded: its own sheep shows");
        assert_eq!(sheep(engine, &client, n % 4 + 1), 0);
        let lines = journal(engine, &mut client);
        assert!(
            lines.contains(&format!(
                "I have herded the {nth} sheep to the pen. Now I must kill it safely."
            )),
            "{lines}"
        );

        // Its own sheep in the pen: prodded, it stays; fed, it dies.
        let (own, x, z) = npcs(engine, &format!("herder_plaguesheep_{n}_enclosure"))[0];
        tele(engine, &mut client, (x + 1, z));
        let t = prod(engine, &mut client, own);
        says(&t, &["The sheep is already in the enclosure."]);
        let t = use_on_npc(engine, &mut client, "poisoned_feed", own);
        says(&t, &["You feed the poisoned food to the sheep."]);
        settle(engine, &mut client, 6);
        assert_eq!(sheep(engine, &client, n), 2, "fed: its own sheep is gone");
        take(engine, &mut client, bones, (x, z));
        assert_eq!(held(engine, &client, bones), 1, "its bones");
        let lines = journal(engine, &mut client);
        assert!(
            lines.contains(&format!(
                "I have killed the {nth} sheep. Now I must incinerate its bones."
            )),
            "{lines}"
        );

        // With its bones held, another like it is not fetched.
        let t = use_on_loc(
            engine,
            &mut client,
            "poisoned_feed",
            "plaguesheep_furnace",
            INCINERATOR,
        );
        tells(&t, "Nothing interesting happens.");
        let t = use_on_loc(
            engine,
            &mut client,
            bones,
            "plaguesheep_furnace",
            INCINERATOR,
        );
        tells(&t, "You put the remains into the furnace.");
        tells(&t, "The remains burn to dust.");
        assert_eq!(held(engine, &client, bones), 0);
        assert_eq!(sheep(engine, &client, n), 6, "burned");
        let lines = journal(engine, &mut client);
        assert!(
            lines.contains(&format!(
                "I have killed the {nth} sheep and incinerated its bones."
            )),
            "{lines}"
        );
        no_errors(engine);
    });
}

/// With the four sheep burned Halgrive pays: 3100 coins and 4 quest points (2011 wiki, Sheep
/// Herder, 2011-04-20).
#[test]
fn halgrive_pays_for_the_four_sheep() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "sh_reward", HERDING);
        give(engine, &client, 0, "poisoned_feed");
        // Three burned and one only herded: not yet.
        set_var(
            engine,
            &client,
            "sheepherdervar",
            (6 << 1) | (6 << 4) | (6 << 7) | (1 << 10),
        );
        let t = talk(engine, &mut client, "councillor_halgrive", &[]);
        says(&t, &["Uh... yeah... not quite just yet..."]);
        assert_eq!(var(engine, &client, "sheepherderquest"), HERDING);

        set_var(engine, &client, "sheepherdervar", 7020);
        let lines = journal(engine, &mut client);
        assert!(
            lines.contains("I should return to Councillor Halgrive to collect the reward"),
            "{lines}"
        );
        let qp = var(engine, &client, "qp");
        let mut t = talk(engine, &mut client, "councillor_halgrive", &[]);
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            t.texts
                .extend(settexts(&packets).into_iter().map(|(_, t)| t));
        }
        says(
            &t,
            &[
                "Yes, I have.",
                "And in recognition of your service to the",
                "You have completed the Sheep Herder Quest!",
                "3100 coins",
            ],
        );
        assert_eq!(var(engine, &client, "sheepherderquest"), COMPLETE);
        assert_eq!(held(engine, &client, "coins"), 3100);
        assert_eq!(var(engine, &client, "qp") - qp, 4, "4 quest points");

        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        let lines = journal(engine, &mut client);
        assert!(lines.contains("QUEST COMPLETE!"), "{lines}");
        let t = talk(engine, &mut client, "councillor_halgrive", &[]);
        says(
            &t,
            &["Much better now I don't have to worry about those sheep."],
        );
        let t = talk(engine, &mut client, "doctor_orbon", &[]);
        says(&t, &["Well hello again."]);
        let t = talk(engine, &mut client, "farmer_brumty", &[]);
        says(
            &t,
            &["That's ok, it had to be done for the sake of the town."],
        );
        no_errors(engine);
    });
}
