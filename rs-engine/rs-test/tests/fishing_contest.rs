//! Fishing Contest at 647 (`quests/quest_fishingcompo`; Hemenster's people in
//! `areas/area_seers/scripts/hemenster`; the tunnel's dwarves in `areas/area_white_wolf_mountain`;
//! the red vines in `areas/area_seers/scripts/mcgrubors_wood.rs2`): 289's quest. Cache 786 moves
//! the contest's gate a tile and has two dwarves, Austri and Vestri; the 2011 wiki (Fishing
//! Contest, 2011-05-17) gives the reward, 1 quest point and 2,437 Fishing xp.

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
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const FISHING: usize = 10;
const BACKPACK: i32 = 149 << 16;
/// View on the quest list's row (questlist_v4:quests) and the journal's first line (qj1).
const QUEST_LIST: i32 = 190 << 16 | 18;
const FISHING_CONTEST: i32 = 47;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;

/// `^fishingcompo_started` .. `^fishingcompo_complete`.
const STARTED: i32 = 1;
const IN_COMP: i32 = 2;
const GARLIC_COMP: i32 = 3;
const WON_COMP: i32 = 4;
const COMPLETE: i32 = 5;

/// maps/l41_53.jm2 (mapsquare base 2624, 3392): the gate's south leaf. Its tile is the lane
/// outside; the contest is west of it.
const GATE: (u16, u16) = (2643, 3441);
/// The middle wall pipe.
const PIPE: (u16, u16) = (2637, 3446);
/// 289's tile for the stranger once the garlic moves him.
const STRANGER_MOVES_TO: (u16, u16) = (2631, 3435);
/// maps/l41_54.jm2: a red vine in McGrubor's Wood.
const VINE: (u16, u16) = (2631, 3498);
/// maps/l44_54.jm2 and l44_154.jm2: the tunnel's west stairs, and where 289 lands a player.
const WEST_STAIRS_TOP: (u16, u16) = (2820, 3484);
const WEST_TUNNEL: (u16, u16) = (2820, 9882);

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
    set_var(engine, &client, "fishingcompo", stage);
    client
}

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

fn tele(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16), level: u8) {
    assert!(
        walkable(x, level, z),
        "({x}, {z}) on level {level} is walkable"
    );
    engine.harness_telejump(client.pid, x, level, z);
    settle(engine, client, 3);
}

fn level_of(engine: &Engine, client: &HarnessClient) -> u8 {
    engine.harness_player(client.pid).unwrap().pathing.coord.y()
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

/// Stands the player on a tile beside the npc with no wall between them.
fn stand_by(engine: &mut Engine, client: &mut HarnessClient, npc: &str) -> u16 {
    let (nid, x, z, level) = npcs(engine, npc)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{npc} is spawned"));
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
            walkable(ax, level, az)
                && rsmod::can_travel(level, ax, az, -dx, -dz, 1, 0, CollisionType::Normal)
        })
        .map(|(ax, az, ..)| (ax, az))
        .unwrap_or_else(|| panic!("a tile beside {npc}"));
    tele(engine, client, at, level);
    nid
}

