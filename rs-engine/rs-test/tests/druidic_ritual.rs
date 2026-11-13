//! Druidic Ritual at 647 (`quests/quest_druid`, Kaqemeex and Sanfew in `areas/area_taverly`):
//! 289's quest, which 2011 still has, checked against the 2011 recording (YouTube agUATXHTRIg,
//! uploaded 2011-04-27). The recording differs from 289 in the player's "Ok, I will try and
//! help.", the comma in "Actually, I don't need to speak to you.", the journal's first
//! paragraph, the scroll's reward lines, and in nothing following the scroll.

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
const HERBLORE: usize = 15;
/// The Cauldron of Thunder in Taverley Dungeon (cache 786's map, maps/l45_153.jm2).
const CAULDRON: (u16, u16) = (2893, 9831);
/// View on the quest list's row (questlist_v4:quests) and the journal's first line (qj1).
const QUEST_LIST: i32 = 190 << 16 | 18;
const DRUIDIC_RITUAL: i32 = 33;
const JOURNAL_LINE_1: u32 = 275 << 16 | 16;

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

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    let v = id(engine, "varp", "druidquest");
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(v)
        .as_int()
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

/// A walkable tile beside (x, z) on `level`.
fn beside(x: u16, level: u8, z: u16) -> (u16, u16) {
    [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(x, z)| !rsmod::is_flagged(x, z, level, 0x100 | 0x200000))
        .unwrap_or_else(|| panic!("a walkable tile beside {x}, {z}"))
}

/// Logs in with the quest at `stage`.
fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    let v = id(engine, "varp", "druidquest");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v, VarValue::Int(stage));
    client
}

/// Stands beside the npc of that name and talks to it, taking `choices`.
fn talk(
    engine: &mut Engine,
    client: &mut HarnessClient,
    name: &str,
    choices: &[&str],
) -> Conversation {
    let want = id(engine, "npc", name);
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
        .unwrap_or_else(|| panic!("{name} is spawned"));
    let (sx, sz) = beside(x, level, z);
    engine.harness_telejump(client.pid, sx, level, sz);
    settle(engine, client, 3);
    talk_to(client, nid);
    let talk = converse(engine, client, choices, 300);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    talk
}

fn says(t: &Conversation, lines: &[&str]) {
    for line in lines {
        assert!(
            t.said(line),
            "2011 shows {line:?}; {:?} (chat {:?})",
            t.texts,
            t.messages
        );
    }
}

/// Uses the backpack's `slot` on the cauldron and returns the game messages that follow.
fn dip(engine: &mut Engine, client: &mut HarnessClient, obj: &str, slot: u16) -> Vec<String> {
    let obj = id(engine, "obj", obj);
    let cauldron = id(engine, "loc", "cauldron_of_thunder");
    let mut p = ClientPacket::new();
    p.p2_alt2(obj as i32)
        .p2_alt2(slot as i32)
        .p2(CAULDRON.1 as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(CAULDRON.0 as i32)
        .p1_alt1(0)
        .p2_alt1(cauldron as i32);
    client.send(ClientProt::OpLocT, &p.finish());
    let mut out = Vec::new();
    for _ in 0..4 {
        engine.cycle();
        out.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    out
}

/// The journal's lines once View is clicked on the quest's row, their colour tags taken off.
fn journal(engine: &mut Engine, client: &mut HarnessClient) -> Vec<String> {
    let mut p = ClientPacket::new();
    p.p4(QUEST_LIST).p2_alt3(-1).p2_alt1(DRUIDIC_RITUAL);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut texts = Vec::new();
    for _ in 0..3 {
        engine.cycle();
        texts.extend(settexts(&client.packets()));
    }
    (0..8)
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

const PREPARING: [&str; 4] = [
    "Kaqemeex and the druids are preparing a ceremony to",
    "purify the Varrock stone circle. I told Kaqemeex I would",
    "help them.",
    "",
];

#[test]
fn kaqemeex_starts_the_quest_in_2011s_words() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "druid_helper", 0);
        let t = talk(
            engine,
            &mut client,
            "kaqemeex",
            &["I'm in search of a quest.", "Ok, I will try and help."],
        );
        says(
            &t,
            &[
                "Hello there.",
                "What brings you to our holy monument?",
                "Hmm. I think I may have a worthwhile quest for you actually. I don't know if you are familiar with the stone circle south of Varrock or not, but...",
                "That used to be OUR stone circle. Unfortunately, many many years ago, dark wizards cast a wicked spell upon it so that they could corrupt its power for their own evil ends.",
                "So... is there anything in this for me?",
                "Excellent. Go to the village south of this place and speak to my fellow Sanfew who is working on the purification ritual. He knows better than I what is required to complete it.",
                "Will do.",
            ],
        );
        assert!(
            !t.said("Ok, I will try to help."),
            "289's line; 2011's player says the option's words: {:?}",
            t.texts
        );
        assert_eq!(stage(engine, &client), 1, "Druidic Ritual started");

        let lines = journal(engine, &mut client);
        assert_eq!(lines[..4], PREPARING, "the recording at 133s");
        assert_eq!(
            lines[4],
            "I should speak to Sanfew in the village to the south."
        );
    });
}

