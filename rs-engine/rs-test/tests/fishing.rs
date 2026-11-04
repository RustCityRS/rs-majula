//! Fishing from 289, at a 2011 spot.
//!
//! 289's spots are 2005's; the small-net spot on Lumbridge's rebuilt shore
//! (npc 4908) is 2011's, given 289's `saltfish` category because its ops and
//! its catch (void-634) are a saltfish spot's. The case clicks Net on it as
//! the client does (OPNPC1, `Statics1.do_action`: `p1_alt2(ctrl) p2(nid)`)
//! with a small fishing net in the backpack, and fishes until a shrimp comes
//! up. The catch's numbers come from 289's `fishing_struct_shrimps`, a
//! server-only struct, which only loads since structs past the cache's range
//! are appended at startup.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const FISHING: usize = 10;
const SPOT: u16 = 4908;

#[test]
fn a_2011_spot_gives_shrimps() {
    runner::on_loaded_engine(|engine| {
        let net = *engine
            .cache
            .objs
            .debugnames
            .get("small_fishing_net")
            .unwrap();
        let shrimps = *engine.cache.objs.debugnames.get("raw_shrimps").unwrap();
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

        engine.harness_reset();
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .filter(|n| n.npc.uid.id() == SPOT)
            .map(|n| (n.npc.uid.nid(), n.npc.pathing.coord))
            .map(|(nid, c)| (nid, c.x(), c.z()))
            .next()
            .expect("Lumbridge's small-net spot is spawned");
        let mut client = engine.harness_login_returning("fisher").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, x, 0, z + 2);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, net, 1);
        let xp = player.stats.xp[FISHING];
        engine.cycle();
        client.packets();

        let mut p = ClientPacket::new();
        p.p1_alt2(0).p2(nid as i32);
        client.send(ClientProt::OpNpc1, &p.finish());
        let mut said = Vec::new();
        for _ in 0..150 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            let player = engine.harness_player(client.pid).unwrap();
            if player.invs.get(&inv).unwrap().total(shrimps) > 0 {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.invs.get(&inv).unwrap().total(shrimps),
            1,
            "one raw shrimp; the game said {said:?}"
        );
        assert_eq!(player.stats.xp[FISHING] - xp, 100, "shrimps' productexp");
        assert_eq!(
            said.first().map(String::as_str),
            Some("You cast out your net...")
        );
        assert_eq!(
            said.last().map(String::as_str),
            Some("You catch some shrimps.")
        );
    });
}