/// Talks to the npc of that name, taking `choices`.
fn talk(
    engine: &mut Engine,
    client: &mut HarnessClient,
    npc: &str,
    choices: &[&str],
) -> Conversation {
    let nid = stand_by(engine, client, npc);
    talk_to(client, nid);
    let talk = converse(engine, client, choices, 300);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    talk
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

/// Stands the player on a walkable tile next to a fishing spot (the spot is in the water).
fn stand_at_spot(engine: &mut Engine, client: &mut HarnessClient, spot: &str) -> u16 {
    let (nid, x, z, level) = npcs(engine, spot)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{spot} is spawned"));
    let at = [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(ax, az)| walkable(ax, level, az))
        .unwrap_or_else(|| panic!("a tile beside {spot}"));
    tele(engine, client, at, level);
    nid
}

/// Waits until the stranger, who wanders, is within four tiles of the player: his say in a
/// cast or in the garlic's smell reaches eight (289's npc_find).
fn stranger_near(engine: &mut Engine, client: &mut HarnessClient) {
    for tick in 0..600 {
        let (px, pz) = coord(engine, client);
        let (_, x, z, _) = npcs(engine, "sinister_stranger")[0];
        if x.abs_diff(px) <= 4 && z.abs_diff(pz) <= 4 {
            return;
        }
        if tick % 20 == 0 {
            client.send(ClientProt::NoTimeout, &[]);
        }
        engine.cycle();
        client.packets();
    }
    panic!("the stranger did not come back to the pipes");
}

/// Waits until a wandering npc is within three tiles of `at`.
fn wait_near(engine: &mut Engine, client: &mut HarnessClient, npc: &str, at: (u16, u16)) {
    for tick in 0..900 {
        let (_, x, z, _) = npcs(engine, npc)[0];
        if x.abs_diff(at.0) <= 3 && z.abs_diff(at.1) <= 3 {
            return;
        }
        if tick % 20 == 0 {
            client.send(ClientProt::NoTimeout, &[]);
        }
        engine.cycle();
        client.packets();
    }
    panic!("{npc} did not come near {at:?}");
}

/// A cast at a fishing spot (`OPNPC1`): the talk it starts, if any, and its messages, over the
/// four ticks the cast takes and what follows.
fn cast(engine: &mut Engine, client: &mut HarnessClient, spot: &str) -> Conversation {
    let nid = stand_at_spot(engine, client, spot);
    talk_to(client, nid);
    let mut t = Conversation::default();
    for _ in 0..7 {
        engine.cycle();
        let packets = client.packets();
        t.texts
            .extend(settexts(&packets).into_iter().map(|(_, t)| t));
        t.messages.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
    }
    let more = converse(engine, client, &[], 40);
    t.texts.extend(more.texts);
    t.messages.extend(more.messages);
    t
}

/// The journal's text once View is clicked on the quest's row, its colour tags taken off.
fn journal(engine: &mut Engine, client: &mut HarnessClient) -> String {
    let mut p = ClientPacket::new();
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(FISHING_CONTEST);
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

/// Vestri, the west end's dwarf in cache 786, gives the pass and starts the quest; the stairs
/// he guards are barred until the quest is done.
#[test]
fn the_dwarves_want_the_trophy() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "fc_start", 0);
        assert_eq!(
            npcs(engine, "tunnel_dwarf").len(),
            1,
            "Austri, at the east end"
        );
        assert_eq!(
            npcs(engine, "tunnel_dwarf1").len(),
            1,
            "Vestri, at the west end"
        );

        // The stairs, with the dwarf by them. He wanders, and the stairs only find him within
        // five tiles: wait for him and try again if he has strayed by the time they are clicked.
        let mut t = Conversation::default();
        let mut before = coord(engine, &client);
        for _ in 0..8 {
            wait_near(engine, &mut client, "tunnel_dwarf1", WEST_STAIRS_TOP);
            stand_by(engine, &mut client, "tunnel_dwarf1");
            before = coord(engine, &client);
            t = op_loc1(engine, &mut client, "tunnelstairstop", WEST_STAIRS_TOP);
            if t.said("Hoi there! Halt!") {
                break;
            }
        }
        says(&t, &["Hoi there! Halt!", "You can't come in here!"]);
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);
        assert_eq!(level_of(engine, &client), 0);
        assert!(coord(engine, &client).1 < 9000, "not let down: {before:?}");

        let t = talk(
            engine,
            &mut client,
            "tunnel_dwarf1",
            &[
                "I was just wondering what was down those stairs?",
                "Why not?",
                "If you were my friend I wouldn't mind.",
                "Well, let's be friends!",
                "And how am I meant to do that?",
                "Fortunately I'm alright at fishing!",
            ],
        );
        says(
            &t,
            &[
                "Hmmph. What do you want?",
                "This is the home of the Mountain Dwarves.",
                "This artefact is the first prize at the Hemenster fishing competition.",
                "Okay, I entrust you with our competition pass.",
            ],
        );
        assert_eq!(var(engine, &client, "fishingcompo"), STARTED);
        assert_eq!(held(engine, &client, "fishing_competition_pass"), 1);
        let lines = journal(engine, &mut client);
        assert!(!lines.trim().is_empty(), "the journal has the quest");

        // Austri has the same talk.
        let t = talk(engine, &mut client, "tunnel_dwarf", &[]);
        says(&t, &["Have you won yet?", "No, not yet."]);
        no_errors(engine);
    });
}

