//! 2011's quest-start window (interface 178, quest_intro) on Cook's Assistant, as the 2011-05-20
//! recording shows it (YouTube PnoZkxqcszs, 84-86s). After the cook's "Would you help me?
//! Please?" the window opens with cache struct_512's text and the chef's hat (sprite 2379).
//! "Click here to show" reveals the required items and the rewards, as the 2011-05-04 Ernest the
//! Chicken recording does (sbY-Uq0JI4M, 28-34s). "Mark on map" flips its tick, and Yes carries on
//! with "I'm always happy to help a cook in distress." No leaves the quest unstarted.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, settexts, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const QUEST_INTRO: u16 = 178;

fn com(c: u16) -> i32 {
    (i32::from(QUEST_INTRO) << 16) | i32::from(c)
}

/// Every packet a test saw, as (opcode, payload).
type Packets = Vec<(u16, Vec<u8>)>;

/// A player beside Lumbridge's cook, Cook's Assistant not started, talked through to the window.
fn at_the_window(engine: &mut Engine, name: &str) -> (HarnessClient, Packets, Vec<String>) {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let cook = *engine.cache.npcs.debugnames.get("cook").expect("npc cook");
    let (nid, x, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == cook)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .expect("Lumbridge's cook");
    engine.harness_telejump(client.pid, x + 1, 0, z);
    let varp = *engine.cache.varps.debugnames.get("cookquest").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(varp, VarValue::Int(0));
    engine.cycle();
    client.packets();
    talk_to(&mut client, nid);
    let talk = converse(engine, &mut client, &["What's wrong?"], 60);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    assert!(
        talk.said("Would you help me? Please?"),
        "the cook's plea comes first; {:?}",
        talk.texts
    );
    assert!(
        engine
            .harness_player(client.pid)
            .unwrap()
            .is_interface_open(QUEST_INTRO),
        "the quest-start window opens; {:?}",
        talk.texts
    );
    (client, talk.packets, talk.texts)
}

/// IF_BUTTON1 on a quest_intro component: `p4(com) p2_alt3(obj) p2_alt1(slot)`, no obj or slot.
fn click(engine: &mut Engine, client: &mut HarnessClient, c: u16) -> Vec<(u16, Vec<u8>)> {
    let mut p = ClientPacket::new();
    p.p4(com(c)).p2_alt3(0xFFFF).p2_alt1(0xFFFF);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut packets = Vec::new();
    for _ in 0..2 {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

fn of(packets: &[(u16, Vec<u8>)], prot: ServerProt) -> impl Iterator<Item = &Vec<u8>> {
    let opcode = prot as u16;
    packets
        .iter()
        .filter(move |(op, _)| *op == opcode)
        .map(|(_, p)| p)
}

fn text_of(packets: &[(u16, Vec<u8>)], c: u16) -> Option<String> {
    settexts(packets)
        .into_iter()
        .rev()
        .find(|(h, _)| *h == com(c) as u32)
        .map(|(_, t)| t)
}

/// Each IF_SETGRAPHIC: `p4_alt1(com)` (little-endian), `p2_alt2(graphic)` (low byte + 128).
fn graphics(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16)> {
    of(packets, ServerProt::IfSetGraphic)
        .map(|p| {
            let c = i32::from_le_bytes([p[0], p[1], p[2], p[3]]);
            (c, u16::from_be_bytes([p[4], p[5].wrapping_sub(128)]))
        })
        .collect()
}

/// Each IF_SETHIDE: `p4_alt3(com)` (`>>16, >>24, v, >>8`), `p1_alt2(hidden)` (negated).
fn hides(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool)> {
    of(packets, ServerProt::IfSetHide)
        .map(|p| {
            let c = i32::from_be_bytes([p[1], p[0], p[3], p[2]]);
            (c, p[4].wrapping_neg() == 1)
        })
        .collect()
}

/// Each RUNCLIENTSCRIPT with string arguments only: (script, its strings).
fn scripts(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, Vec<String>)> {
    of(packets, ServerProt::RunClientScript)
        .map(|p| {
            let n = p.len();
            let id = i32::from_be_bytes([p[n - 4], p[n - 3], p[n - 2], p[n - 1]]);
            let mut parts = p[..n - 4].split(|&b| b == 0);
            let types = parts.next().unwrap_or_default();
            let strings = if types.iter().all(|&t| t == b's') {
                parts
                    .take(types.len())
                    .map(|s| String::from_utf8_lossy(s).into_owned())
                    .collect()
            } else {
                Vec::new()
            };
            (id, strings)
        })
        .collect()
}

/// %varplayerbit_6924, "Mark on map": bit 29 of varp 1502 (cache 786).
fn mark(engine: &Engine, client: &HarnessClient) -> i32 {
    let varp = *engine.cache.varps.debugnames.get("varplayer_1502").unwrap();
    (engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(varp)
        .as_int()
        >> 29)
        & 1
}

fn cookquest(engine: &Engine, client: &HarnessClient) -> i32 {
    let varp = *engine.cache.varps.debugnames.get("cookquest").unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(varp)
        .as_int()
}

#[test]
fn the_cook_opens_2011s_quest_start_window() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (_client, packets, texts) = at_the_window(engine, "window_reader");
        for (c, want) in [
            (14, "Cook's Assistant"),
            (52, "Speak to the cook in the kitchen of Lumbridge Castle."),
            (55, "None."),
            (62, "No specific combat, but watch out for local goblins!"),
        ] {
            assert_eq!(
                text_of(&packets, c).as_deref(),
                Some(want),
                "com{c}; {texts:?}"
            );
        }
        assert!(
            graphics(&packets).contains(&(com(79), 2379)),
            "the chef's hat, sprite 2379: {:?}",
            graphics(&packets)
        );
        assert!(
            scripts(&packets).contains(&(2827, vec!["None.".to_string()])),
            "the requirements row's layout: {:?}",
            scripts(&packets)
        );
        assert!(
            !texts
                .iter()
                .any(|t| t == "Start the Cook's Assistant quest?"),
            "void-634's chat choice is not 2011's"
        );
    });
}

