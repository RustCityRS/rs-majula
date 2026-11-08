//! Goblin Diplomacy at 647, the 2011 rework (`quests/quest_gobdip`,
//! `areas/area_falador/scripts/goblin_village`): it starts at the generals, the menu leaves out
//! the option just asked, Grubfoot tries each armour on with the minimap blacked out and comes
//! back in its colour (%varplayerbit_1782), and the quest runs on %varplayerbit_2378, which
//! cache 786's quest 137 reads. Lines from the 2011-06-14 and 2011-08-29 recordings
//! (runescape/target/refs/gobdip/transcript_0614.md).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const CRAFTING: usize = 12;
const BACKPACK: i32 = 149 << 16;

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

/// Varp 62: %varplayerbit_2378 is bits 0-5 (the quest), %varplayerbit_1782 bits 9-10
/// (Grubfoot's mail).
fn varp62(engine: &Engine, client: &HarnessClient) -> (i32, i32) {
    let v = engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(62)
        .as_int();
    (v & 0x3f, (v >> 9) & 0x3)
}

/// A spawn of npc `name`: its nid, and a tile beside it one step from it reaches.
fn beside(engine: &Engine, name: &str) -> (u16, u16, u8, u16) {
    let id = *engine
        .cache
        .npcs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("npc.pack names {name}"));
    let (nid, nx, y, nz) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.y(), c.z())
        })
        .unwrap_or_else(|| panic!("{name} is spawned"));
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|&(dx, dz)| {
            rsmod::can_travel(y, nx, nz, dx, dz, 1, 0, CollisionType::Normal)
                && !rsmod::is_flagged(
                    nx.wrapping_add_signed(dx as i16),
                    nz.wrapping_add_signed(dz as i16),
                    y,
                    0x100 | 0x200000,
                )
        })
        .map(|(dx, dz)| {
            (
                nx.wrapping_add_signed(dx as i16),
                nz.wrapping_add_signed(dz as i16),
            )
        })
        .unwrap_or_else(|| panic!("a tile beside {name} at {nx},{nz}"));
    (nid, sx, y, sz)
}

/// Each MINIMAP_TOGGLE's state, in order.
fn minimap_states(packets: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let opcode = ServerProt::MinimapToggle as u16;
    packets
        .iter()
        .filter(|(op, p)| *op == opcode && p.len() == 1)
        .map(|(_, p)| p[0])
        .collect()
}

/// Talks to General Bentnoze and answers `answers`, then lets any timed boxes and pauses play
/// out.
fn talk(
    engine: &mut Engine,
    client: &mut HarnessClient,
    nid: u16,
    answers: &[&str],
) -> Conversation {
    talk_to(client, nid);
    let mut talk = converse(engine, client, answers, 400);
    // The changing room's timed boxes and pause can end the first pass early; a short second
    // pass plays them out. Short: with no box open it runs to its limit, and a harness client
    // that sends nothing for long is logged out.
    let rest = converse(engine, client, &[], 30);
    talk.texts.extend(rest.texts);
    talk.packets.extend(rest.packets);
    talk.unanswered = talk.unanswered.or(rest.unanswered);
    talk
}

#[test]
fn goblin_diplomacy_as_2011s_rework() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let (orange, blue, brown, gold) = (
            obj(engine, "orange_goblin_mail"),
            obj(engine, "blue_goblin_mail"),
            obj(engine, "goblin_mail"),
            obj(engine, "gold_bar"),
        );
        let _ = engine.script_errors.take();
        engine.harness_reset();
        let mut client: HarnessClient = engine
            .harness_login_returning("gobdip_2011")
            .expect("login");
        for _ in 0..10 {
            engine.cycle();
        }
        let (nid, x, y, z) = beside(engine, "general_bentnoze_4493");
        engine.harness_telejump(client.pid, x, y, z);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();

        // The start, at the generals: an answer, the menu without it, then orange.
        let start = talk(
            engine,
            &mut client,
            nid,
            &[
                "Why are you arguing about the colour of your armour?",
                "Do you want me to pick an armour colour for you?",
                "What about a different colour?",
            ],
        );
        assert_eq!(start.unanswered, None, "{:?}", start.texts);
        for line in [
            "We decide to celebrate goblin new century by changing colour of our armour, brown get boring after a bit. We want change.",
            "Problem is they want different change to us.",
            "Yes as long as you pick green.",
            "No you have to pick red!",
            "You should wear red",
            "What about a different colour? Not green or red?",
            "That would mean me wrong... but at least Wartface not right!",
            "Yep bring us orange armour.",
        ] {
            assert!(start.said(line), "2011 shows {line:?}; {:?}", start.texts);
        }
        assert_eq!(varp62(engine, &client).0, 2, "asked for orange armour");

        // Orange, blue and brown, each tried on by Grubfoot.
        for (mail, colour, stage, grubfoot, verdict) in [
            (orange, "orange", 3, 1, "Human! Get us blue armour!"),
            (blue, "blue", 4, 2, "Human! Get us brown armour!"),
            (
                brown,
                "brown",
                6,
                0,
                "Thank you for sorting out our argument. Take this gold bar as reward!",
            ),
        ] {
            engine
                .harness_player_mut(client.pid)
                .unwrap()
                .invs
                .get_mut(&inv)
                .unwrap()
                .set(0, mail, 1);
            let xp = engine.harness_player(client.pid).unwrap().stats.xp[CRAFTING];
            let option = format!("I have some {colour} armour here");
            let hand_over = talk(engine, &mut client, nid, &[option.as_str()]);
            assert_eq!(hand_over.unanswered, None, "{:?}", hand_over.texts);
            for line in [
                format!("I have some {colour} armour here."),
                "Grubfoot!".to_owned(),
                "Yes General Wartface sir?".to_owned(),
                "Put on this armour!".to_owned(),
                "Grubfoot goes into the changing room...".to_owned(),
                format!("...And emerges wearing {colour} armour."),
                "What do you think?".to_owned(),
                verdict.to_owned(),
            ] {
                assert!(
                    hand_over.said(&line),
                    "2011 shows {line:?}; {:?}",
                    hand_over.texts
                );
            }
            assert_eq!(
                minimap_states(&hand_over.packets),
                [2, 0],
                "the minimap blacked out for the changing room, then back"
            );
            for _ in 0..3 {
                engine.cycle();
            }
            assert_eq!(varp62(engine, &client), (stage, grubfoot), "after {colour}");
            let player = engine.harness_player(client.pid).unwrap();
            assert_eq!(
                player.invs.get(&inv).unwrap().total(mail),
                0,
                "the {colour} mail is given"
            );
            if colour == "brown" {
                assert_eq!(
                    player.invs.get(&inv).unwrap().total(gold),
                    1,
                    "the gold bar"
                );
                assert_eq!(
                    player.stats.xp[CRAFTING] - xp,
                    2000,
                    "200 Crafting experience"
                );
            }
        }
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}

