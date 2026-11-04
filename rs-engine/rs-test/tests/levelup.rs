//! 289's level-up at 2011, and 2011's stats-tab guide.
//!
//! A level-up says 289's message and plays 289's jingle (2011's by its notes),
//! and opens 2011's one level-up box (interface 740) with void-634's two lines
//! and the stat's icon (varbit 4757, `level_up_icon`); the stat's icon then
//! flashes in the stats tab (its bit of varp 1179, `skill_stat_flash`, which
//! clientscript 970 reads per component).
//!
//! In the stats tab (interface 320) a click on a flashing stat opens 2011's
//! level-up details (741) once, at varp 1230 = the stat's menu index times 8,
//! and stops the flash; a click on a quiet stat opens 2011's guide (499) at
//! varp 965 = its menu index. The click is IF_BUTTON1 as the client writes it,
//! `p4(com) ip2_alt1(obj) ip2(slot)`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const ATTACK: usize = 0;
/// 83 xp, level 2, in the engine's tenths.
const LEVEL_2_XP: i32 = 830;
const LEVELUP: u16 = 740;
const LEVELUP_DETAILS: u16 = 741;
const SKILL_GUIDE: u16 = 499;
/// The stats tab's attack: its View menu (void-634's id).
const STATS_ATTACK: i32 = 320 << 16 | 200;
const SKILL_STAT_FLASH: u16 = 1179;
const LEVEL_UP_DETAILS: u16 = 1230;
const SKILL_GUIDE_VARP: u16 = 965;
/// 289's `advance attack`: 2011's jingle 29 (void-634's level_up_attack).
const ADVANCE_ATTACK: u16 = 29;

fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<(u32, String)> {
    let opcode = ServerProt::IfSetText as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, p)| {
            let com = u32::from_be_bytes([p[0], p[1], p[2], p[3]]);
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            (com, String::from_utf8_lossy(&p[4..end]).into_owned())
        })
        .collect()
}

/// Each MIDI_JINGLE's jingle: `ip2_alt1(id) p3_alt2(length) p1(volume)`.
fn jingles(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    let opcode = ServerProt::MidiJingle as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, p)| (p[0].wrapping_sub(128) as u16) | (p[1] as u16) << 8)
        .collect()
}

fn varp(engine: &Engine, client: &HarnessClient, id: u16) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

/// Clicks `com`; the game messages said meanwhile.
fn click(engine: &mut Engine, client: &mut HarnessClient, com: i32) -> Vec<String> {
    let mut p = ClientPacket::new();
    p.p4(com).ip2_alt1(0xFFFF).ip2(0xFFFF);
    client.send(ClientProt::IfButton1, &p.finish());
    let mut said = Vec::new();
    for _ in 0..2 {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

#[test]
fn an_attack_level_opens_2011s_box_and_flashes_the_stat() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("leveller").expect("login");
        engine.cycle();
        client.packets();

        engine.harness_add_xp(client.pid, ATTACK, LEVEL_2_XP);
        let (mut texts, mut said, mut played) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..3 {
            engine.cycle();
            let packets = client.packets();
            texts.extend(settexts(&packets));
            said.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, t)| t),
            );
            played.extend(jingles(&packets));
        }
        assert!(
            is_open(engine, &client, LEVELUP),
            "the level-up box; said {said:?}"
        );
        let text = |com: u32| {
            texts
                .iter()
                .find(|(c, _)| *c == (LEVELUP as u32) << 16 | com)
                .map(|(_, t)| t.as_str())
        };
        assert_eq!(
            text(0),
            Some("Congratulations! You've just advanced an Attack level!")
        );
        assert_eq!(text(1), Some("You have now reached level 2!"));
        assert_eq!(
            said,
            ["Congratulations, you just advanced an attack level."]
        );
        assert_eq!(played, [ADVANCE_ATTACK]);
        assert_eq!(
            varp(engine, &client, SKILL_STAT_FLASH),
            1 | 1 << 26,
            "attack's flash bit and its icon (1)"
        );

        // "Click here to continue" (740:3): RESUME_PAUSEBUTTON,
        // `p2(slot) p4_alt2(com)`.
        let mut p = ClientPacket::new();
        p.p2(0xFFFF).p4_alt2((LEVELUP as i32) << 16 | 3);
        client.send(ClientProt::ResumePauseButton, &p.finish());
        for _ in 0..2 {
            engine.cycle();
        }
        assert!(!is_open(engine, &client, LEVELUP), "the box closes");

        // The flashing stat opens its level-up details once.
        let said = click(engine, &mut client, STATS_ATTACK);
        assert!(
            is_open(engine, &client, LEVELUP_DETAILS),
            "lvl_scroll; said {said:?}"
        );
        // In the full-size slot like the guide: its root is the same
        // backdrop sized to its parent.
        let player = engine.harness_player(client.pid).unwrap();
        let wide = match player.toplevel {
            548 => 8,
            746 => 6,
            other => panic!("toplevel {other}"),
        };
        assert_eq!(
            player.modal_main_parent,
            (player.toplevel as i32) << 16 | wide,
            "lvl_scroll in the full-size slot"
        );
        assert_eq!(
            varp(engine, &client, LEVEL_UP_DETAILS),
            8,
            "attack is 1 in the menu"
        );
        assert_eq!(
            varp(engine, &client, SKILL_STAT_FLASH) & 1,
            0,
            "the flash stops"
        );

        // Then it opens attack's guide.
        let said = click(engine, &mut client, STATS_ATTACK);
        assert!(
            is_open(engine, &client, SKILL_GUIDE),
            "skill_guide_v2; said {said:?}"
        );
        assert_eq!(varp(engine, &client, SKILL_GUIDE_VARP), 1);
        // In the toplevel's full-size slot (void-634's `wide_screen`), where
        // the guide's own backdrop fills the view: 548:8 fixed, 746:6
        // resizable. In the 512x334 main slot it showed as a tinted box.
        let player = engine.harness_player(client.pid).unwrap();
        let wide = match player.toplevel {
            548 => 8,
            746 => 6,
            other => panic!("toplevel {other}"),
        };
        assert_eq!(
            player.modal_main_parent,
            (player.toplevel as i32) << 16 | wide
        );
    });
}

/// 2011's jingle "changes with each skill to be slightly more complex,
/// whenever the skill reaches level 50" (the 2011 wiki's Skills page): 289's
/// second jingle, `advance attack2` (2011's 30), from level 50, where 289
/// played it at the levels that unlock something.
#[test]
fn from_level_50_a_level_plays_the_second_jingle() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("fifty").expect("login");
        engine.cycle();
        client.packets();

        // Level 50 is 101,333 xp; level 49 is 92,224.
        engine.harness_add_xp(client.pid, ATTACK, 922_240);
        for _ in 0..3 {
            engine.cycle();
        }
        let mut p = ClientPacket::new();
        p.p2(0xFFFF).p4_alt2((LEVELUP as i32) << 16 | 3);
        client.send(ClientProt::ResumePauseButton, &p.finish());
        for _ in 0..2 {
            engine.cycle();
        }
        client.packets();

        engine.harness_add_xp(client.pid, ATTACK, 1_013_330 - 922_240);
        let mut played = Vec::new();
        for _ in 0..3 {
            engine.cycle();
            played.extend(jingles(&client.packets()));
        }
        assert_eq!(played, [ADVANCE_ATTACK + 1], "advance attack2 at level 50");
    });
}