#[test]
fn click_here_to_show_reveals_the_items_and_the_rewards() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, _, _) = at_the_window(engine, "window_shower");
        let items = click(engine, &mut client, 57);
        assert_eq!(text_of(&items, 59).as_deref(), Some("A pot, a bucket."));
        let hidden = hides(&items);
        assert!(hidden.contains(&(com(56), true)), "{hidden:?}");
        assert!(hidden.contains(&(com(58), false)), "{hidden:?}");
        assert!(scripts(&items).contains(&(2828, vec!["A pot, a bucket.".to_string()])));

        let rewards = click(engine, &mut client, 31);
        let want =
            "1 Quest Point, 300 Cooking XP, 500 coins, 20 sardines and access to the cook's range.";
        assert_eq!(text_of(&rewards, 66).as_deref(), Some(want));
        let hidden = hides(&rewards);
        assert!(hidden.contains(&(com(64), true)), "{hidden:?}");
        assert!(hidden.contains(&(com(65), false)), "{hidden:?}");
        assert!(scripts(&rewards).contains(&(4074, vec![want.to_string()])));

        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(QUEST_INTRO),
            "the window stays open"
        );
        assert_eq!(cookquest(engine, &client), 0, "not started by looking");
    });
}

#[test]
fn mark_on_map_flips_its_tick() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, _, _) = at_the_window(engine, "window_marker");
        assert_eq!(
            mark(engine, &client),
            0,
            "ticked, as both recordings open it"
        );
        click(engine, &mut client, 76);
        assert_eq!(mark(engine, &client), 1, "the text unticks it");
        click(engine, &mut client, 75);
        assert_eq!(mark(engine, &client), 0, "the box ticks it again");
        click(engine, &mut client, 75);
        assert_eq!(mark(engine, &client), 1);
        let packets = click(engine, &mut client, 73);
        let said: Vec<String> = HarnessClient::game_messages(&packets)
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        assert!(
            said.iter()
                .any(|m| m == "The quest area marking has been cleared."),
            "2011-05-04 Ernest the Chicken, 38.1s: {said:?}"
        );
    });
}