/// Morris lets a pass through the gate. Cache 786 has the gate a tile east of 289's, facing
/// the other way: its tile is outside, the contest west of it.
#[test]
fn morris_wants_the_pass_at_the_gate() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let inside = (GATE.0 - 1, GATE.1);
        let mut client = login(engine, "fc_gate", STARTED);
        tele(engine, &mut client, GATE, 0);
        let t = {
            let loc = id(engine, "loc", "fishinggateclosedl");
            let mut p = ClientPacket::new();
            p.p2_alt1(GATE.1 as i32)
                .p2_alt1(loc as i32)
                .p1(0)
                .p2_alt2(GATE.0 as i32);
            client.send(ClientProt::OpLoc1, &p.finish());
            converse(engine, &mut client, &["What do I need that for?"], 40)
        };
        says(
            &t,
            &[
                "Competition pass please.",
                "This is the entrance to the Hemenster fishing",
            ],
        );
        assert_eq!(coord(engine, &client), GATE, "kept out without the pass");

        give(engine, &client, 0, "fishing_competition_pass");
        let t = op_loc1(engine, &mut client, "fishinggateclosedl", GATE);
        says(&t, &["You show Morris your pass.", "Move on through."]);
        settle(engine, &mut client, 3);
        assert_eq!(coord(engine, &client), inside, "let in");
        settle(engine, &mut client, 6);

        // Out again, by the north leaf: no one stops a player who is not competing.
        let north = (GATE.0, GATE.1 + 1);
        tele(engine, &mut client, (north.0 - 1, north.1), 0);
        op_loc1(engine, &mut client, "fishinggateclosedl", GATE);
        settle(engine, &mut client, 3);
        assert_eq!(coord(engine, &client).0, GATE.0, "let out onto the lane");
        settle(engine, &mut client, 6);
        no_errors(engine);
    });
}

/// The red vines of McGrubor's Wood give their worms to a spade.
#[test]
fn the_red_vines_give_worms_to_a_spade() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "fc_vine", STARTED);
        let at = [(1i16, 0i16), (-1, 0), (0, 1), (0, -1), (0, 0)]
            .into_iter()
            .map(|(dx, dz)| {
                (
                    VINE.0.wrapping_add_signed(dx),
                    VINE.1.wrapping_add_signed(dz),
                )
            })
            .find(|&(x, z)| walkable(x, 0, z))
            .expect("a tile by the vine");
        tele(engine, &mut client, at, 0);
        let t = op_loc1(engine, &mut client, "red_worm_vine", VINE);
        tells(&t, "You need a spade to dig up this vine.");
        assert_eq!(held(engine, &client, "red_vine_worm"), 0);

        give(engine, &client, 0, "spade");
        let t = op_loc1(engine, &mut client, "red_worm_vine", VINE);
        tells(&t, "You find a red vine worm.");
        assert_eq!(held(engine, &client, "red_vine_worm"), 1);
        let t = use_on_loc(engine, &mut client, "spade", "red_worm_vine", VINE);
        tells(&t, "You find a red vine worm.");
        assert_eq!(held(engine, &client, "red_vine_worm"), 2);
        no_errors(engine);
    });
}

