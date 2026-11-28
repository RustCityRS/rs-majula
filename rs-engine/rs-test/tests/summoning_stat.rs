//! Summoning (stat 23, `animallore`) as a stat at 2011: its level-up, its place in the stats tab,
//! and its points.
//!
//! - A level opens 2011's level-up box (740) with the stat's icon and flashes its icon in the
//!   stats tab: bit 23 of varp 1179 (`skill_stat_flash`; clientscript 970 reads varbit 4755 for
//!   `stats:com_145`), with menu index 24 (enum 681) in `level_up_icon`.
//! - The stats tab's Summoning button (320:150, View) opens the level-up details once, then the
//!   guide at index 24.
//! - Summoning points are the stat's current level and come back only at an obelisk or from a
//!   potion (2011 wiki, Summoning points, 12 Dec 2010), so the stat does not regenerate.

#![cfg(since_647)]

use std::sync::Mutex;

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;

static TURN: Mutex<()> = Mutex::new(());

const ANIMALLORE: usize = 23;
const ATTACK: usize = 0;
/// 83 xp, level 2, in the engine's tenths.
const LEVEL_2_XP: i32 = 830;
const LEVELUP: u16 = 740;
const LEVELUP_DETAILS: u16 = 741;
const SKILL_GUIDE: u16 = 499;
const SKILL_STAT_FLASH: u16 = 1179;
const LEVEL_UP_DETAILS: u16 = 1230;
const SKILL_GUIDE_VARP: u16 = 965;
/// Summoning's place in 2011's skill menu (enum 681) and its flash bit (varbit 4755).
const MENU_INDEX: i32 = 24;
const FLASH_BIT: i32 = 23;
/// Void-634's `level_up_summoning`.
const ADVANCE_SUMMONING: u16 = 300;
/// The stat regeneration timer's period.
const REGEN_TICKS: usize = 100;

fn varp(engine: &Engine, client: &HarnessClient, id: u16) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

fn is_open(engine: &Engine, client: &HarnessClient, iface: u16) -> bool {
    engine
        .harness_player(client.pid)
        .unwrap()
        .is_interface_open(iface)
}

fn click(engine: &mut Engine, client: &mut HarnessClient, com: i32) {
    client.if_button(1, com, 0xFFFF, TypeId::NULL);
    for _ in 0..2 {
        engine.cycle();
    }
    client.packets();
}

#[test]
fn a_summoning_level_opens_the_box_and_flashes_the_stat() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("summoner").expect("login");
        engine.cycle();
        client.packets();

        engine.harness_add_xp(client.pid, ANIMALLORE, LEVEL_2_XP);
        let (mut texts, mut said, mut played) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..3 {
            engine.cycle();
            let packets = client.packets();
            texts.extend(server::settexts(&packets));
            said.extend(
                HarnessClient::game_messages(&packets)
                    .into_iter()
                    .map(|(_, t)| t),
            );
            played.extend(server::jingles(&packets));
        }
        assert!(
            is_open(engine, &client, LEVELUP),
            "the level-up box; said {said:?}"
        );
        let text = |com: u32| {
            texts
                .iter()
                .find(|(c, _)| *c as u32 == (LEVELUP as u32) << 16 | com)
                .map(|(_, t)| t.as_str())
        };
        assert_eq!(
            text(0),
            Some("Congratulations! You've just advanced a Summoning level!")
        );
        assert_eq!(text(1), Some("You have now reached level 2!"));
        assert_eq!(
            said,
            ["You've just advanced a Summoning level! You have reached level 2."]
        );
        assert_eq!(played, [ADVANCE_SUMMONING]);
        assert_eq!(
            varp(engine, &client, SKILL_STAT_FLASH),
            1 << FLASH_BIT | MENU_INDEX << 26,
            "Summoning's flash bit and its icon"
        );

        client.resume_pausebutton(com("interface_740:close"), 0xFFFF);
        for _ in 0..2 {
            engine.cycle();
        }

        // The flashing stat opens its level-up details once, then its guide.
        click(engine, &mut client, com("stats:com_150"));
        assert!(is_open(engine, &client, LEVELUP_DETAILS), "lvl_scroll");
        assert_eq!(varp(engine, &client, LEVEL_UP_DETAILS), MENU_INDEX * 8);
        assert_eq!(
            varp(engine, &client, SKILL_STAT_FLASH) & 1 << FLASH_BIT,
            0,
            "the flash stops"
        );
        click(engine, &mut client, com("stats:com_150"));
        assert!(is_open(engine, &client, SKILL_GUIDE), "skill_guide_v2");
        assert_eq!(varp(engine, &client, SKILL_GUIDE_VARP), MENU_INDEX);
        assert!(engine.script_errors.take().is_empty());
    });
}

#[test]
fn summoning_points_do_not_come_back_by_themselves() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("drained").expect("login");
        engine.cycle();
        client.packets();

        let player = engine.harness_player_mut(client.pid).unwrap();
        player.stats.base_levels[ANIMALLORE] = 50;
        player.stats.levels[ANIMALLORE] = 20;
        player.stats.base_levels[ATTACK] = 50;
        player.stats.levels[ATTACK] = 20;
        for tick in 0..REGEN_TICKS + 5 {
            // The engine logs a silent client out.
            if tick % 20 == 0 {
                client.send(ClientProt::NoTimeout, &[]);
            }
            engine.cycle();
            client.packets();
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert!(
            player.stats.levels[ATTACK] > 20,
            "attack regenerates: the timer ran"
        );
        assert_eq!(
            player.stats.levels[ANIMALLORE], 20,
            "Summoning points stay drained"
        );
        assert!(engine.script_errors.take().is_empty());
    });
}