#[test]
fn yes_starts_cooks_assistant_where_2011s_dialogue_goes_on() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, _, _) = at_the_window(engine, "window_yes");
        let packets = click(engine, &mut client, 73);
        assert!(
            HarnessClient::game_messages(&packets).is_empty(),
            "ticked, the Cook's recording says nothing"
        );
        assert!(
            !engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(QUEST_INTRO),
            "Yes closes the window"
        );
        let mut said: Vec<String> = settexts(&packets).into_iter().map(|(_, t)| t).collect();
        said.extend(converse(engine, &mut client, &[], 60).texts);
        assert!(
            said.join(" ")
                .contains("I'm always happy to help a cook in distress."),
            "2011-05-20, 86.3s; {said:?}"
        );
        assert_eq!(cookquest(engine, &client), 1, "Cook's Assistant started");
    });
}

#[test]
fn no_leaves_cooks_assistant_unstarted() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, _, _) = at_the_window(engine, "window_no");
        click(engine, &mut client, 74);
        assert!(
            !engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(QUEST_INTRO),
            "No closes the window"
        );
        converse(engine, &mut client, &[], 30);
        assert_eq!(cookquest(engine, &client), 0, "not started");
    });
}

/// A player on a walkable tile beside npc `npc`, quest varp `varp` at 0, talked to with
/// `choices` until the quest-start window opens.
fn at_the_window_of(
    engine: &mut Engine,
    name: &str,
    npc: &str,
    varp: &str,
    choices: &[&str],
) -> (HarnessClient, Conversation) {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    for _ in 0..10 {
        engine.cycle();
    }
    let id = *engine.cache.npcs.debugnames.get(npc).unwrap();
    let (nid, x, y, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.y(), c.z())
        })
        .unwrap_or_else(|| panic!("{npc} is spawned"));
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|&(dx, dz)| rsmod::can_travel(y, x, z, dx, dz, 1, 0, CollisionType::Normal))
        .map(|(dx, dz)| {
            (
                x.wrapping_add_signed(dx as i16),
                z.wrapping_add_signed(dz as i16),
            )
        })
        .unwrap_or_else(|| panic!("a tile beside {npc}"));
    engine.harness_telejump(client.pid, sx, y, sz);
    let varp = *engine.cache.varps.debugnames.get(varp).unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(varp, VarValue::Int(0));
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    talk_to(&mut client, nid);
    let talk = converse(engine, &mut client, choices, 90);
    assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
    assert!(
        engine
            .harness_player(client.pid)
            .unwrap()
            .is_interface_open(QUEST_INTRO),
        "{npc} opens the quest-start window; {:?}",
        talk.texts
    );
    (client, talk)
}

fn quest_varp(engine: &Engine, client: &HarnessClient, varp: &str) -> i32 {
    let varp = *engine.cache.varps.debugnames.get(varp).unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(varp)
        .as_int()
}

/// Yes on the window, then every box after it: all the texts set.
fn accept(engine: &mut Engine, client: &mut HarnessClient) -> Vec<String> {
    let packets = click(engine, client, 73);
    let mut said: Vec<String> = settexts(&packets).into_iter().map(|(_, t)| t).collect();
    said.extend(converse(engine, client, &[], 60).texts);
    said
}

