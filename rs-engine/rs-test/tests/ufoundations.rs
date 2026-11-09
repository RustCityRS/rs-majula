//! Unstable Foundations' start (docs/647-unstable-foundations.md, build plan stages 1 and 3).
//!
//! - Character creation's end (`[queue,character_created]`) sets varp 281 to 5, the start value
//!   of quest config 131 (`op3_counted_g2_g4_g4=1,281,5,1000`), not 1001.
//! - It puts the player in their own copy of the cellar, square 57_77, on local (30, 9): the
//!   2011 recordings' minimaps fit there (design note 3.5 #1).
//! - The plank, ground obj 15276, lies on local (35, 15) (3.5 #2).
//! - The Mentor plays (3.5 #6).
//! - Sir Vant (8850) stands on local (32, 16), the north doorway, and his squire on (33, 15), and
//!   neither wanders off (the 2011 minimaps show them there at 8.5 s and 80 s).
//! - A region nobody is in is freed after 50 ticks; logging back in mid-tutorial builds a fresh one
//!   at the start tile (no source says where 2011 put them).

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
use rs_vm::subject::ScriptSubject;

/// Square 57_77's south-west tile.
const SQUARE: (u16, u16) = (57 * 64, 77 * 64);
const NONE: i32 = 0xFFFF;
const INVENTORY: u16 = 149;
const OPTIONS: u16 = 261;
const CONTROLS: u16 = 897;
const TASK_SIDE: u16 = 1056;
/// The task system's pinned Task (cs2 3973 draws it).
const PINNED_TASK: u16 = 8576;
/// The task system popup's Task (cs2 3968).
const POPUP_TASK: u16 = 1425;
/// The side tab the yellow arrow flashes, + 1 (cs2 117).
const FLASH_SIDE: u16 = 3756;

