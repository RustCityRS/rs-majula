//! 289's Rune Mysteries hooks: the wizards who teleport a player into the
//! rune essence mine, on 2011's npcs and op slots.
//!
//! - Sedridor (npc 300, the Wizards' Tower basement) teleports on op3, as 289
//!   has it, once the quest is complete; before, he says it needs the quest.
//! - Aubury (npc 5913, Varrock's rune shop) trades on op4 and teleports on op5
//!   in 2011 (cache 786), where 289 traded on op3 and teleported on op4.
//!
//! The ops are the client's (OPNPC3: `p1_alt3(ctrl) p2_alt2(nid)`; OPNPC4:
//! `p1_alt3(ctrl) p2_alt3(nid)`; OPNPC5: `p1_alt2(ctrl) p2_alt1(nid)`). The
//! essence mine is map square 45,75.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{converse, settexts, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

const SEDRIDOR: u16 = 300;
const AUBURY: u16 = 5913;
const SHOP: u16 = 620;
const COMPLETE: i32 = 6;

// The cases share one loaded engine and each starts with a reset, which would
// log the others' players out: one at a time.
static TURN: Mutex<()> = Mutex::new(());

/// The spawned npc `id`: its nid and tile.
fn npc(engine: &Engine, id: u16) -> (u16, u16, u16) {
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == id)
        .map(|n| (n.npc.uid.nid(), n.npc.pathing.coord))
        .map(|(nid, c)| (nid, c.x(), c.z()))
        .next()
        .unwrap_or_else(|| panic!("npc {id} is spawned"))
}

/// A player beside npc `id`, with Rune Mysteries at `stage`.
fn visitor(engine: &mut Engine, name: &str, id: u16, stage: i32) -> (HarnessClient, u16) {
    engine.harness_reset();
    let (nid, x, z) = npc(engine, id);
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    // A tile one step from the npc reaches, so no wall stands between them (npcs wander).
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .filter(|&(dx, dz)| rsmod::can_travel(0, x, z, dx, dz, 1, 0, CollisionType::Normal))
        .map(|(dx, dz)| {
            (
                x.wrapping_add_signed(dx as i16),
                z.wrapping_add_signed(dz as i16),
            )
        })
        .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
        .expect("a walkable tile beside the npc");
    engine.harness_telejump(client.pid, sx, 0, sz);
    let varp = *engine.cache.varps.debugnames.get("runemysteries").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(varp, VarValue::Int(stage));
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    (client, nid)
}

/// Cycles `n` ticks: the game messages said.
fn run(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

fn in_the_mine(engine: &Engine, pid: u16) -> bool {
    let c = engine.harness_player(pid).unwrap().pathing.coord;
    c.x() / 64 == 45 && c.z() / 64 == 75
}

#[test]
fn sedridor_teleports_a_player_who_finished_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, "apprentice", SEDRIDOR, COMPLETE);
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt2(nid as i32);
        client.send(ClientProt::OpNpc3, &p.finish());
        let said = run(engine, &mut client, 12);
        assert!(
            in_the_mine(engine, client.pid),
            "teleported into the essence mine; said {said:?}"
        );

        // Before the quest is done, he will not.
        let (mut client, nid) = visitor(engine, "novice", SEDRIDOR, 0);
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt2(nid as i32);
        client.send(ClientProt::OpNpc3, &p.finish());
        let said = run(engine, &mut client, 8);
        assert!(!in_the_mine(engine, client.pid), "stays; said {said:?}");
        assert_eq!(
            said,
            ["You need to have completed the Rune Mysteries Quest to use this feature."]
        );
    });
}

#[test]
fn aubury_trades_on_op4_and_teleports_on_op5() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, "runebuyer", AUBURY, COMPLETE);
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt3(nid as i32);
        client.send(ClientProt::OpNpc4, &p.finish());
        let said = run(engine, &mut client, 6);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(SHOP),
            "the rune shop is open; said {said:?}"
        );

        let (mut client, nid) = visitor(engine, "runetraveller", AUBURY, COMPLETE);
        let mut p = ClientPacket::new();
        p.p1_alt2(0).p2_alt1(nid as i32);
        client.send(ClientProt::OpNpc5, &p.finish());
        let said = run(engine, &mut client, 12);
        assert!(
            in_the_mine(engine, client.pid),
            "teleported into the essence mine; said {said:?}"
        );
    });
}

/// 2011's Sedridor (2011-05-22 recording, YouTube VIE8MXrw-7M, 131.9-156.6s): "That's me", the
/// talisman taken without a choice, "Wow! This is incredible!", the research package for Aubury
/// and his directions to Varrock's south-east quarter.
#[test]
fn sedridor_takes_the_talisman_and_sends_a_research_package() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, "sedridor_2011", SEDRIDOR, 1);
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let talisman = *engine.cache.objs.debugnames.get("air_talisman").unwrap();
        let package = *engine
            .cache
            .objs
            .debugnames
            .get("research_package")
            .unwrap();
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .set(0, talisman, 1);
        talk_to(&mut client, nid);
        let talk = converse(
            engine,
            &mut client,
            &["I'm looking for the head wizard.", "Yes, certainly."],
            200,
        );
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        for line in [
            "Welcome, adventurer, to the world-renowned Wizards' Tower. How may I help you?",
            "That's me, but why would you be doing that?",
            "Just some crude amulet, I'll wager.",
            "Okay, here you are.",
            "You give the talisman to the wizard.",
            "Wow! This is incredible!",
            "It is a secret so powerful that it destroyed the original Wizards' Tower many centuries ago!",
            "Aubury's rune store is in the south-east quarter.",
            "The head wizard gives you a research package.",
        ] {
            assert!(talk.said(line), "2011 shows {line:?}; {:?}", talk.texts);
        }
        assert!(
            !talk.said("No, I'll only give it to the head wizard."),
            "2011 asks nothing before the talisman"
        );
        let pack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&inv)
            .unwrap();
        assert_eq!(pack.total(talisman), 0, "the talisman is given");
        assert_eq!(pack.total(package), 1, "the research package");
    });
}

