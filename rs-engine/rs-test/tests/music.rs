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
//! child at its index * 2 (clientscript 828), and Play is IF_BUTTON1 on it.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;

/// Tutorial Island (0_48_48), the harness's spawn: 289's Newbie Melody
/// (track 89, song 62), which 2011 unlocks automatically.
const NEWBIE_MELODY: u16 = 62;
/// Varrock's palace (0_50_54): Adventure (track 0, song 177).
const VARROCK: (u16, u16) = (3212, 3473);
const ADVENTURE: u16 = 177;
/// The music player's "Playing" text (music_v3, 187:4).
fn playing() -> i32 {
    com("music_v3:now_playing_text")
}

/// The music player's list (187:1).
fn tracks() -> i32 {
    com("music_v3:jukebox")
}

/// Scape Main (track 150), unlocked automatically: song 16.
const SCAPE_MAIN: u16 = 16;

/// Each MIDI_SONG's song: `p1_alt3 volume, p2_alt3 song, p1_alt1 delay`.
fn songs(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    server::songs(packets)
}

/// Each IF_SETTEXT as `(com, text)`: `p4 com, pjstr text`.
fn texts(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, String)> {
    server::settexts(packets)
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
    engine
        .cache
        .varps
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no varp {name}"))
        .id
        .g2()
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
        // no message. 530's automatic tracks (enum 1349) leave it out.
        let sent = ticks(engine, &mut client, 3);
        assert_eq!(songs(&sent), [NEWBIE_MELODY], "the spawn square's track");
        assert!(texts(&sent).contains(&(playing(), "Newbie Melody".to_string())));
        assert_eq!(
            HarnessClient::game_messages(&sent)
                .iter()
                .any(|(_, t)| t.contains("unlocked")),
            cfg!(before_647),
            "Newbie Melody unlocks automatically from 647"
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
        assert!(texts(&sent).contains(&(playing(), "Adventure".to_string())));
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
/// the track's name, its index * 2 (530: its index).
fn play(client: &mut HarnessClient, track: i32) {
    let slot = if cfg!(before_647) { track } else { track * 2 };
    client.if_button(1, tracks(), slot as u16, TypeId::NULL);
}

/// Each IF_SETEVENTS as `(com, from, to, ops)`: `p2_alt2 from, p4_alt3
/// events, p4_alt3 com, p2_alt3 to`.
fn events(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16, u16, i32)> {
    server::events(packets)
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
            .find(|e| e.0 == tracks())
            .expect("the list's events");
        // 530's list ends at 646 (login.rs2).
        #[cfg(before_647)]
        let last = 646;
        #[cfg(since_647)]
        let last = 1959;
        assert_eq!((ops.1, ops.2, ops.3 & 0x7fe), (0, last, 0b110));

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
        assert!(texts(&sent).contains(&(playing(), "Scape Main".to_string())));
    });
}
