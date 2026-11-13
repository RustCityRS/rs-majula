#![cfg(since_647)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_vm::engine::{ScriptPlayer, engine_typed_mut, with_engine};
use rs_vm::subject::ScriptSubject;

fn wait_for_map(engine: &mut Engine, client: &HarnessClient) {
    let uid = engine.player_list.players[client.pid as usize]
        .as_ref()
        .unwrap()
        .player
        .uid;
    engine
        .run_script_by_name(
            "[proc,p_loaddelay]",
            Some(ScriptSubject::Player(uid)),
            None,
            Some(true),
            None,
            None,
        )
        .unwrap();
}

fn waiting(engine: &Engine, client: &HarnessClient) -> bool {
    engine.player_list.players[client.pid as usize]
        .as_ref()
        .unwrap()
        .player
        .state
        .active_script
        .is_some()
}

fn tick(engine: &mut Engine, client: &mut HarnessClient) -> Vec<(u16, Vec<u8>)> {
    client.send(ClientProt::NoTimeout, &[]);
    engine.cycle();
    client.packets()
}

fn acknowledge(engine: &mut Engine, client: &mut HarnessClient) {
    client.send(ClientProt::MapBuildComplete, &[]);
    tick(engine, client);
    assert!(!waiting(engine, client));
    assert!(
        engine.player_list.players[client.pid as usize]
            .as_ref()
            .unwrap()
            .map_build_complete()
    );
}

#[test]
fn load_delay_waits_for_the_scene_and_bounds_a_missing_acknowledgement() {
    runner::on_loaded_engine(|engine| {
        with_engine(engine, || {
            let engine = unsafe { engine_typed_mut::<Engine>() };
            let mut client = engine
                .harness_login_returning_at("mapready", 3222, 3218)
                .unwrap();
            tick(engine, &mut client);
            wait_for_map(engine, &client);
            tick(engine, &mut client);
            assert!(waiting(engine, &client));
            acknowledge(engine, &mut client);

            let pid = client.pid as usize;
            engine.player_list.players[pid]
                .as_mut()
                .unwrap()
                .telejump(CoordGrid::new(3223, 0, 3218));
            wait_for_map(engine, &client);
            assert!(!waiting(engine, &client));

            engine.player_list.players[pid]
                .as_mut()
                .unwrap()
                .telejump(CoordGrid::new(3200, 0, 3420));
            wait_for_map(engine, &client);
            assert!(waiting(engine, &client));
            client.send(ClientProt::MapBuildComplete, &[]);
            let packets = tick(engine, &mut client);
            assert!(
                packets
                    .iter()
                    .any(|p| p.0 == ServerProt::RebuildNormal as u16)
            );
            assert!(waiting(engine, &client));
            tick(engine, &mut client);
            assert!(waiting(engine, &client));
            acknowledge(engine, &mut client);

            let region = engine.harness_region_create(1, 4, 4).unwrap();
            let source = CoordGrid::new(3200, 0, 3200);
            engine
                .harness_region_set(region, 0, 1, 1, source, 0)
                .unwrap();
            let destination = engine.harness_region_coord(region, 0, 10, 10).unwrap();
            engine.player_list.players[pid]
                .as_mut()
                .unwrap()
                .telejump(destination);
            wait_for_map(engine, &client);
            let packets = tick(engine, &mut client);
            assert!(
                packets
                    .iter()
                    .any(|p| p.0 == ServerProt::RebuildRegion as u16)
            );
            assert!(waiting(engine, &client));
            acknowledge(engine, &mut client);

            engine
                .harness_region_set(region, 0, 1, 1, source, 1)
                .unwrap();
            wait_for_map(engine, &client);
            let packets = tick(engine, &mut client);
            assert!(
                packets
                    .iter()
                    .any(|p| p.0 == ServerProt::RebuildRegion as u16)
            );
            assert!(waiting(engine, &client));
            acknowledge(engine, &mut client);

            engine.player_list.players[pid]
                .as_mut()
                .unwrap()
                .rebuild_normal(true);
            wait_for_map(engine, &client);
            tick(engine, &mut client);
            assert!(waiting(engine, &client));
            acknowledge(engine, &mut client);

            engine.player_list.players[pid]
                .as_mut()
                .unwrap()
                .rebuild_normal(true);
            wait_for_map(engine, &client);
            for _ in 0..99 {
                tick(engine, &mut client);
                assert!(waiting(engine, &client));
            }
            for _ in 0..2 {
                tick(engine, &mut client);
            }
            assert!(!waiting(engine, &client));
            assert!(
                !engine.player_list.players[pid]
                    .as_ref()
                    .unwrap()
                    .map_build_complete()
            );
            acknowledge(engine, &mut client);
        });
    });
}
