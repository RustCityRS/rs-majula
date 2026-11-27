#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
#[cfg(before_742)]
use rs_entity::MoveSpeed;
#[cfg(before_742)]
use rs_grid::CoordGrid;
use rs_pack::cache::VarValue;
use rs_test::runner;
#[cfg(before_742)]
use rs_test::server;
use rs_ver::TypeId;
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::fs;

fn click(client: &mut HarnessClient, x: u16, z: u16) {
    client.move_gameclick(x, z);
}

fn route(engine: &Engine, pid: u16) -> Vec<u32> {
    let pathing = &engine.harness_player(pid).unwrap().pathing;
    (0..=pathing.waypoints.index)
        .rev()
        .map(|i| pathing.waypoints[i as usize].packed())
        .collect()
}

fn expected(x: u16, z: u16) -> Vec<u32> {
    rsmod::find_route(
        0,
        3222,
        3218,
        x,
        z,
        1,
        1,
        1,
        0,
        -1,
        true,
        0,
        25,
        CollisionType::Normal,
        true,
    )
    .to_vec()
}

#[cfg(before_742)]
#[test]
fn reads_the_lumbridge_signpost_from_six_tiles_away() {
    runner::on_loaded_engine(|engine| {
        let sign = if cfg!(since_647) { 2754 } else { 18493 };
        for (name, x, z, running) in [
            ("reader", 3235, 3222, false),
            ("northreader", 3236, 3230, false),
            ("runreader", 3244, 3225, true),
        ] {
            let mut client = engine.harness_login_returning_at(name, x, z).unwrap();
            for _ in 0..4 {
                engine.cycle();
            }
            if running {
                engine.harness_player_mut(client.pid).unwrap().move_speed = MoveSpeed::Run;
            }
            client.packets();
            client.op_loc(1, 3235, 3228, TypeId::from_int(sign));
            engine.cycle();

            let player = engine.harness_player(client.pid).unwrap();
            let coord = player.pathing.coord;
            assert_eq!((coord.x(), coord.z()) != (x, z), running);
            assert_eq!(
                player.modal_main,
                Some(rs_test::names::iface("aide_compass"))
            );
            let local_x = (coord.x() - player.build_area.origin.zone_origin_x()) as u8;
            let local_z = (coord.z() - player.build_area.origin.zone_origin_z()) as u8;
            let packets = client.packets();
            assert_eq!(server::cam_forceangles(&packets), [(280, 0)]);
            assert_eq!(
                server::cam_movetos(&packets),
                [(local_x, local_z - 6, 1500, 2, 10)]
            );
            assert_eq!(
                server::cam_lookats(&packets),
                [(local_x, local_z + 3, 450, 2, 10)]
            );
            let north = Some((
                CoordGrid::fine(coord.x(), 1),
                CoordGrid::fine(coord.z() + 1, 1),
            ));
            if running {
                assert_ne!(
                    player.info.orientation_x.zip(player.info.orientation_z),
                    north
                );
                engine.cycle();
            }
            let player = engine.harness_player(client.pid).unwrap();
            assert_eq!(player.pathing.coord, coord);
            assert_eq!(
                player.info.orientation_x.zip(player.info.orientation_z),
                north
            );
        }
    });
}

#[test]
fn last_input_route_wins_without_skipping_walk_triggers() {
    runner::on_loaded_engine(|engine| {
        let mut clients: Vec<_> = (0..24)
            .map(|i| engine.harness_login_returning(&format!("path{i}")).unwrap())
            .collect();
        for _ in 0..4 {
            engine.cycle();
        }
        for client in &mut clients {
            engine.harness_telejump(client.pid, 3222, 0, 3218);
            client.packets();
        }
        engine.harness_inputs();

        for threads in [1, 2, 4] {
            engine.set_pathfinder_threads(threads).unwrap();
            for client in &mut clients {
                for x in 3223..3228 {
                    click(client, x, 3218);
                }
            }
            engine.harness_inputs();
            assert_eq!(engine.pathfinding_stats().requests, 24 * 5);
            assert_eq!(engine.pathfinding_stats().searches, 24);
            let expected = expected(3227, 3218);
            assert!(!expected.is_empty());
            for client in &clients {
                assert_eq!(route(engine, client.pid), expected, "threads={threads}");
            }
        }

        for x in 3223..3229 {
            click(&mut clients[0], x, 3218);
        }
        engine.harness_inputs();
        assert_eq!(engine.pathfinding_stats().searches, 1);
        assert_eq!(route(engine, clients[0].pid), expected(3227, 3218));
        engine.harness_inputs();
        assert_eq!(engine.pathfinding_stats().searches, 1);
        assert_eq!(route(engine, clients[0].pid), expected(3228, 3218));

        click(&mut clients[0], 3227, 3218);
        click(&mut clients[0], 3500, 3218);
        engine.harness_inputs();
        assert_eq!(engine.pathfinding_stats().searches, 0);
        assert!(route(engine, clients[0].pid).is_empty());

        let last = clients.len() - 1;
        let trigger = engine
            .scripts
            .get_by_name("[walktrigger,stunned]")
            .unwrap()
            .id;
        let stunned = engine.cache.varps.get_by_debugname("stunned").unwrap().id;
        let until = engine.clock as i32 + 20;
        let player = engine.harness_player_mut(clients[last].pid).unwrap();
        player.vars.set(stunned.g2(), VarValue::Int(until));
        player.walktrigger = Some(trigger);
        for client in &mut clients {
            for x in 3223..3228 {
                click(client, x, 3218);
            }
        }
        engine.harness_inputs();
        assert_eq!(engine.pathfinding_stats().requests, 24 * 5 + 5);
        assert_eq!(engine.pathfinding_stats().searches, 24);
        assert!(engine.pathfinding_stats().batches >= 2);
        assert!(route(engine, clients[last].pid).is_empty());
        engine.cycle();
        let messages = HarnessClient::game_messages(&clients[last].packets());
        assert_eq!(
            messages
                .iter()
                .filter(|(_, s)| s == "You're stunned!")
                .count(),
            5
        );
        let coord = engine
            .harness_player(clients[last].pid)
            .unwrap()
            .pathing
            .coord;
        assert_eq!((coord.x(), coord.z()), (3222, 3218));

        let door = fs::read_to_string(format!("{}/pack/loc.pack", rs_pack::CONTENT_DIR))
            .unwrap()
            .lines()
            .find_map(|line| {
                line.split_once('=')
                    .filter(|(_, name)| *name == "poordoor")
                    .map(|(id, _)| TypeId::new(id.parse().unwrap()))
            })
            .unwrap();
        engine
            .harness_player_mut(clients[0].pid)
            .unwrap()
            .pathing
            .clear_waypoints();
        engine.harness_telejump(clients[0].pid, 2551, 0, 3099);
        engine.cycle();
        click(&mut clients[0], 2552, 3099);
        clients[0].op_loc(1, 2551, 3098, door);
        engine.harness_inputs();
        assert_eq!(engine.pathfinding_stats().requests, 2);
        assert_eq!(engine.pathfinding_stats().searches, 1);
        for _ in 0..6 {
            engine.cycle();
        }
        assert!(
            engine
                .zones
                .zone(2551, 0, 3098)
                .unwrap()
                .get_loc(2551, 3098, door)
                .is_none()
        );
    });
}