/// The contest: Bonzo takes 5 coins, garlic in the pipe moves the stranger, and three casts
/// with red vine worms at the pipes land the carp that wins the trophy. Without the garlic
/// the stranger keeps the pipes and wins.
#[test]
fn the_garlic_wins_the_contest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        assert!(
            walkable(STRANGER_MOVES_TO.0, 0, STRANGER_MOVES_TO.1),
            "the stranger's second spot"
        );
        let mut client = login(engine, "fc_contest", STARTED);
        {
            let player = engine.harness_player_mut(client.pid).unwrap();
            player.stats.levels[FISHING] = 10;
            player.stats.base_levels[FISHING] = 10;
        }
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let stock = |engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str, n: u32| {
            let o = id(engine, "obj", name);
            engine
                .harness_player_mut(client.pid)
                .unwrap()
                .invs
                .get_mut(&inv)
                .unwrap()
                .set(slot, o, n);
        };
        stock(engine, &client, 0, "coins", 10);
        stock(engine, &client, 1, "fishing_rod", 1);
        stock(engine, &client, 2, "fishing_bait", 3);

        // Casting before paying: Bonzo wants his fee.
        let t = cast(engine, &mut client, "0_41_53_compofishspot");
        says(
            &t,
            &["Hey, you need to pay to enter the competition first!"],
        );

        let t = talk(
            engine,
            &mut client,
            "bonzo",
            &["I'll enter the competition please."],
        );
        says(
            &t,
            &[
                "Only 5gp entrance fee!",
                "You pay Bonzo 5 coins.",
                "Your fishing competition spot is beside the willow tree.",
            ],
        );
        assert_eq!(held(engine, &client, "coins"), 5);
        assert_eq!(var(engine, &client, "fishingcompo"), IN_COMP);

        // The pipes are the stranger's.
        stand_at_spot(engine, &mut client, "0_41_53_sinisterfishspot");
        stranger_near(engine, &mut client);
        let t = cast(engine, &mut client, "0_41_53_sinisterfishspot");
        says(&t, &["I think you will find that is my spot."]);

        // Three casts at the willow with plain bait: shrimps, and the stranger wins.
        let mut last = Conversation::default();
        for _ in 0..3 {
            last = cast(engine, &mut client, "0_41_53_compofishspot");
        }
        assert_eq!(
            held(engine, &client, "raw_shrimp"),
            3,
            "{:?}",
            last.messages
        );
        says(
            &last,
            &["Okay folks, time's up!", "You hand over your catch."],
        );
        says(&last, &["And the winner is... The stranger in black!"]);
        assert_eq!(var(engine, &client, "fishingcompo"), STARTED);
        assert_eq!(var(engine, &client, "garlicpipe"), 0, "the fee is spent");

        // Again, with garlic in the pipe.
        stock(engine, &client, 3, "garlic", 1);
        stock(engine, &client, 4, "red_vine_worm", 3);
        let shrimp = id(engine, "obj", "raw_shrimp");
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .delete(shrimp, 3);
        talk(
            engine,
            &mut client,
            "bonzo",
            &["I'll enter the competition please."],
        );
        assert_eq!(var(engine, &client, "fishingcompo"), IN_COMP);
        assert_eq!(held(engine, &client, "coins"), 0);

        // The garlic goes in from the shore: cache 786 blocks the tiles under the pipes, so the
        // pipes answer from the one shore tile by them, two tiles from the farthest.
        let shore = (PIPE.0 + 1, PIPE.1 - 1);
        tele(engine, &mut client, shore, 0);
        let t0 = {
            let loc = id(engine, "loc", "garlicpipe");
            let mut p = ClientPacket::new();
            p.p2_alt1(PIPE.1 as i32)
                .p2_alt1(loc as i32)
                .p1(0)
                .p2_alt2((PIPE.0 - 1) as i32);
            client.send(ClientProt::OpLoc1, &p.finish());
            converse(engine, &mut client, &[], 6)
        };
        tells(&t0, "Nothing interesting happens.");
        // The smell reaches a stranger within eight tiles; he wanders, so wait for him close by.
        stranger_near(engine, &mut client);
        let mut t = use_on_loc(
            engine,
            &mut client,
            "garlic",
            "garlicpipe",
            (PIPE.0 - 1, PIPE.1),
        );
        assert_eq!(coord(engine, &client), shore, "from the shore");
        let more = converse(engine, &mut client, &[], 40);
        t.texts.extend(more.texts);
        t.messages.extend(more.messages);
        tells(&t, "You stash the garlic in the pipe.");
        says(
            &t,
            &[
                "Arrgh! WHAT is that GHASTLY smell???",
                "Your fishing competition spot is now beside the pipes.",
            ],
        );
        assert_eq!(held(engine, &client, "garlic"), 0);
        assert_eq!(var(engine, &client, "fishingcompo"), GARLIC_COMP);

        let mut last = Conversation::default();
        for _ in 0..3 {
            last = cast(engine, &mut client, "0_41_53_sinisterfishspot");
        }
        assert!(
            last.messages.iter().any(|m| m == "You catch a giant carp."),
            "{:?}",
            last.messages
        );
        says(
            &last,
            &[
                "We have a new winner!",
                "You are given the Hemenster fishing trophy!",
            ],
        );
        assert_eq!(var(engine, &client, "fishingcompo"), WON_COMP);
        assert_eq!(held(engine, &client, "hemenster_fishing_trophy"), 1);
        assert_eq!(
            held(engine, &client, "raw_giant_carp"),
            0,
            "the catch is handed over"
        );

        // The stranger goes back to the pipes for the next case.
        engine.harness_reset();
        no_errors(engine);
    });
}