fn varbit(engine: &Engine, client: &HarnessClient, id: u16) -> i32 {
    let bit = engine.cache.varbits.get_by_id(id).unwrap();
    let value = engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(bit.basevar)
        .as_int() as u32;
    let width = u32::from(bit.end_bit - bit.start_bit) + 1;
    ((value >> bit.start_bit) & ((1u64 << width) - 1) as u32) as i32
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

/// An arrow key on the Controls overlay's listener: IF_BUTTON1 on 897:0.
fn arrow_key(client: &mut HarnessClient) {
    let mut p = ClientPacket::new();
    p.p4(i32::from(CONTROLS) << 16).p2_alt3(NONE).p2_alt1(NONE);
    client.send(ClientProt::IfButton1, &p.finish());
}

fn varp(engine: &Engine, name: &str) -> u16 {
    *engine.cache.varps.debugnames.get(name).unwrap()
}

fn progress(engine: &Engine, client: &HarnessClient) -> i32 {
    let id = varp(engine, "learning_the_ropes");
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

/// The square 57_77 tile the player stands on a copy of, as local (x, z).
fn cellar_tile(engine: &Engine, client: &HarnessClient) -> Option<(u16, u16)> {
    let coord = engine.harness_player(client.pid)?.pathing.coord;
    let source = engine.regions.source(coord)?;
    Some((source.x() - SQUARE.0, source.z() - SQUARE.1))
}

fn create_character(engine: &mut Engine, client: &mut HarnessClient) {
    let uid = engine.harness_player(client.pid).unwrap().uid;
    engine
        .run_script_by_name(
            "[queue,character_created]",
            Some(ScriptSubject::Player(uid)),
            None,
            Some(true),
            Some(true),
            None,
        )
        .expect("character_created");
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
}

#[test]
fn character_creation_starts_unstable_foundations_in_the_cellar() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login("uf_newcomer").expect("login");
        engine.cycle();
        client.packets();
        assert_eq!(
            progress(engine, &client),
            0,
            "a new account designs its character first"
        );

        create_character(engine, &mut client);
        // Creation sets quest config 131's start value, 5; the Controls overlay takes it to 6.
        assert_eq!(progress(engine, &client), 6, "the Controls overlay's stage");
        assert_eq!(
            cellar_tile(engine, &client),
            Some((30, 9)),
            "the start tile"
        );

        let player = engine.harness_player(client.pid).unwrap();
        let here = player.pathing.coord;
        let (x, z) = (here.x() + 5, here.z() + 6);
        let plank = *engine.cache.objs.debugnames.get("plank_15276").unwrap();
        assert!(
            engine
                .zones
                .find_obj(x, 0, z, plank, Some(player.uid.username37()))
                .is_some(),
            "the plank on local (35, 15)"
        );

        let mentor = *engine.cache.midi_ids.get("the mentor").unwrap();
        let song = varp(engine, "currentsong");
        assert_eq!(
            player.vars.get(song).as_int(),
            mentor as i32,
            "The Mentor plays"
        );

        // The side panel: the task system and options only (8jKH_7To46o @56), Rotate Your Camera
        // pinned, the Controls overlay up.
        assert!(is_open(engine, &client, TASK_SIDE), "the task system tab");
        assert!(is_open(engine, &client, OPTIONS), "the options tab");
        assert!(!is_open(engine, &client, INVENTORY), "no inventory tab yet");
        assert!(is_open(engine, &client, CONTROLS), "the Controls overlay");
        assert_eq!(
            varbit(engine, &client, PINNED_TASK),
            3522,
            "Rotate Your Camera"
        );
        assert_eq!(
            varbit(engine, &client, FLASH_SIDE),
            2,
            "the arrow at the task tab"
        );

        // An arrow key turns the camera: Rotate Your Camera is done.
        arrow_key(&mut client);
        for _ in 0..2 {
            engine.cycle();
        }
        client.packets();
        assert_eq!(
            progress(engine, &client),
            7,
            "Rotate Your Camera done (cs2 3227)"
        );
        assert!(!is_open(engine, &client, CONTROLS), "the overlay closes");
        let popup = engine
            .harness_player(client.pid)
            .unwrap()
            .varcs
            .get(&POPUP_TASK)
            .copied();
        assert_eq!(popup, Some(3522), "TASK COMPLETE! Rotate Your Camera");
        assert_eq!(
            varbit(engine, &client, PINNED_TASK),
            3500,
            "Talk to Sir Vant"
        );
        assert_eq!(varbit(engine, &client, FLASH_SIDE), 0, "no arrow after");

        // Talk to Sir Vant: "Hello!" and his warning, each with its voiced take (s001, s002).
        let vant = *engine.cache.npcs.debugnames.get("sir_vant_8850").unwrap();
        let nid = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .find(|n| n.npc.uid.id() == vant)
            .map(|n| n.npc.uid.nid())
            .expect("Sir Vant in the cellar");
        talk_to(&mut client, nid);
        let talk = converse(engine, &mut client, &[], 80);
        assert!(talk.said("Hello!"), "{:?}", talk.texts);
        assert!(
            talk.said("Wait a moment - there's a dragon coming!"),
            "{:?}",
            talk.texts
        );
        let clips: Vec<u16> = talk
            .packets
            .iter()
            .filter(|(op, _)| *op == ServerProt::VorbisSpeechSound as u16)
            .map(|(_, body)| u16::from_be_bytes([body[0], body[1]]))
            .collect();
        assert_eq!(
            clips,
            vec![1850, 770],
            "the male voice's Hello. and Sir Vant's line"
        );
        assert_eq!(
            progress(engine, &client),
            9,
            "talked; the dragon cutscene's stage"
        );

        for _ in 0..60 {
            engine.cycle();
        }
        client.packets();
        for (name, tile) in [("sir_vant_8850", (32, 16)), ("squire_8857", (33, 15))] {
            let id = *engine.cache.npcs.debugnames.get(name).unwrap();
            let at = engine
                .npc_list
                .npcs
                .iter()
                .flatten()
                .find(|n| n.npc.uid.id() == id)
                .map(|n| n.npc.pathing.coord)
                .unwrap_or_else(|| panic!("{name} is in the cellar"));
            let source = engine.regions.source(at).expect("in the cellar copy");
            assert_eq!(
                (source.x() - SQUARE.0, source.z() - SQUARE.1),
                tile,
                "{name} keeps his tile"
            );
        }
    });
}
