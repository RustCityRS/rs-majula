//! Zone updates once a player's zones have settled.
//!
//! 647's zone headers (UPDATE_ZONE_PARTIAL_FOLLOWS and _ENCLOSED) carry the
//! zone's index in the build area, which the client turns back into tiles
//! with `<< 3`. The first pass after a login or a move sends every zone; once
//! the build area is settled, only changed zones are sent, by a second path.
//! A tile offset there instead of the index put every change outside the
//! area: a felled tree stayed standing and a lit fire never showed until a
//! relog sent the zone again.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

/// The backpack (`inventory:inv`), whose op 8 is Drop.
const BACKPACK: i32 = 149 << 16;
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

        let logs = *engine.cache.objs.debugnames.get("logs").unwrap();
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, logs, 1);
        engine.cycle();
        client.packets();

        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(logs as i32).p2_alt1(0);
        client.send(ClientProt::IfButton8, &p.finish());
        let mut packets = Vec::new();
        for _ in 0..2 {
            engine.cycle();
            packets.extend(client.packets());
        }

        // The zone's x is a plain byte in both headers -- FOLLOWS: `p1_alt2
        // level; p1 x; p1_alt3 z`, ENCLOSED: `p1 x; p1_alt2 level; p1 z` --
        // and a tile offset in place of the index shows in x as well as z.
        let xs: Vec<u8> = packets
            .iter()
            .filter_map(|(op, p)| match *op {
                op if op == ServerProt::UpdateZonePartialFollows as u16 => Some(p[1]),
                op if op == ServerProt::UpdateZonePartialEnclosed as u16 => Some(p[0]),
                _ => None,
            })
            .collect();
        assert!(!xs.is_empty(), "the drop sent no zone update");
        for x in xs {
            assert!(x < ZONES, "zone header at x {x}, outside the build area");
        }
    });
}
