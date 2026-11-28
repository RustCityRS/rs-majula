//! Summoning (`skill_animallore`) at 2011, the parts that are not Wolf Whistle's own: the
//! charged obelisk's two ops as a player meets them outside the quest's obelisk stages.
//!
//! Messages are the dated recordings' (`target/refs/wolf_whistle/.claude/647-wolf-whistle-states.md`, section 5).

#![cfg(since_647)]

use std::sync::Mutex;

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_test::names::iface;
use rs_test::runner;

static TURN: Mutex<()> = Mutex::new(());

const ANIMALLORE: usize = 23;
/// The Taverley cellar's obelisk (loc 28716; cache 786's map l34_83) and a tile south of it.
const OBELISK: (u16, u16) = (2209, 5344);
const CELLAR: (u16, u16) = (2209, 5347);
/// `wolf_quest` is bits 1-10 of varp 1178; the cache completes the quest at 110.
const WOLF_COMPLETE: i32 = 110;

fn at_the_obelisk(engine: &mut Engine, name: &str, wolf_quest: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    for _ in 0..10 {
        engine.cycle();
    }
    let varp = engine.cache.varps.get_by_debugname("wolf_var").unwrap().id;
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(varp.g2(), VarValue::Int(wolf_quest << 1));
    engine.harness_telejump(client.pid, CELLAR.0, 0, CELLAR.1);
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    client
}

fn op(engine: &mut Engine, client: &mut HarnessClient, op: u8) -> Vec<String> {
    let obelisk = engine
        .cache
        .locs
        .get_by_debugname("wolf_obelisk_druid01")
        .unwrap()
        .id;
    client.op_loc(op, OBELISK.0, OBELISK.1, obelisk);
    let mut out = Vec::new();
    for _ in 0..6 {
        engine.cycle();
        out.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    out
}

#[test]
fn renew_points_restores_summoning_points_to_the_level() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = at_the_obelisk(engine, "lore_renewer", WOLF_COMPLETE);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.stats.base_levels[ANIMALLORE] = 50;
        player.stats.levels[ANIMALLORE] = 20;

        // 2011's: 2EfU5Mrh8Kc 2011-05-24 @310, JQZnlZk0KqE 2011-05-24 @468.
        let said = op(engine, &mut client, 2);
        assert!(
            said.iter()
                .any(|m| m == "You have recharged your Summoning skill points."),
            "{said:?}"
        );
        assert!(
            !said
                .iter()
                .any(|m| m == "You should speak to Pikkupstix to get your reward."),
            "the quest's line is only the quest's: {said:?}"
        );
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.levels[ANIMALLORE],
            50
        );

        // 2011's: HemCMqn3Y3E 2009-11-20 @168, ldSYrbNZbI8 2010-07-31 @327.
        let said = op(engine, &mut client, 2);
        assert!(
            said.iter()
                .any(|m| m == "You already have full Summoning points."),
            "{said:?}"
        );
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}

#[test]
fn the_obelisk_needs_summoning_knowledge_before_the_quest() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = at_the_obelisk(engine, "lore_stranger", 0);
        // 2011's: 2EfU5Mrh8Kc 2011-05-24 @306, ldSYrbNZbI8 2010-07-31 @323.
        let said = op(engine, &mut client, 1);
        assert!(
            said.iter().any(|m| m
                == "You do not have the required Summoning knowledge to use this obelisk yet."),
            "{said:?}"
        );
        assert!(
            !engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(iface("interface_672")),
            "no creation window"
        );
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}
