//! Jungle Potion at 647 (`quests/quest_junglepotion`, Trufitus in `areas/area_karamja`): 289's
//! quest, which 2011 still has. The 2011 wiki (Jungle Potion, 2011-05-15) and a recording of
//! 24 April 2011 (YouTube -vIfGilq-sY) give the reward, 1 quest point and 775 Herblore xp, and
//! that each herb is found only once Trufitus has asked for it. Cache 786's map has the herbs'
//! spots; the cave's fungus wall is loc 32106 there, 289's id being a chair.

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

const HERBLORE: usize = 15;
/// View on the quest list's row (questlist_v4:quests) and the journal's first line (qj1).
const QUEST_LIST: i32 = 190 << 16 | 18;
const JUNGLE_POTION: i32 = 66;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;
/// Druidic Ritual complete (`^druid_complete`).
const DRUID_COMPLETE: i32 = 4;

/// The rocks over the cave (maps/l44_48.jm2) and the hand holds out of it (maps/l44_148.jm2).
const POTHOLE: (u16, u16) = (2824, 3118);
const HAND_HOLDS: (u16, u16) = (2827, 9522);

/// A herb's spot in cache 786's map: its loc, where, the stage Trufitus asks for it at, the
/// grimy herb, what the search is of, and whether the spot is left bare.
struct Spot {
    loc: &'static str,
    at: (u16, u16),
    wall: bool,
    asked: i32,
    herb: &'static str,
    of: &'static str,
    nothing: &'static str,
}

const SPOTS: [Spot; 5] = [
    Spot {
        loc: "snake_vine_full",
        at: (2757, 3041),
        wall: false,
        asked: 1,
        herb: "unidentified_snake_weed",
        of: "vine",
        nothing: "Unfortunately, you find nothing of interest.",
    },
    Spot {
        loc: "ardrigal_palm_full",
        at: (2864, 3118),
        wall: false,
        asked: 3,
        herb: "unidentified_ardrigal",
        of: "palm",
        nothing: "You find nothing of significance.",
    },
    Spot {
        loc: "sito_soil_full",
        at: (2789, 3047),
        wall: false,
        asked: 5,
        herb: "unidentified_sito_foil",
        of: "scorched earth",
        nothing: "You find nothing of significance.",
    },
    Spot {
        loc: "volencia_moss_rock_full",
        at: (2850, 3035),
        wall: false,
        asked: 7,
        herb: "unidentified_volencia_moss",
        of: "rock",
        nothing: "You find nothing of significance.",
    },
    Spot {
        loc: "di2_rogues_cave_wall_purse_full",
        at: (2831, 9490),
        wall: true,
        asked: 9,
        herb: "unidentified_rogues_purse",
        of: "wall",
        nothing: "Unfortunately, you find nothing of interest.",
    },
];

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

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    var(engine, client, "junglepotion")
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

/// A walkable tile beside (x, z) on `level`.
fn beside(x: u16, level: u8, z: u16) -> (u16, u16) {
    [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(x, z)| walkable(x, level, z))
        .unwrap_or_else(|| panic!("a walkable tile beside {x}, {z}"))
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

/// Logs in with Druidic Ritual done and Jungle Potion at `stage`.
fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "druidquest", DRUID_COMPLETE);
    set_var(engine, &client, "junglepotion", stage);
    client
}

