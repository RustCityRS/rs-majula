//! Voice-over: `sound_speech_volume` sends the client's VORBIS_SPEECH_SOUND (7) and
//! `sound_speech_stop` its VORBIS_SPEECH_STOP (66), through the dev cheats in
//! `_test/scripts/cheats/cheat_speech.rs2`.
//!
//! The client reads `g2 clip, g1 loops, g2 delay, g1 volume` (`MapAreaConfig.java:1921`)
//! and nothing for the stop (`:401`).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

/// Lumbridge, east of the castle.
const HOME: (u16, u16) = (3222, 3218);

fn developer(engine: &mut Engine, name: &str) -> HarnessClient {
    let mut client = engine.harness_login_returning(name).expect("login");
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .staff_mod_level = StaffModLevel::Maintainer;
    engine.cycle();
    engine.harness_telejump(client.pid, HOME.0, 0, HOME.1);
    for _ in 0..12 {
        engine.cycle();
    }
    client.packets();
    client
}

fn cheat(engine: &mut Engine, client: &mut HarnessClient, input: &str) -> Vec<(u16, Vec<u8>)> {
    let mut p = ClientPacket::new();
    p.p1(0).p1(0).pjstr(input);
    client.send(ClientProt::ClientCheat, &p.finish());
    let mut packets = Vec::new();
    for _ in 0..2 {
        engine.cycle();
        packets.extend(client.packets());
    }
    packets
}

fn bodies(packets: &[(u16, Vec<u8>)], prot: ServerProt) -> Vec<Vec<u8>> {
    let opcode = prot as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, body)| body.clone())
        .collect()
}

#[test]
fn speech_plays_and_stops_a_voice_over_clip() {
    runner::on_loaded_engine(|engine| {
        let mut client = developer(engine, "speech_tester");

        // Clip 687: Sir Vant's farewell (docs/647-unstable-foundations.md §3.6).
        let packets = cheat(engine, &mut client, "~speech 687");
        assert_eq!(
            bodies(&packets, ServerProt::VorbisSpeechSound),
            vec![vec![0x02, 0xaf, 1, 0, 0, 255]],
            "one VORBIS_SPEECH_SOUND: clip 687, once, no delay, full volume"
        );

        let packets = cheat(engine, &mut client, "~speechstop");
        assert_eq!(
            bodies(&packets, ServerProt::VorbisSpeechStop),
            vec![Vec::<u8>::new()],
            "one empty VORBIS_SPEECH_STOP"
        );
        assert!(bodies(&packets, ServerProt::VorbisSpeechSound).is_empty());
    });
}