/// 2011's dye message: "You dye the goblin armour blue." (289's mail-first path said "dark
/// blue"; the 2011-06-14 recording's chat history).
#[test]
fn goblin_mail_dyes_as_2011_says() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let (mail, blue_dye, blue) = (
            obj(engine, "goblin_mail"),
            obj(engine, "blue_dye"),
            obj(engine, "blue_goblin_mail"),
        );
        engine.harness_reset();
        let mut client: HarnessClient = engine
            .harness_login_returning("gobdip_dyer")
            .expect("login");
        for _ in 0..10 {
            engine.cycle();
        }
        {
            let pack = engine
                .harness_player_mut(client.pid)
                .unwrap()
                .invs
                .get_mut(&inv)
                .unwrap();
            pack.set(0, blue_dye, 1);
            pack.set(1, mail, 1);
        }
        engine.cycle();
        client.packets();
        // IF_BUTTONT: the dye (slot 0) on the mail (slot 1).
        let mut p = ClientPacket::new();
        p.p4(BACKPACK)
            .p2_alt1(1)
            .p2_alt1(0)
            .p2_alt2(blue_dye as i32)
            .p4_alt1(BACKPACK)
            .p2(mail as i32);
        client.send(ClientProt::IfButtonT, &p.finish());
        let mut said = Vec::new();
        for _ in 0..4 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
        }
        assert!(
            said.iter().any(|m| m == "You dye the goblin armour blue."),
            "{said:?}"
        );
        let pack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&inv)
            .unwrap();
        assert_eq!(
            (pack.total(blue), pack.total(mail), pack.total(blue_dye)),
            (1, 0, 0)
        );
    });
}

/// The crate behind the generals' hut (loc 16559, 46_54 15,58): "You find some goblin armour."
/// in an item box, before the quest is started (2011-05-27 recording, YouTube coiqGsbSVks, 66s).
#[test]
fn the_crate_behind_the_generals_hut_gives_goblin_mail() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let mail = obj(engine, "goblin_mail");
        let _ = engine.script_errors.take();
        engine.harness_reset();
        let mut client: HarnessClient = engine
            .harness_login_returning("gobdip_crate")
            .expect("login");
        for _ in 0..10 {
            engine.cycle();
        }
        let (loc, x, z) = (16559u16, 2959u16, 3514u16);
        let (sx, sz) = [(1i16, 0i16), (0, -1), (-1, 0), (0, 1)]
            .into_iter()
            .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
            .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
            .expect("a walkable tile beside the crate");
        engine.harness_telejump(client.pid, sx, 0, sz);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        let mut p = ClientPacket::new();
        p.p2_alt1(z as i32)
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let talk = converse(engine, &mut client, &[], 60);
        assert!(
            talk.said("You find some goblin armour."),
            "{:?}",
            talk.texts
        );
        assert!(
            talk.messages.is_empty(),
            "no chat message: {:?}",
            talk.messages
        );
        let pack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&inv)
            .unwrap();
        assert_eq!(pack.total(mail), 1, "the goblin mail");
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}

/// 2011's generals (npcs 4493 and 4494) take 289's generals' wander range, 3.
#[test]
fn the_generals_wander_as_289s_did() {
    runner::on_loaded_engine(|engine| {
        for name in ["general_bentnoze_4493", "general_wartface_4494"] {
            let id = *engine.cache.npcs.debugnames.get(name).unwrap();
            let general = engine.cache.npcs.get_by_id(id).unwrap();
            assert_eq!(general.wanderrange, 3, "{name}");
        }
    });
}