/// Stands beside Trufitus and clicks Talk-to.
fn click(engine: &mut Engine, client: &mut HarnessClient) {
    let want = id(engine, "npc", "trufitus");
    let (nid, level, x, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == want)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.y(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .expect("Trufitus is spawned");
    let (sx, sz) = beside(x, level, z);
    engine.harness_telejump(client.pid, sx, level, sz);
    settle(engine, client, 3);
    talk_to(client, nid);
}

/// Talks to Trufitus, taking `choices`.
fn talk(engine: &mut Engine, client: &mut HarnessClient, choices: &[&str]) -> Conversation {
    click(engine, client);
    let talk = converse(engine, client, choices, 300);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    talk
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

/// `OPLOC1`: `p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`.
fn op_loc1(client: &mut HarnessClient, loc: u16, (x, z): (u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

/// `OPLOC2`: `p2_alt1(loc) p1_alt2(ctrl) p2_alt2(z) p2_alt3(x)`.
fn op_loc2(client: &mut HarnessClient, loc: u16, (x, z): (u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt1(loc as i32)
        .p1_alt2(0)
        .p2_alt2(z as i32)
        .p2_alt3(x as i32);
    client.send(ClientProt::OpLoc2, &p.finish());
}

/// Stands at the spot and searches it, answering whatever box follows.
fn search(engine: &mut Engine, client: &mut HarnessClient, spot: &Spot) -> Conversation {
    let loc = id(engine, "loc", spot.loc);
    // The fungus wall is a wall (shape 0) on a tile of rock: it too is searched from beside it.
    let (x, z) = beside(spot.at.0, 0, spot.at.1);
    engine.harness_telejump(client.pid, x, 0, z);
    settle(engine, client, 3);
    op_loc2(client, loc, spot.at);
    converse(engine, client, &[], 12)
}

/// The journal's lines once View is clicked on the quest's row, their colour tags taken off.
fn journal(engine: &mut Engine, client: &mut HarnessClient) -> Vec<String> {
    let mut p = ClientPacket::new();
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(JUNGLE_POTION);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut texts = Vec::new();
    for _ in 0..3 {
        engine.cycle();
        texts.extend(settexts(&client.packets()));
    }
    (0..10)
        .map(|line| {
            let text = texts
                .iter()
                .find(|(c, _)| *c == JOURNAL_LINE_1 + line)
                .map_or("", |(_, t)| t.as_str());
            let mut plain = String::new();
            let mut tag = false;
            for ch in text.chars() {
                match ch {
                    '<' => tag = true,
                    '>' => tag = false,
                    _ if !tag => plain.push(ch),
                    _ => {}
                }
            }
            plain
        })
        .collect()
}

const START: [&str; 5] = [
    "What does Bwana mean?",
    "Yes, of course I do.",
    "A proposition eh? Sounds interesting!",
    "Me? How can I help?",
    "It sounds like just the challenge for me.",
];

#[test]
fn trufitus_starts_the_quest_once_druidic_ritual_is_done() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "jungle_herbalist", 0);
        let lines = journal(engine, &mut client);
        assert_eq!(
            lines[..3],
            [
                "I can start this quest by speaking to Trufitus Shakaya",
                "who lives in the main hut in Tai Bwo Wannai",
                "village on the island of Karamja.",
            ]
        );

        // Without Druidic Ritual he does not start it (289).
        set_var(engine, &client, "druidquest", 0);
        let t = talk(engine, &mut client, &START);
        says(
            &t,
            &[
                "I am Trufitus Shakaya",
                "In return for this great favour I will give you training in Herblore.",
                "You do not meet all of the requirements to start",
            ],
        );
        assert_eq!(stage(engine, &client), 0, "not started");

        set_var(engine, &client, "druidquest", DRUID_COMPLETE);
        let t = talk(engine, &mut client, &START);
        says(
            &t,
            &[
                "Greetings Bwana!",
                "It means friend,",
                "My people are afraid to stay in the village.",
                "And it would make a nice break from killing things!",
                "The first herb that you need to gather is called",
                "Snake Weed.",
                "the ground turns soft and the water kisses your feet.",
            ],
        );
        assert_eq!(stage(engine, &client), 1, "sent for snake weed");

        let lines = journal(engine, &mut client);
        assert_eq!(
            lines[..4],
            [
                "I spoke to Trufitus, he needs to commune with the",
                "gods, he's asked me to help him by collecting herbs.",
                "",
                "I need to pick some fresh Snakeweed for Trufitus.",
            ]
        );
    });
}

#[test]
fn each_herb_is_found_only_once_trufitus_asks_for_it() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for spot in &SPOTS {
            // The stage before he asks: nothing to find.
            let mut client = login(engine, "jungle_herbalist", spot.asked - 1);
            let t = search(engine, &mut client, spot);
            assert!(
                t.messages.iter().any(|m| m == spot.nothing),
                "{}: {:?}",
                spot.loc,
                t.messages
            );
            assert_eq!(held(engine, &client, spot.herb), 0, "{}", spot.loc);

            let mut client = login(engine, "jungle_herbalist", spot.asked);
            let t = search(engine, &mut client, spot);
            let line = format!("You search the {}...", spot.of);
            assert!(t.messages.contains(&line), "{line:?}; {:?}", t.messages);
            assert!(t.said("You find a herb."), "{}: {:?}", spot.loc, t.texts);
            assert_eq!(held(engine, &client, spot.herb), 1, "{}", spot.loc);
            assert_eq!(stage(engine, &client), spot.asked + 1, "{} found", spot.loc);

            // 289 leaves the vine, palm, earth and rock bare for a while; the wall keeps its fungus.
            search(engine, &mut client, spot);
            let again = if spot.wall { 2 } else { 1 };
            assert_eq!(held(engine, &client, spot.herb), again, "{}", spot.loc);
        }
    });
}

