//! 289's sheep shearing (`general_use/scripts/shear_sheep.rs2`) on 2011's
//! sheep, which have a Shear op (op1, cache 786): it shears with shears in the
//! pack (2011 wiki, Wool) and says void-634's line without them.
//! `OPNPC1`: `p1_alt2(ctrl) p2(npc index)`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const UNSHORN: u16 = 43;

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

/// The unshorn sheep nearest Lumbridge's pen (3205,3262): its index and tile.
fn sheep(engine: &Engine) -> (u16, u16, u16) {
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.base_type == UNSHORN)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.z())
        })
        .min_by_key(|&(_, x, z)| (x as i32 - 3205).abs() + (z as i32 - 3262).abs())
        .expect("an unshorn sheep by Lumbridge")
}

fn shear(client: &mut HarnessClient, nid: u16) {
    let mut p = ClientPacket::new();
    p.p1_alt2(0).p2(nid as i32);
    client.send(ClientProt::OpNpc1, &p.finish());
}

#[test]
fn shear_takes_wool_with_shears_and_asks_for_them_without() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("shepherd").expect("login");
        engine.cycle();
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let (wool, shears) = (obj(engine, "wool"), obj(engine, "shears"));

        let (nid, x, z) = sheep(engine);
        engine.harness_telejump(client.pid, x, 0, z + 1);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        shear(&mut client, nid);
        let mut said = Vec::new();
        for _ in 0..15 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, m)| m),
            );
        }
        assert!(
            said.iter()
                .any(|m| m == "You need a set of shears to do this."),
            "without shears: {said:?}"
        );

        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, shears, 1);
        // 289's sheep get away one time in four: try again until one is shorn.
        let mut shorn = false;
        for _ in 0..12 {
            let (nid, x, z) = sheep(engine);
            engine.harness_telejump(client.pid, x, 0, z + 1);
            for _ in 0..3 {
                engine.cycle();
            }
            client.packets();
            shear(&mut client, nid);
            for _ in 0..15 {
                engine.cycle();
            }
            if engine
                .harness_player(client.pid)
                .unwrap()
                .invs
                .get(&inv)
                .unwrap()
                .total(wool)
                > 0
            {
                shorn = true;
                break;
            }
        }
        assert!(shorn, "Shear with shears gave wool");
    });
}