/// The trophy opens the tunnel: 1 quest point and 2,437 Fishing xp (2011 wiki, Fishing
/// Contest, 2011-05-17), and the stairs go down.
#[test]
fn the_trophy_opens_the_tunnel() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "fc_reward", WON_COMP);
        give(engine, &client, 0, "hemenster_fishing_trophy");
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[FISHING];
        let qp = var(engine, &client, "qp");
        let mut t = talk(engine, &mut client, "tunnel_dwarf1", &[]);
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            t.texts
                .extend(settexts(&packets).into_iter().map(|(_, t)| t));
        }
        says(
            &t,
            &[
                "Yes I have!",
                "You give the trophy to the dwarf.",
                "Okay, we will let you in now.",
                "You have completed the Fishing Competition Quest!",
                "2,437 Fishing XP",
            ],
        );
        assert_eq!(var(engine, &client, "fishingcompo"), COMPLETE);
        assert_eq!(held(engine, &client, "hemenster_fishing_trophy"), 0);
        let gained = engine.harness_player(client.pid).unwrap().stats.xp[FISHING] - xp;
        assert_eq!(gained, 24_370, "2,437 Fishing xp (tenths)");
        assert_eq!(var(engine, &client, "qp") - qp, 1, "1 quest point");
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        // Down the west stairs and up again.
        assert!(
            walkable(WEST_TUNNEL.0, 0, WEST_TUNNEL.1),
            "the tunnel's west end"
        );
        stand_by(engine, &mut client, "tunnel_dwarf1");
        op_loc1(engine, &mut client, "tunnelstairstop", WEST_STAIRS_TOP);
        settle(engine, &mut client, 3);
        assert_eq!(coord(engine, &client), WEST_TUNNEL, "in the tunnel");
        let stairs = (WEST_STAIRS_TOP.0, WEST_STAIRS_TOP.1 - 1 + 6400);
        op_loc1(engine, &mut client, "tunnelstairs", stairs);
        settle(engine, &mut client, 3);
        let (x, z) = coord(engine, &client);
        assert!(
            z < 9000 && walkable(x, 0, z),
            "back on the mountain: ({x}, {z})"
        );

        let t = talk(engine, &mut client, "tunnel_dwarf", &[]);
        says(&t, &["Welcome, oh great Fishing Champion!"]);
        no_errors(engine);
    });
}