#[test]
fn trufitus_takes_the_clean_herbs_he_asked_for() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // A clean herb he asked for but the player did not pick.
        let mut client = login(engine, "jungle_herbalist", 1);
        give(engine, &client, 0, "snake_weed");
        let t = talk(engine, &mut client, &["Of course!"]);
        says(
            &t,
            &[
                "Hello Bwana, do you have the Snake Weed?",
                "That's not fresh snake weed, did you pick it yourself?",
            ],
        );
        assert_eq!(stage(engine, &client), 1);
        assert_eq!(held(engine, &client, "snake_weed"), 1);

        // Picked, but still grimy.
        let mut client = login(engine, "jungle_herbalist", 2);
        give(engine, &client, 0, "unidentified_snake_weed");
        let t = talk(engine, &mut client, &["Of course!"]);
        says(
            &t,
            &["Sorry, Bwana, that herb is so dirty that I can't even tell whether it is fresh."],
        );
        assert_eq!(stage(engine, &client), 2);

        // Picked and cleaned: each in turn.
        let turns = [
            (2, "snake_weed", "Great, you have the Snake Weed!"),
            (4, "ardrigal", "Great, you have the Ardrigal! Many thanks."),
            (
                6,
                "sito_foil",
                "Well done Bwana, just two more herbs to collect.",
            ),
            (8, "volencia_moss", "Ah Volencia Moss, beautiful."),
        ];
        for (found, herb, line) in turns {
            let mut client = login(engine, "jungle_herbalist", found);
            give(engine, &client, 0, herb);
            let t = talk(engine, &mut client, &["Of course!"]);
            says(&t, &[line]);
            assert_eq!(held(engine, &client, herb), 0, "{herb} handed over");
            assert_eq!(stage(engine, &client), found + 1, "{herb}");
        }
    });
}

#[test]
fn the_last_herb_completes_the_quest_with_2011s_scroll() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "jungle_herbalist", 10);
        give(engine, &client, 0, "rogues_purse");
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[HERBLORE];
        let mut t = talk(engine, &mut client, &["Of course!"]);
        for _ in 0..10 {
            engine.cycle();
            let packets = client.packets();
            t.texts
                .extend(settexts(&packets).into_iter().map(|(_, t)| t));
            t.messages.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, t)| t),
            );
        }
        says(
            &t,
            &[
                "Greetings Bwana, have you been successful in getting the Rogues Purse?",
                "Most excellent Bwana!",
                "shows you some techniques in Herblore.",
                // The wiki and the recording's scroll.
                "You have completed the Jungle Potion Quest!",
                "1 Quest Point",
                "775 Herblore XP",
            ],
        );
        assert!(
            t.messages
                .iter()
                .any(|m| m == "Congratulations! Quest complete!"),
            "{:?}",
            t.messages
        );
        assert_eq!(held(engine, &client, "rogues_purse"), 0);
        assert_eq!(stage(engine, &client), 12, "Jungle Potion complete");
        let gained = engine.harness_player(client.pid).unwrap().stats.xp[HERBLORE] - xp;
        assert_eq!(gained, 7750, "775 Herblore xp (tenths)");

        let lines = journal(engine, &mut client);
        assert_eq!(
            lines[0],
            "Trufitus Shakaya of the Tai Bwo Wannai village needed"
        );
        assert!(lines.iter().any(|l| l == "QUEST COMPLETE!"), "{lines:?}");

        // Until Tai Bwo Wannai Trio moves on, he sends the player to Timfraku (289).
        // The first click on him shuts the scroll, and the level-up box (Herblore 1 to 7, 775 xp)
        // opens in its place; the second shuts that and he speaks.
        click(engine, &mut client);
        settle(engine, &mut client, 3);
        let t = talk(engine, &mut client, &[]);
        says(
            &t,
            &[
                "My greatest respects Bwana,",
                "You should deliver the good news to Bwana Timfraku, Chief of Tai Bwo Wannai.",
            ],
        );
        assert_eq!(stage(engine, &client), 13);
    });
}

#[test]
fn the_rocks_lead_into_the_cave_and_the_hand_holds_out() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "jungle_herbalist", 9);
        let pothole = id(engine, "loc", "pothole_cave_entrance");
        let (x, z) = beside(POTHOLE.0 - 1, 0, POTHOLE.1);
        engine.harness_telejump(client.pid, x, 0, z);
        settle(engine, &mut client, 3);

        op_loc2(&mut client, pothole, POTHOLE);
        let t = converse(engine, &mut client, &["No thanks"], 40);
        says(
            &t,
            &[
                "You search the rocks... You find an entrance into some caves.",
                "Would you like to enter the caves?",
                "You decide to stay where you are!",
            ],
        );
        assert!(coord(engine, &client).1 < 4000, "still above ground");

        op_loc2(&mut client, pothole, POTHOLE);
        let t = converse(engine, &mut client, &["Yes, I'll enter the cave."], 40);
        says(
            &t,
            &[
                "You decide to enter the caves. You climb down several steep rock faces into the cavern below.",
            ],
        );
        let inside = coord(engine, &client);
        assert_eq!(inside, (2830, 9520), "289's tile, in 2011's cave");
        assert!(walkable(inside.0, 0, inside.1), "a floor tile of the cave");

        let holds = id(engine, "loc", "jp_caverocksout");
        op_loc1(&mut client, holds, HAND_HOLDS);
        let t = converse(engine, &mut client, &[], 40);
        says(&t, &["You attempt to climb the rocks back out."]);
        let outside = coord(engine, &client);
        assert_eq!(outside, (2823, 3120), "289's tile beside the rocks");
        assert!(walkable(outside.0, 0, outside.1));
    });
}
