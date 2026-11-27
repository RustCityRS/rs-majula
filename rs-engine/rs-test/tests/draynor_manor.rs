//! Draynor Manor at 647 (`areas/area_draynor`): the garden's vines.

#![cfg(since_530)]

use rs_engine::harness::HarnessClient;
use rs_test::life;
use rs_test::runner;
use rs_vm::ops::test::{calls_for, set_recording};

/// 289's vines whip a player in the manor's garden (zone 0_48_52_40_8, from
/// 3112,3336) every five ticks; 2011's hit for 10 or 20 life points (2011
/// wiki, Draynor Manor: "always hit for 10 or 20 lifepoints").
#[test]
fn the_manor_garden_s_vines_hit_for_10_or_20() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client: HarnessClient = engine
            .harness_login_returning("manor_walker")
            .expect("login");
        engine.cycle();
        // Level 99 Constitution and 990 life points, so the vines do not kill.
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.stats.levels[3] = 99;
        player.stats.base_levels[3] = 99;
        life::set(player, 99 * life::SCALE);
        engine.cycle();
        let walkable = (3112..3120)
            .flat_map(|x| (3336..3344).map(move |z| (x, z)))
            .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
            .expect("a walkable tile in the garden zone");
        set_recording(true);
        engine.harness_telejump(client.pid, walkable.0, 0, walkable.1);
        let mut hits = Vec::new();
        let mut last = life::get(engine, client.pid);
        for _ in 0..40 {
            engine.cycle();
            client.packets();
            let now = life::get(engine, client.pid);
            if now < last {
                hits.push(last - now);
            }
            last = now;
        }
        let said = calls_for("mes");
        assert!(
            said.iter()
                .any(|m| m == "A vine whips out and attacks you!"),
            "{said:?}"
        );
        assert!(!hits.is_empty(), "the vines hit");
        assert!(
            hits.iter()
                .all(|h| *h == life::SCALE || *h == 2 * life::SCALE),
            "each hit is 10 or 20 life points (1 or 2 hitpoints before 647): {hits:?}"
        );
    });
}
