//! 289's music on 2011's music system.
//!
//! 289 plays a mapsquare's track as the player enters it (`move.rs2`'s
//! `[mapzone]` triggers, `music_playbyregion`) and unlocks it. 2011 keeps
//! the unlock, announces it in red, lets a song play to its end and then
//! plays the area's (the 2011 wiki's Audio Week update, 5 February 2008).
//! A track's unlock is a bit of the musicmulti varps by its place in the
//! music player's list (clientscript 837), and its song is enum 1351's.
//!
//! 289's music tab is 2011's music player (187): a track's name is com1's
//! child at its index * 2 (clientscript 828), and Play is IF_BUTTON1 on it
//! (`Class199.method1317`: `p4(com) p2_alt3(obj) p2_alt1(slot)`).

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

/// Tutorial Island (0_48_48), the harness's spawn: 289's Newbie Melody
/// (track 89, song 62), which 2011 unlocks automatically.
const NEWBIE_MELODY: u16 = 62;
/// Varrock's palace (0_50_54): Adventure (track 0, song 177).
const VARROCK: (u16, u16) = (3212, 3473);
const ADVENTURE: u16 = 177;
/// The music player's "Playing" text (music_v3, 187:4).
const PLAYING: i32 = 187 << 16 | 4;
/// The music player's list (187:1).
const TRACKS: i32 = 187 << 16 | 1;
/// Scape Main (track 150), unlocked automatically: song 16.
const SCAPE_MAIN: u16 = 16;

fn of(packets: &[(u16, Vec<u8>)], prot: ServerProt) -> impl Iterator<Item = &Vec<u8>> {
    let opcode = prot as u16;
    packets
        .iter()
        .filter(move |(op, _)| *op == opcode)
        .map(|(_, p)| p)
}

/// Each MIDI_SONG's song: `p1_alt3 volume, p2_alt3 song, p1_alt1 delay`.
fn songs(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    of(packets, ServerProt::MidiSong)
        .map(|p| u16::from_le_bytes([p[1].wrapping_sub(128), p[2]]))
        .collect()
}

/// Each IF_SETTEXT as `(com, text)`: `p4 com, pjstr text`.
fn texts(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, String)> {
    of(packets, ServerProt::IfSetText)
        .map(|p| {
            let com = i32::from_be_bytes([p[0], p[1], p[2], p[3]]);
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            (com, String::from_utf8_lossy(&p[4..end]).into_owned())
        })
        .collect()
}

/// Runs `n` ticks and gathers what the client was sent.
fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::new();
    for _ in 0..n {
        engine.cycle();
        out.extend(client.packets());
    }
    out
}

fn varp(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .varps
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no varp {name}"))
}

fn unlocked(engine: &Engine, pid: u16, track: i32) -> bool {
    let var = varp(engine, &format!("musicmulti_{}", track / 32 + 1));
    let player = engine.harness_player(pid).unwrap();
    player.vars.get(var).as_int() & (1 << (track % 32)) != 0
}

#[test]
fn a_square_plays_its_track_and_unlocks_it() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("listener").expect("login");

        // Logging in: the spawn square's track, unlocked automatically, so
        // no message.
        let sent = ticks(engine, &mut client, 3);
        assert_eq!(songs(&sent), [NEWBIE_MELODY], "the spawn square's track");
        assert!(texts(&sent).contains(&(PLAYING, "Newbie Melody".to_string())));
        assert!(
            !HarnessClient::game_messages(&sent)
                .iter()
                .any(|(_, t)| t.contains("unlocked")),
            "Newbie Melody unlocks automatically"
        );
        assert!(unlocked(engine, client.pid, 89), "Newbie Melody");
        assert!(
            unlocked(engine, client.pid, 150),
            "Scape Main, automatic too"
        );
        assert!(!unlocked(engine, client.pid, 0), "not Adventure yet");

        // A square whose track is new: unlocked, announced, and played at
        // once over the song playing.
        engine.harness_telejump(client.pid, VARROCK.0, 0, VARROCK.1);
        let sent = ticks(engine, &mut client, 3);
        assert_eq!(
            HarnessClient::game_messages(&sent),
            [(
                0,
                "<col=ff0000>You have unlocked a new music track: Adventure.".to_string()
            )]
        );
        assert_eq!(songs(&sent), [ADVENTURE]);
        assert!(texts(&sent).contains(&(PLAYING, "Adventure".to_string())));
        assert!(unlocked(engine, client.pid, 0), "Adventure");

        // Back to a square whose track is unlocked: the song plays on.
        engine.harness_telejump(client.pid, 3094, 0, 3106);
        let sent = ticks(engine, &mut client, 3);
        assert!(songs(&sent).is_empty(), "Adventure plays to its end");

        // Its end: the square's track follows.
        let start = varp(engine, "musicstart");
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.vars.set(start, VarValue::Int(-100_000));
        let sent = ticks(engine, &mut client, 2);
        assert_eq!(songs(&sent), [NEWBIE_MELODY], "the square's track next");
    });
}

/// Clicks Play on a track of the music player's list: IF_BUTTON1 on 187:1 at
/// the track's name, its index * 2 (`p4 com, p2_alt3 obj, p2_alt1 slot`).
fn play(client: &mut HarnessClient, track: i32) {
    let mut p = ClientPacket::new();
    p.p4(TRACKS).p2_alt3(0xFFFF).p2_alt1(track * 2);
    client.send(ClientProt::IfButton1, &p.finish());
}

/// Each IF_SETEVENTS as `(com, from, to, ops)`: `p2_alt2 from, p4_alt3
/// events, p4_alt3 com, p2_alt3 to`.
fn events(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16, u16, i32)> {
    of(packets, ServerProt::IfSetEvents)
        .map(|p| {
            let from = u16::from_be_bytes([p[0], p[1].wrapping_sub(128)]);
            let ops = i32::from_be_bytes([p[3], p[2], p[5], p[4]]);
            let com = i32::from_be_bytes([p[7], p[6], p[9], p[8]]);
            let to = u16::from_le_bytes([p[10].wrapping_sub(128), p[11]]);
            (com, from, to, ops)
        })
        .collect()
}

#[test]
fn the_music_player_plays_an_unlocked_track() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("player").expect("login");
        let sent = ticks(engine, &mut client, 3);
        // Play and Unlock hint on every name of the list (2 a track, 980
        // places); the playlist's ops stay off.
        let ops = events(&sent)
            .into_iter()
            .find(|e| e.0 == TRACKS)
            .expect("the list's events");
        assert_eq!((ops.1, ops.2, ops.3 & 0x7fe), (0, 1959, 0b110));

        // A locked track: 289's answer (the client offers no Play on one).
        play(&mut client, 0);
        let sent = ticks(engine, &mut client, 1);
        assert_eq!(
            HarnessClient::game_messages(&sent),
            [(
                0,
                "You have not unlocked this piece of music yet!".to_string()
            )]
        );
        assert!(songs(&sent).is_empty());

        // An unlocked one plays at once, named as it plays.
        play(&mut client, 150);
        let sent = ticks(engine, &mut client, 1);
        assert_eq!(songs(&sent), [SCAPE_MAIN], "Scape Main");
        assert!(texts(&sent).contains(&(PLAYING, "Scape Main".to_string())));
    });
}