/// Ernest the Chicken, 2011-05-04 (YouTube sbY-Uq0JI4M, 25.6-37.5s): Veronica's story, the
/// window (cache struct_516), then "Aha, sounds like a quest. I'll help." and her thanks.
#[test]
fn veronica_tells_her_story_then_asks_in_the_window() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, talk) =
            at_the_window_of(engine, "window_ernest", "veronica", "haunted", &[]);
        for line in [
            "Can you please help me? I'm in a terrible spot of trouble. My fiancé, Ernest, and I came upon this house.",
            "Seeing as we were a little lost, Ernest decided to go in and ask for directions.",
            "That was an hour ago...and that house looks spooky. Can you go and see if you can find him for me?",
        ] {
            assert!(talk.said(line), "2011 shows {line:?}; {:?}", talk.texts);
        }
        assert_eq!(
            text_of(&talk.packets, 14).as_deref(),
            Some("Ernest the Chicken")
        );
        assert!(graphics(&talk.packets).contains(&(com(79), 2382)));
        let said = accept(engine, &mut client).join(" ");
        for line in [
            "Aha, sounds like a quest. I'll help.",
            "Thank you, thank you. I'm very grateful. I think I spotted some lights flashing in one of the top floor windows, so you may want to head up there first.",
        ] {
            assert!(said.contains(line), "2011 shows {line:?}; {said:?}");
        }
        assert_eq!(
            quest_varp(engine, &client, "haunted"),
            1,
            "Ernest the Chicken started"
        );
    });
}

/// Rune Mysteries, 2011-05-22 (YouTube VIE8MXrw-7M, 25.5-32.8s): the Duke's 2011 lines, the
/// window (cache struct_524), "Sure, no problem.", his thanks in two boxes and the talisman.
#[test]
fn the_duke_asks_in_the_window_and_hands_over_a_talisman() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, talk) = at_the_window_of(
            engine,
            "window_duke",
            "duke_horacio",
            "runemysteries",
            &["Have you any quests for me?"],
        );
        for line in [
            "Well, it's not really a quest, but I recently discovered this strange talisman.",
            "It's not like anything I have seen before. Would you take it to the head wizard in the basement of",
            "the Wizards' Tower for me? It should not take you very long at all and I would be awfully grateful.",
        ] {
            assert!(talk.said(line), "2011 shows {line:?}; {:?}", talk.texts);
        }
        assert_eq!(
            text_of(&talk.packets, 14).as_deref(),
            Some("Rune Mysteries")
        );
        assert!(graphics(&talk.packets).contains(&(com(79), 2378)));
        let said = accept(engine, &mut client).join(" ");
        for line in [
            "Sure, no problem.",
            "Thank you very much, stranger.",
            "I am sure the head wizard will reward you for such an interesting find.",
            "The Duke hands you a <col=000080>talisman</col>.",
        ] {
            assert!(said.contains(line), "2011 shows {line:?}; {said:?}");
        }
        assert_eq!(
            quest_varp(engine, &client, "runemysteries"),
            1,
            "Rune Mysteries started"
        );
    });
}

/// The Restless Ghost, 2011-04-20 (YouTube i7PABMV6o2k, 68-98s) and 2011-06-24 (iZmUDYaw2Cc):
/// "That's lucky, I need someone to do a quest for me.", the window (cache struct_522), then
/// 2011's lines where void-634 has its own.
#[test]
fn father_aereck_asks_in_the_window_and_sends_you_to_urhney() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, talk) = at_the_window_of(
            engine,
            "window_ghost",
            "father_aereck",
            "prieststart",
            &["I'm looking for a quest."],
        );
        assert!(talk.said("That's lucky, I need someone to do a quest for me."));
        assert_eq!(
            text_of(&talk.packets, 14).as_deref(),
            Some("The Restless Ghost")
        );
        assert!(graphics(&talk.packets).contains(&(com(79), 2381)));
        let said = accept(engine, &mut client).join(" ");
        for line in [
            "Okay, let me help then.",
            "Thank you. The problem is there's a ghost in the graveyard crypt just south of this church. I would like you to get rid of it.",
            "You'll need the help of my friend, Father Urhney, who is a bit of a ghost expert.",
            "He's currently living in a little shack in the south of Lumbridge Swamps, near the coast.",
            "My name is Father Aereck, by the way. Pleased to meet you.",
            "Likewise.",
            "Take care travelling through the swamps. To get there just follow the path south, through the graveyard.",
            "I will, thanks.",
        ] {
            assert!(said.contains(line), "2011 shows {line:?}; {said:?}");
        }
        assert_eq!(
            quest_varp(engine, &client, "prieststart"),
            1,
            "The Restless Ghost started"
        );
    });
}
