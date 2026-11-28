//! 289's level-up at 2011, and 2011's stats-tab guide.
//!
//! A level-up says 289's message and plays 289's jingle (2011's by its notes),
//! and opens 2011's one level-up box (interface 740) with void-634's two lines
//! and the stat's icon (varpbit 4757, `level_up_icon`); the stat's icon then
//! flashes in the stats tab (its bit of varp 1179, `skill_stat_flash`, which
//! clientscript 970 reads per component).
//!
//! In the stats tab (interface 320) a click on a flashing stat opens 2011's
//! level-up details (741) once, at varp 1230 = the stat's menu index times 8,
//! and stops the flash; a click on a quiet stat opens 2011's guide (499) at
//! varp 965 = its menu index. The click is IF_BUTTON1 as the client writes it.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::varp::VarPlayerScope;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls, set_recording};

const ATTACK: usize = 0;
/// 83 xp, level 2, in the engine's tenths.
#[cfg(since_647)]
const LEVEL_2_XP: i32 = 830;
const LEVELUP: u16 = 740;
#[cfg(since_647)]
const LEVELUP_DETAILS: u16 = 741;
#[cfg(since_647)]
const SKILL_GUIDE: u16 = 499;
/// The stats tab's attack: its View menu (void-634's id).
#[cfg(since_647)]
fn stats_attack() -> i32 {
    com("stats:attack_button")
}

#[cfg(since_647)]
const SKILL_STAT_FLASH: u16 = 1179;
#[cfg(since_647)]
const LEVEL_UP_DETAILS: u16 = 1230;
#[cfg(since_647)]
const SKILL_GUIDE_VARP: u16 = 965;
/// 289's `advance attack`: 2011's jingle 29 (void-634's level_up_attack).
const ADVANCE_ATTACK: u16 = 29;
/// 2011's level-up fireworks, every level.
const LEVELUP_FIREWORKS: u16 = 4860;
/// 2011's rocket at 99, three times.
const MASTERY_ROCKET: u16 = 4859;
/// Levels 98 and 99, in the engine's tenths.
const LEVEL_98_XP: i32 = 118_056_060;
const LEVEL_99_XP: i32 = 130_344_310;

#[cfg(since_647)]
fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<(u32, String)> {
    server::settexts(packets)
        .into_iter()
        .map(|(c, t)| (c as u32, t))
        .collect()
}

/// Each MIDI_JINGLE's jingle: `p2_alt3(id) p3_alt2(length) p1(volume)`.
fn jingles(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    server::jingles(packets)
}

/// Each SYNTH_SOUND's synth and delay: `p2(synth) p1(loops) p2(delay) p1(volume) p2(speed)`.
fn synths(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, u16)> {
    server::synths(packets)
}

/// Clicks the level-up box's "Click here to continue" (740:3): RESUME_PAUSEBUTTON.
fn continue_box(client: &mut HarnessClient) {
    client.resume_pausebutton(com("interface_740:close"), 0xFFFF);
}

#[cfg(since_647)]
fn varp(engine: &Engine, client: &HarnessClient, id: u16) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

/// Clicks `com`; the game messages said meanwhile.
#[cfg(since_647)]
fn click(engine: &mut Engine, client: &mut HarnessClient, com: i32) -> Vec<String> {
    client.if_button(1, com, 0xFFFF, TypeId::NULL);
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

/// The toplevel's full-size slot: 548's `mainoverlay_clipped` fixed, 746's
/// `mainmodal_fullscreen` resizable.
#[cfg(since_647)]
fn full_size_slot(toplevel: u16) -> i32 {
    match toplevel {
        548 => com("toplevel:mainoverlay_clipped"),
        746 => com("toplevel_fullscreen:mainmodal_fullscreen"),
        other => panic!("toplevel {other}"),
    }
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

#[cfg(since_647)]
#[test]
fn an_attack_level_opens_2011s_box_and_flashes_the_stat() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("leveller").expect("login");
        engine.cycle();
        client.packets();

        engine.harness_add_xp(client.pid, ATTACK, LEVEL_2_XP);
        let (mut texts, mut said, mut played, mut sounds) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
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
            sounds.extend(synths(&packets));
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
        // 2011's chat line (xqWnfqno_1M 2010-12-12, 4KO2h2xPtdM 2011-04-30: "You've just
        // advanced a Woodcutting level! You have reached level 4.").
        assert_eq!(
            said,
            ["You've just advanced an Attack level! You have reached level 2."]
        );
        assert_eq!(played, [ADVANCE_ATTACK]);
        assert_eq!(sounds, [(LEVELUP_FIREWORKS, 0)], "the level-up fireworks");
        assert_eq!(
            varp(engine, &client, SKILL_STAT_FLASH),
            1 | 1 << 26,
            "attack's flash bit and its icon (1)"
        );

        // "Click here to continue" (740:3): RESUME_PAUSEBUTTON.
        client.resume_pausebutton(com("interface_740:close"), 0xFFFF);
        for _ in 0..2 {
            engine.cycle();
        }
        assert!(!is_open(engine, &client, LEVELUP), "the box closes");

        // The flashing stat opens its level-up details once.
        let said = click(engine, &mut client, stats_attack());
        assert!(
            is_open(engine, &client, LEVELUP_DETAILS),
            "lvl_scroll; said {said:?}"
        );
        // In the full-size slot like the guide: its root is the same
        // backdrop sized to its parent.
        let player = engine.harness_player(client.pid).unwrap();
        let wide = full_size_slot(player.toplevel);
        assert_eq!(
            player.modal_main_parent, wide,
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
        let said = click(engine, &mut client, stats_attack());
        assert!(
            is_open(engine, &client, SKILL_GUIDE),
            "skill_guide_v2; said {said:?}"
        );
        assert_eq!(varp(engine, &client, SKILL_GUIDE_VARP), 1);
        // In the toplevel's full-size slot, where the guide's own backdrop
        // fills the view: 548:8 `mainoverlay_clipped` fixed, 746:8
        // `mainmodal_fullscreen` resizable. In the 512x334 main slot it
        // showed as a tinted box.
        let player = engine.harness_player(client.pid).unwrap();
        let wide = full_size_slot(player.toplevel);
        assert_eq!(
            player.modal_main_parent,
            (player.toplevel as i32) << 16 | wide
        );
    });
}

