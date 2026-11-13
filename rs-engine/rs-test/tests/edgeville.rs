//! Edgeville at 647 (`areas/area_edgeville`): 289's brass key door on the house that leads down to
//! the dungeon (loc 1804, 3115,3450, on the tile's south edge), locked to Open and opened with the
//! brass key into its opened stage.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const BACKPACK: i32 = 149 << 16;
const DOOR: (u16, u16, u16) = (1804, 3115, 3450);

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn give_key(engine: &mut Engine, client: &HarnessClient) {
    let key = obj(engine, "edgevilledungeonkey");
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(0, key, 1);
}

fn at(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

fn messages(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..ticks {
        engine.cycle();
        out.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    out
}

fn stand(engine: &mut Engine, client: &mut HarnessClient, x: u16, z: u16) {
    engine.harness_telejump(client.pid, x, 0, z);
    messages(engine, client, 3);
}

#[test]
fn the_brass_key_door_is_locked_and_the_key_opens_it_from_either_side() {
    runner::on_loaded_engine(|engine| {
        let _ = engine.script_errors.take();
        engine.harness_reset();
        let mut client = engine.harness_login_returning("edgeville").expect("login");
        messages(engine, &mut client, 10);

        // Open, without the key.
        stand(engine, &mut client, 3115, 3449);
        let (id, x, z) = DOOR;
        let mut p = ClientPacket::new();
        p.p2_alt1(z as i32)
            .p2_alt1(id as i32)
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());
        let said = messages(engine, &mut client, 6);
        assert!(said.iter().any(|m| m == "The door is locked."), "{said:?}");

        // The key, from each side: through to the other.
        give_key(engine, &client);
        let key = obj(engine, "edgevilledungeonkey");
        for (from, to) in [((3115, 3449), (3115, 3450)), ((3115, 3450), (3115, 3449))] {
            stand(engine, &mut client, from.0, from.1);
            let mut p = ClientPacket::new();
            p.p2_alt2(key as i32)
                .p2_alt2(0)
                .p2(z as i32)
                .p4_alt2(BACKPACK)
                .p2_alt2(x as i32)
                .p1_alt1(0)
                .p2_alt1(id as i32);
            client.send(ClientProt::OpLocT, &p.finish());
            let said = messages(engine, &mut client, 8);
            assert!(said.iter().any(|m| m == "You unlock the door."), "{said:?}");
            assert_eq!(at(engine, &client), to, "through from {from:?}: {said:?}");
            // The opened door shuts again after 3 ticks (loc_add's duration).
            messages(engine, &mut client, 6);
        }
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}
