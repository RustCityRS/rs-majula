//! A 2011 arrow leaves the bow, not the sky above it.
//!
//! 289's ranged attack launches its projectile at height 40 and lands it at
//! 36 (`player_ranged.rs2`, `~npc_projectile`). 647's client scales
//! MAP_PROJANIM's heights itself (`AClass17`: `g1() * 4`, then `<< 2` on its
//! 512-unit tiles), so the zone message must carry 40 and 36. The VM's
//! projectile ops scaled them by 4 first, for the pre-647 client, and every
//! 647 arrow started and landed four times too high. The case wields a
//! shortbow and bronze arrows as the client does (`IF_BUTTON2`), clicks
//! Attack on a field cow (`OPNPC2`) and reads the projectile out of the zone
//! update the player is sent.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server::zone_message::zone_prot;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const COW: u16 = 12362;
/// `inventory:inv`.
const BACKPACK: i32 = 149 << 16;

/// `ZoneProt.java`: each rev-647 zone sub-prot's payload size (-1 varies).
const ZONE_PROT_SIZES: [i32; 15] = [-1, 3, 8, 4, 7, 3, 7, 7, 2, -1, 8, 5, 19, 4, 16];

/// The (src, dst) heights of each MAP_PROJANIM in an enclosed zone update:
/// `p1 x, p1_alt1 level, p1 z`, then `[id][payload]` messages.
fn projanim_heights(payload: &[u8]) -> Vec<(u8, u8)> {
    let mut out = Vec::new();
    let mut pos = 3;
    while pos < payload.len() {
        let id = payload[pos] as usize;
        let size = ZONE_PROT_SIZES[id];
        assert!(
            size >= 0,
            "a variable-size zone message ({id}) in the test's update"
        );
        let body = &payload[pos + 1..pos + 1 + size as usize];
        if id == zone_prot::MAP_PROJANIM as usize {
            // coord, dx, dz, target (2), spotanim (2), then the heights.
            out.push((body[7], body[8]));
        }
        pos += 1 + size as usize;
    }
    out
}

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine.cache.objs.debugnames.get(name).unwrap()
}

#[test]
fn an_arrow_flies_at_289s_heights() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("archer").expect("login");
        // 289's combat waits out `%lastcombat`'s window on a new world.
        for _ in 0..20 {
            engine.cycle();
        }
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .map(|n| (n.npc.uid.nid(), n.npc.uid.id(), n.npc.pathing.coord))
            .find(|(_, id, c)| {
                *id == COW && (3254..3262).contains(&c.x()) && (3256..3297).contains(&c.z())
            })
            .map(|(nid, _, c)| (nid, c.x(), c.z()))
            .expect("a cow in the Lumbridge field");
        engine.harness_telejump(client.pid, x + 4, 0, z);
        let (bow, arrows) = (obj(engine, "shortbow"), obj(engine, "bronze_arrow"));
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let backpack = engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&inv)
            .unwrap();
        backpack.set(0, bow, 1);
        backpack.set(1, arrows, 50);
        engine.cycle();
        for (slot, item) in [(0, bow), (1, arrows)] {
            let mut p = ClientPacket::new();
            p.p4(BACKPACK).ip2_alt1(item as i32).ip2(slot);
            client.send(ClientProt::IfButton2, &p.finish());
            engine.cycle();
        }
        client.packets();

        let mut p = ClientPacket::new();
        p.p1_alt1(0).ip2_alt1(nid as i32);
        client.send(ClientProt::OpNpc2, &p.finish());
        let enclosed = ServerProt::UpdateZonePartialEnclosed as u16;
        let mut heights = Vec::new();
        for _ in 0..20 {
            engine.cycle();
            for (op, payload) in client.packets() {
                if op == enclosed {
                    heights.extend(projanim_heights(&payload));
                }
            }
            if !heights.is_empty() {
                break;
            }
        }
        assert_eq!(heights.first(), Some(&(40, 36)), "the arrow's heights");
    });
}