/// 2011's ending (2011-05-22 recording, 301.5-321.5s): Sedridor takes the research notes, tells
/// the rune essence's story in 2011's words, and hands back the air talisman in an item box.
#[test]
fn sedridor_takes_the_notes_and_returns_the_talisman() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, "sedridor_notes", SEDRIDOR, 5);
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let notes = *engine.cache.objs.debugnames.get("research_notes").unwrap();
        let talisman = *engine.cache.objs.debugnames.get("air_talisman").unwrap();
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap()
            .set(0, notes, 1);
        talk_to(&mut client, nid);
        let talk = converse(engine, &mut client, &[], 300);
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        for line in [
            "May I have them?",
            "You have been nothing but helpful, adventurer. In return, I can let you in on the secret of our research.",
            "So, only you and Aubury know the teleport spell to the rune essence?",
            "No, there are others. When you speak to them, they will know you and grant you access to that place when asked.",
            "You give the research notes to Sedridor. He gives you an air talisman.",
        ] {
            assert!(talk.said(line), "2011 shows {line:?}; {:?}", talk.texts);
        }
        let mut scroll = talk.texts.clone();
        for _ in 0..4 {
            engine.cycle();
            scroll.extend(settexts(&client.packets()).into_iter().map(|(_, t)| t));
        }
        // 2011's scroll (2011-05-15 NBvmMylvOY8, 459s), its reward wrapped after "Runecrafting".
        for line in [
            "You have completed Rune Mysteries!",
            "Access to the Runecrafting",
            "skill and rune essence mine",
            "Air talisman",
        ] {
            assert!(
                scroll.iter().any(|t| t == line),
                "the 2011 scroll shows {line:?}; {scroll:?}"
            );
        }
        let pack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&inv)
            .unwrap();
        assert_eq!(pack.total(notes), 0, "the notes are given");
        assert_eq!(pack.total(talisman), 1, "the air talisman comes back");
        let varp = *engine.cache.varps.debugnames.get("runemysteries").unwrap();
        assert_eq!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .vars
                .get(varp)
                .as_int(),
            COMPLETE,
            "Rune Mysteries complete"
        );
    });
}

/// Where Aubury's 2011 teleport lands: the bridge north of the Wizards' Tower
/// (refs/runemys/sg/tp_full.png, pinned in client B by its minimap).
const BRIDGE: (u16, u16) = (3113, 3186);

/// 2011's hand-over (2011-05-22 recording, YouTube VIE8MXrw-7M, 229-247s): the package for the
/// notes at once, without 289's second talk, then "Teleport back to the Wizards' Tower?", Yes,
/// and his essence-mine cast onto the bridge north of the tower.
#[test]
fn aubury_takes_the_package_and_teleports_the_player_back() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let package = *engine
            .cache
            .objs
            .debugnames
            .get("research_package")
            .unwrap();
        let notes = *engine.cache.objs.debugnames.get("research_notes").unwrap();
        let varp = *engine.cache.varps.debugnames.get("runemysteries").unwrap();
        for (name, answer) in [
            ("aubury_yes", "Yes, please."),
            ("aubury_no", "No, thank you."),
        ] {
            let (mut client, nid) = visitor(engine, name, AUBURY, 3);
            engine
                .harness_player_mut(client.pid)
                .unwrap()
                .invs
                .get_mut(&inv)
                .unwrap()
                .set(0, package, 1);
            let start = engine.harness_player(client.pid).unwrap().pathing.coord;
            talk_to(&mut client, nid);
            let talk = converse(
                engine,
                &mut client,
                &["I've been sent here with a package for you.", answer],
                300,
            );
            assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
            for line in [
                "Do you want to buy some runes?",
                "I've been sent here with a package for you. It's from Sedridor, the head wizard at the Wizards' Tower.",
                "Really? Surely he can't have...? Please, let me have it.",
                "You hand Aubury the research package.",
                "My gratitude, adventurer, for bringing me this research package.",
                "Combined with the information I have already collated regarding rune essence, I think we have finally unlocked the power to...",
                "No, I'm getting ahead of myself. Take this summary of my research back to Sedridor in the basement of the Wizards' Tower. He will know whether or not to let you in on our little secret.",
                "Aubury gives you his research notes.",
                "Now, I'm sure I can spare a couple of runes for such a worthy cause as these notes. Do you want me to teleport you back?",
                "Teleport back to the Wizards' Tower?",
            ] {
                assert!(talk.said(line), "2011 shows {line:?}; {:?}", talk.texts);
            }
            for _ in 0..10 {
                engine.cycle();
            }
            let player = engine.harness_player(client.pid).unwrap();
            let pack = player.invs.get(&inv).unwrap();
            assert_eq!(pack.total(package), 0, "the package is given");
            assert_eq!(pack.total(notes), 1, "the notes at once");
            assert_eq!(player.vars.get(varp).as_int(), 5, "notes received");
            let at = player.pathing.coord;
            if answer == "Yes, please." {
                assert_eq!(
                    (at.x(), at.z(), at.y()),
                    (BRIDGE.0, BRIDGE.1, 0),
                    "on the bridge"
                );
            } else {
                // Talk-to walks a step up to him, so the map square, not the tile.
                assert_eq!(
                    (at.x() / 64, at.z() / 64, at.y()),
                    (start.x() / 64, start.z() / 64, 0),
                    "stays in Varrock, at {:?}",
                    (at.x(), at.z())
                );
            }
        }
    });
}
