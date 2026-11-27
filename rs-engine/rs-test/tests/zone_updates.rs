//! Zone updates once a player's zones have settled.
//!
//! 647's zone headers (UPDATE_ZONE_PARTIAL_FOLLOWS and _ENCLOSED) carry the
//! zone's index in the build area, which the client turns back into tiles
//! with `<< 3`. The first pass after a login or a move sends every zone; once
//! the build area is settled, only changed zones are sent, by a second path.
//! A tile offset there instead of the index put every change outside the
//! area: a felled tree stayed standing and a lit fire never showed until a
//! relog sent the zone again.

#![cfg(since_530)]

use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_test::server;

/// A build area is 13 zones a side.
const ZONES: u8 = 13;

#[test]
fn a_drop_after_the_zones_settle_is_sent_in_its_own_zone() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("dropper").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, 3222, 0, 3218);
        // Long enough for the full pass to send every zone and settle.
        for _ in 0..4 {
            engine.cycle();
        }
        client.packets();

        let logs = engine.cache.objs.get_by_debugname("logs").unwrap().id;
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, logs, 1);
        engine.cycle();
        client.packets();

        client.if_button(8, BACKPACK, 0, logs);
        let mut packets = Vec::new();
        for _ in 0..2 {
            engine.cycle();
            packets.extend(client.packets());
        }

        // A tile offset in place of the index shows in x as well as z.
        let xs: Vec<u8> = server::zone_partials(&packets)
            .into_iter()
            .map(|(x, _)| x)
            .collect();
        assert!(!xs.is_empty(), "the drop sent no zone update");
        for x in xs {
            assert!(x < ZONES, "zone header at x {x}, outside the build area");
        }
    });
}