#[test]
fn sanfew_sends_the_player_to_the_cauldron() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "druid_helper", 1);
        let t = talk(
            engine,
            &mut client,
            "sanfew",
            &[
                "I've been sent to help purify the Varrock stone circle.",
                "Where can I find this cauldron?",
            ],
        );
        says(
            &t,
            &[
                "What can I do for you young 'un?",
                "Actually, I don't need to speak to you.",
                "I've been sent to assist you with the ritual to purify the Varrockian stone circle.",
                "Each meat has to be dipped individually into the Cauldron of Thunder for it to work correctly.",
                "It is located somewhere in the mysterious underground halls which are located somewhere in the woods just South of here. They are too dangerous for me to go myself however.",
            ],
        );
        assert_eq!(stage(engine, &client), 2, "spoken to Sanfew");

        let lines = journal(engine, &mut client);
        assert_eq!(lines[..4], PREPARING, "the recording at 276s");
        assert_eq!(
            lines[4..7],
            [
                "Sanfew told me for the ritual they would need me to place",
                "raw bear meat, raw chicken, raw rat meat and raw beef in",
                "the Cauldron of Thunder in the dungeon south of Taverley.",
            ]
        );
    });
}

#[test]
fn the_cauldron_enchants_the_four_meats_once_sanfew_asks() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let meats = [
            ("raw_rat_meat", "enchanted_rat_meat", "raw rat meat"),
            ("raw_chicken", "enchanted_chicken", "raw chicken"),
            ("raw_bear_meat", "enchanted_bear_meat", "raw bear meat"),
            ("raw_beef", "enchanted_beef", "raw beef"),
        ];
        // Before Sanfew asks, the cauldron does nothing (289).
        let mut client = login(engine, "druid_helper", 1);
        give(engine, &client, 0, "raw_beef");
        let (x, z) = beside(CAULDRON.0, 0, CAULDRON.1);
        engine.harness_telejump(client.pid, x, 0, z);
        settle(engine, &mut client, 3);
        let said = dip(engine, &mut client, "raw_beef", 0);
        assert!(
            said.iter().any(|m| m == "Nothing interesting happens."),
            "{said:?}"
        );
        assert_eq!(held(engine, &client, "raw_beef"), 1);

        let mut client = login(engine, "druid_helper", 2);
        for (slot, (raw, _, _)) in meats.iter().enumerate() {
            give(engine, &client, slot as u16, raw);
        }
        engine.harness_telejump(client.pid, x, 0, z);
        settle(engine, &mut client, 3);
        for (slot, (raw, enchanted, name)) in meats.iter().enumerate() {
            let said = dip(engine, &mut client, raw, slot as u16);
            let line = format!("You dip the {name} in the cauldron.");
            assert!(said.contains(&line), "2011 shows {line:?}; {said:?}");
            assert_eq!(held(engine, &client, raw), 0, "{raw} dipped");
            assert_eq!(held(engine, &client, enchanted), 1, "{enchanted}");
        }
    });
}

#[test]
fn sanfew_takes_the_meats_and_kaqemeex_teaches_herblore() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let meats = [
            "enchanted_beef",
            "enchanted_rat_meat",
            "enchanted_bear_meat",
            "enchanted_chicken",
        ];
        let mut client = login(engine, "druid_helper", 2);
        for (slot, meat) in meats.iter().enumerate() {
            give(engine, &client, slot as u16, meat);
        }
        let t = talk(engine, &mut client, "sanfew", &[]);
        says(
            &t,
            &[
                "Did you bring me the required ingredients for the potion?",
                "Yes, I have all four now!",
                "Well hand 'em over then lad!",
                "Thank you so much adventurer! These meats will allow our potion to honour Guthix to be completed, and bring one step closer to reclaiming our stone circle!",
                "Now go and talk to Kaqemeex and he will introduce you to the wonderful world of herblore and potion making!",
            ],
        );
        assert_eq!(stage(engine, &client), 3, "meats handed over");
        for meat in meats {
            assert_eq!(held(engine, &client, meat), 0, "{meat} handed over");
        }

        let xp = engine.harness_player(client.pid).unwrap().stats.xp[HERBLORE];
        let mut t = talk(engine, &mut client, "kaqemeex", &[]);
        // Whatever follows the scroll within ten ticks.
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
                "I have word from Sanfew that you have been very helpful in assisting him with his preparations for the purification ritual. As promised I will now teach you the ancient arts of Herblore.",
                "You have completed the Druidic Ritual quest!",
                "4 Quest Points",
                "250 Herblore XP",
                "Access to Herblore skill",
            ],
        );
        assert!(
            t.messages
                .iter()
                .any(|m| m == "Congratulations! Quest complete!"),
            "{:?}",
            t.messages
        );
        assert!(
            !t.said("I will now explain the fundamentals of Herblore:"),
            "289 follows the scroll with the fundamentals; the recording shows nothing: {:?}",
            t.texts
        );
        assert_eq!(stage(engine, &client), 4, "Druidic Ritual complete");
        let gained = engine.harness_player(client.pid).unwrap().stats.xp[HERBLORE] - xp;
        assert_eq!(gained, 2500, "250 Herblore xp (tenths)");
    });
}