/// 2011's 99: the box and fireworks as at any level, then, once the box is continued, the dark
/// red line and three rockets 1.2 s (60 client cycles) apart.
#[test]
fn level_99_says_well_done_and_fires_three_rockets_after_the_box() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("mastery").expect("login");
        engine.cycle();
        client.packets();

        engine.harness_add_xp(client.pid, ATTACK, LEVEL_98_XP);
        for _ in 0..3 {
            engine.cycle();
        }
        continue_box(&mut client);
        for _ in 0..2 {
            engine.cycle();
        }
        client.packets();

        engine.harness_add_xp(client.pid, ATTACK, LEVEL_99_XP - LEVEL_98_XP);
        let (mut said, mut sounds) = (Vec::new(), Vec::new());
        for _ in 0..3 {
            engine.cycle();
            let packets = client.packets();
            said.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, t)| t),
            );
            sounds.extend(synths(&packets));
        }
        assert!(is_open(engine, &client, LEVELUP), "the level-up box");
        assert_eq!(
            said,
            ["You've just advanced an Attack level! You have reached level 99."]
        );
        assert_eq!(sounds, [(LEVELUP_FIREWORKS, 0)]);

        set_recording(true);
        forget_calls();
        continue_box(&mut client);
        let (mut said, mut sounds) = (Vec::new(), Vec::new());
        for _ in 0..2 {
            engine.cycle();
            let packets = client.packets();
            said.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, t)| t),
            );
            sounds.extend(synths(&packets));
        }
        assert!(!is_open(engine, &client, LEVELUP), "the box closes");
        assert_eq!(
            said,
            ["<col=800000>Well done! You've achieved the highest possible level in this skill!"]
        );
        assert_eq!(
            sounds,
            [
                (MASTERY_ROCKET, 0),
                (MASTERY_ROCKET, 60),
                (MASTERY_ROCKET, 120)
            ]
        );
        let shown = calls_for("spotanim_pl");
        set_recording(false);
        assert_eq!(shown, ["1637,92,0"], "the three rockets' graphic");
        assert!(engine.script_errors.take().is_empty());
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
        client.resume_pausebutton(com("interface_740:close"), 0xFFFF);
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

/// "Every skill" changes at 50 (the same page): ranged and defence, which 289 gives one jingle
/// each, play the second ones cache 786 holds for them (58 `advance ranged2`, 38 `advance
/// defense2`) from level 50, and the first below it.
#[test]
fn ranged_and_defence_change_at_level_50_too() {
    runner::on_loaded_engine(|engine| {
        for (name, stat, first) in [("fiftyr", 4, 57), ("fiftyd", 1, 37)] {
            engine.harness_reset();
            let mut client = engine.harness_login_returning(name).expect("login");
            engine.cycle();
            client.packets();

            engine.harness_add_xp(client.pid, stat, 922_240);
            let mut played = Vec::new();
            for _ in 0..3 {
                engine.cycle();
                played.extend(jingles(&client.packets()));
            }
            assert_eq!(played, [first], "the first jingle at level 49");
            client.resume_pausebutton(com("interface_740:close"), 0xFFFF);
            for _ in 0..2 {
                engine.cycle();
            }
            client.packets();

            engine.harness_add_xp(client.pid, stat, 1_013_330 - 922_240);
            let mut played = Vec::new();
            for _ in 0..3 {
                engine.cycle();
                played.extend(jingles(&client.packets()));
            }
            assert_eq!(played, [first + 1], "the second jingle at level 50");
        }
    });
}

/// A stat's level-up flash (varp 1179, `skill_stat_flash`) lasts only while
/// the player stays logged in (2011 wiki, Update:Level-up Messages, 21 May
/// 2008), so it is not saved; void-634 saved it.
#[test]
fn the_stat_flash_is_not_saved() {
    runner::on_loaded_engine(|engine| {
        let flash = engine
            .cache
            .varps
            .get_by_id(TypeId::new(1179))
            .expect("varp 1179");
        assert_ne!(
            flash.scope,
            VarPlayerScope::Perm,
            "skill_stat_flash is not saved"
        );
    });
}
