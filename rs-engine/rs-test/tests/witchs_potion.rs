//! Witch's Potion at 647, a miniquest since 24 June 2010 (`areas/area_rimmington`,
//! `quests/quest_hetty`): Hetty's lines as the latest recording found has them (uploaded
//! 2011-04-25, YouTube cuYC6VjUrKk), %hetty on the cache's varp 67 and transmitted, since
//! her cauldron (loc 2024) offers "Drink-from" only at 2, and the reward with no quest point
//! (cache 786's quest 178).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_test::talk::{converse, talk_to};
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

/// Hetty's cauldron, loc 2024 in Rimmington (maps/l46_50.jm2: level 0, 23,5).
const CAULDRON: (u16, u16, u16) = (2024, 2967, 3205);
const MAGIC: usize = 6;

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

/// Hetty's nid, and a walkable tile beside her.
fn hetty(engine: &Engine) -> (u16, u16, u16) {
    let id = *engine.cache.npcs.debugnames.get("hetty").unwrap();
    let (nid, x, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .expect("Hetty is spawned");
    let (sx, sz) = [(1i16, 0i16), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1)]
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
        .expect("a walkable tile beside Hetty");
    (nid, sx, sz)
}

/// Each VARP_SMALL's (varp, value): `p1_alt3 value, p2 varp`.
fn varps_small(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, u8)> {
    let opcode = ServerProt::VarpSmall as u16;
    packets
        .iter()
        .filter(|(op, p)| *op == opcode && p.len() >= 3)
        .map(|(_, p)| (u16::from_be_bytes([p[1], p[2]]), 128u8.wrapping_sub(p[0])))
        .collect()
}

fn hetty_var(engine: &Engine, client: &HarnessClient, varp: u16) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(varp)
        .as_int()
}

#[test]
fn witchs_potion_as_2011s_miniquest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let varp = *engine.cache.varps.debugnames.get("hetty").unwrap();
        assert_eq!(varp, 67, "289's hetty is the cache's varp 67");
        let backpack = *engine.cache.invs.debugnames.get("inv").unwrap();
        let ingredients = [
            obj(engine, "burnt_meat"),
            obj(engine, "rats_tail"),
            obj(engine, "eye_of_newt"),
            obj(engine, "onion"),
        ];
        let _ = engine.script_errors.take();
        engine.harness_reset();
        let mut client: HarnessClient =
            engine.harness_login_returning("witch_2011").expect("login");
        for _ in 0..10 {
            engine.cycle();
        }
        let (nid, x, z) = hetty(engine);
        engine.harness_telejump(client.pid, x, 0, z);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();

        // The start: the recording's lines, with its commas.
        talk_to(&mut client, nid);
        let talk = converse(
            engine,
            &mut client,
            &[
                "I am in search of a quest.",
                "Yes, help me become one with my darker side.",
            ],
            300,
        );
        assert_eq!(talk.unanswered, None, "{:?}", talk.texts);
        for line in [
            "What could you want with an old woman like me?",
            "Hmmm... Maybe I can think of something for you.",
            "No, I have my principles and honour.",
            "Ok, I'm going to make a potion to help bring out your darker self.",
            "You need an eye of newt, a rat's tail, an onion... Oh, and a piece of burnt meat.",
            "Great, I'll go and get them.",
        ] {
            assert!(talk.said(line), "shows {line:?}; {:?}", talk.texts);
        }
        assert_eq!(hetty_var(engine, &client, varp), 1, "started");

        // The ingredients: into her cauldron, and varp 67 at 2 reaches the client.
        {
            let pack = engine
                .harness_player_mut(client.pid)
                .unwrap()
                .invs
                .get_mut(&backpack)
                .unwrap();
            for (slot, o) in ingredients.iter().enumerate() {
                pack.set(slot as u16, *o, 1);
            }
        }
        talk_to(&mut client, nid);
        let talk = converse(engine, &mut client, &[], 300);
        assert_eq!(talk.unanswered, None, "{:?}", talk.texts);
        for line in [
            "So have you found the things for the potion?",
            "Yes I have everything!",
            "Excellent, can I have them then?",
            "You pass the ingredients to Hetty and she puts them all into her cauldron. Hetty closes her eyes and begins to chant. The cauldron bubbles mysteriously.",
            "Well, is it ready?",
            "Ok, now drink from the cauldron.",
        ] {
            assert!(talk.said(line), "shows {line:?}; {:?}", talk.texts);
        }
        assert_eq!(hetty_var(engine, &client, varp), 2, "the ingredients given");
        assert!(
            varps_small(&talk.packets).contains(&(67, 2)),
            "varp 67 = 2 is sent, so the client offers Drink-from: {:?}",
            varps_small(&talk.packets)
        );
        let pack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&backpack)
            .unwrap();
        for o in ingredients {
            assert_eq!(pack.total(o), 0, "Hetty keeps {o}");
        }

        // Drink-from: 325 Magic experience, complete, and no quest scroll.
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[MAGIC];
        let (loc, lx, lz) = CAULDRON;
        let mut p = ClientPacket::new();
        p.p2_alt1(lz as i32)
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(lx as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let talk = converse(engine, &mut client, &[], 300);
        assert!(
            talk.said(
                "You drink from the cauldron, it tastes horrible! You feel yourself imbued with power."
            ),
            "{:?}",
            talk.texts
        );
        for _ in 0..3 {
            engine.cycle();
        }
        assert_eq!(hetty_var(engine, &client, varp), 3, "complete");
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.xp[MAGIC] - xp,
            3250,
            "325 Magic experience"
        );
        assert!(
            !talk.said("Witches Potion Quest"),
            "no quest scroll: {:?}",
            talk.texts
        );
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}
