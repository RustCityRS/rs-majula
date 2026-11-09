//! Regions: static zones copied into region space, built as the 647 client
//! builds them from the same templates, sent with REBUILD_REGION, saved at
//! their exit and torn down.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_entity::player::StaffModLevel;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
use rs_vm::RegionUid;
use rsmod::rsmod::flag::zone_flag::ZoneFlag;

/// Lumbridge castle: a 4x4 zone area of mapsquare 50_50, walls, doors and
/// furniture throughout.
const SOURCE: (u16, u16) = (3200, 3200);
const TILES: u16 = 32;

/// The npc and player blocking flags, which stand for whoever is there.
const ENTITIES: u32 = 0x80000 | 0x100000;

/// Each directional collision flag family, clockwise from north (and from
/// north-east for the corners).
const TURNS: [[u32; 4]; 6] = [
    [0x2, 0x8, 0x20, 0x80],
    [0x4, 0x10, 0x40, 0x1],
    [0x400, 0x1000, 0x4000, 0x10000],
    [0x800, 0x2000, 0x8000, 0x200],
    [0x800000, 0x2000000, 0x8000000, 0x20000000],
    [0x1000000, 0x4000000, 0x10000000, 0x400000],
];

fn flags(x: u16, z: u16, y: u8) -> u32 {
    (0..32)
        .filter(|bit| rsmod::is_flagged(x, z, y, 1 << bit))
        .fold(0, |flags, bit| flags | 1 << bit)
}

/// `flags` turned `rotation` quarter turns clockwise.
fn turn(flags: u32, rotation: usize) -> u32 {
    let mut turned = flags;
    for family in TURNS {
        for bit in family {
            turned &= !bit;
        }
        for (i, bit) in family.iter().enumerate() {
            if flags & bit != 0 {
                turned |= family[(i + rotation) & 3];
            }
        }
    }
    turned
}

/// Where tile (`x`, `z`) of the area lands with the area turned.
fn turned(x: u16, z: u16, rotation: usize) -> (u16, u16) {
    match rotation {
        0 => (x, z),
        1 => (z, TILES - 1 - x),
        2 => (TILES - 1 - x, TILES - 1 - z),
        _ => (TILES - 1 - z, x),
    }
}

/// Where a `width` x `length` loc at `angle` with its south-west tile on
/// (`x`, `z`) has its south-west tile with the area turned.
fn turned_loc(x: u16, z: u16, width: u16, length: u16, angle: u8, rotation: usize) -> (u16, u16) {
    let (width, length) = if angle & 1 == 1 {
        (length, width)
    } else {
        (width, length)
    };
    match rotation {
        0 => (x, z),
        1 => (z, TILES - x - width),
        2 => (TILES - x - width, TILES - z - length),
        _ => (TILES - z - length, x),
    }
}

/// Every active loc on `level` lying wholly in the area from (`x`, `z`), as
/// `(id, x, z, angle, shape, width, length)` relative to (`x`, `z`).
fn locs_on(engine: &Engine, x: u16, z: u16, level: u8) -> Vec<(u16, u16, u16, u8, u8, u16, u16)> {
    let mut locs = Vec::new();
    for zx in (0..TILES + 16).step_by(8) {
        for zz in (0..TILES + 16).step_by(8) {
            let Some(zone) = engine.zones.zone(x + zx - 8, level, z + zz - 8) else {
                continue;
            };
            for loc in &zone.locs {
                let coord = loc.world_coord(zone.coord);
                let (width, length) = (loc.width() as i32, loc.length() as i32);
                let (width, length) = if loc.angle() as u8 & 1 == 1 {
                    (length, width)
                } else {
                    (width, length)
                };
                let (lx, lz) = (coord.x() as i32 - x as i32, coord.z() as i32 - z as i32);
                if lx < 0 || lz < 0 || lx + width > TILES as i32 || lz + length > TILES as i32 {
                    continue;
                }
                locs.push((
                    loc.id(),
                    lx as u16,
                    lz as u16,
                    loc.angle() as u8,
                    loc.shape() as u8,
                    loc.width() as u16,
                    loc.length() as u16,
                ));
            }
        }
    }
    locs.sort();
    locs
}

/// Every active loc on the ground lying wholly in the area from (`x`, `z`).
fn locs(engine: &Engine, x: u16, z: u16) -> Vec<(u16, u16, u16, u8, u8, u16, u16)> {
    locs_on(engine, x, z, 0)
}

/// Where zone (`a`, `b`) of the area lands with the area turned.
fn turned_zone(a: u16, b: u16, rotation: usize) -> (u16, u16) {
    let last = (TILES >> 3) - 1;
    match rotation {
        0 => (a, b),
        1 => (b, last - a),
        2 => (last - a, last - b),
        _ => (last - b, a),
    }
}

/// Sets the zones of the area from `from`, on `levels`, onto region `uid`
/// from its zone (2, 2), the area turned `rotation`, a level at a time.
fn fill(engine: &mut Engine, uid: RegionUid, from: (u16, u16), levels: &[u8], rotation: usize) {
    let zones = TILES >> 3;
    for &level in levels {
        for a in 0..zones {
            for b in 0..zones {
                let (i, j) = turned_zone(a, b, rotation);
                let source = CoordGrid::new(from.0 + (a << 3), level, from.1 + (b << 3));
                engine
                    .harness_region_set(
                        uid,
                        level as i32,
                        2 + i as i32,
                        2 + j as i32,
                        source,
                        rotation as i32,
                    )
                    .unwrap();
            }
        }
    }
}

/// A region with the area's ground copied onto it from tile 16, 16, turned
/// `rotation`.
fn copy(engine: &mut Engine, rotation: usize) -> (RegionUid, CoordGrid) {
    let uid = engine
        .harness_region_create(4, 16, 16)
        .expect("a free region");
    fill(engine, uid, SOURCE, &[0], rotation);
    (uid, engine.harness_region_coord(uid, 0, 16, 16).unwrap())
}

#[test]
fn copied_zones_collide_as_their_source_turned() {
    runner::on_loaded_engine(|engine| {
        let source = locs(engine, SOURCE.0, SOURCE.1);
        assert!(!source.is_empty());
        for rotation in 0..4 {
            let (uid, to) = copy(engine, rotation);

            for x in 8..TILES - 8 {
                for z in 8..TILES - 8 {
                    let expected = turn(flags(SOURCE.0 + x, SOURCE.1 + z, 0) & !ENTITIES, rotation);
                    let (dx, dz) = turned(x, z, rotation);
                    let copied = flags(to.x() + dx, to.z() + dz, 0) & !ENTITIES;
                    assert_eq!(
                        copied, expected,
                        "rotation {rotation}: tile {x},{z} copied to {dx},{dz}"
                    );
                }
            }

            let mut expected: Vec<_> = source
                .iter()
                .map(|&(id, x, z, angle, shape, width, length)| {
                    let (x, z) = turned_loc(x, z, width, length, angle, rotation);
                    (id, x, z, (angle + rotation as u8) & 3, shape, width, length)
                })
                .collect();
            expected.sort();
            assert_eq!(
                locs(engine, to.x(), to.z()),
                expected,
                "rotation {rotation}"
            );

            engine.harness_region_free(uid);
            engine.cycle();
            assert!(!rsmod::is_zone_allocated(to.x(), to.z(), 0));
        }
    });
}

#[test]
fn copied_zones_keep_their_sources_ground_and_free_mark() {
    runner::on_loaded_engine(|engine| {
        let uid = engine
            .harness_region_create(4, 16, 16)
            .expect("a free region");
        let to = engine.harness_region_coord(uid, 0, 16, 16).unwrap();
        fill(engine, uid, SOURCE, &[0, 1, 2, 3], 0);
        let free = |x, z, y| rsmod::is_zone_flagged(x, z, y, ZoneFlag::Free as u8);
        let mut void = 0;
        for level in 0..4 {
            for x in (0..TILES).step_by(8) {
                for z in (0..TILES).step_by(8) {
                    let ground = rsmod::is_zone_allocated(SOURCE.0 + x, SOURCE.1 + z, level);
                    void += usize::from(!ground);
                    assert_eq!(
                        rsmod::is_zone_allocated(to.x() + x, to.z() + z, level),
                        ground,
                        "level {level} zone {x},{z}"
                    );
                    assert_eq!(
                        free(to.x() + x, to.z() + z, level),
                        free(SOURCE.0 + x, SOURCE.1 + z, level),
                        "level {level} zone {x},{z}"
                    );
                }
            }
        }
        assert!(void > 0, "the castle has void upstairs");
        assert!(free(to.x(), to.z(), 0), "Lumbridge is free");
        engine.harness_region_free(uid);
        assert!(!free(to.x(), to.z(), 0));
    });
}

/// The templates REBUILD_REGION sends after its 7-byte header: a bit, then a
/// 26-bit template if it's set, for each of 4x13x13 zones; and the bytes they
/// take.
fn templates(payload: &[u8]) -> (Vec<Option<u32>>, usize) {
    let mut bit = 0;
    let mut read = |count: usize| {
        let mut value = 0;
        for _ in 0..count {
            let byte = payload[7 + (bit >> 3)];
            value = (value << 1) | ((byte >> (7 - (bit & 7))) & 1) as u32;
            bit += 1;
        }
        value
    };
    let templates = (0..(13 * 13) << 2)
        .map(|_| (read(1) == 1).then(|| read(26)))
        .collect();
    (templates, (bit + 7) >> 3)
}

#[test]
fn players_get_a_region_map_and_leave_with_the_region() {
    runner::on_loaded_engine(|engine| {
        let mut client = engine
            .harness_login_returning_at("regions", 3222, 3218)
            .unwrap();
        engine.cycle();
        client.packets();

        let (uid, _) = copy(engine, 1);
        let inside = engine.harness_region_coord(uid, 0, 24, 24).unwrap();
        engine.harness_telejump(client.pid, inside.x(), 0, inside.z());
        engine.cycle();

        let packets = client.packets();
        let rebuild = packets
            .iter()
            .find(|(prot, _)| *prot == ServerProt::RebuildRegion as u16)
            .map(|(_, payload)| payload)
            .expect("REBUILD_REGION");
        let (templates, bytes) = templates(rebuild);
        assert_eq!(
            rebuild.len() - 7 - bytes,
            16,
            "one key, for mapsquare 50_50"
        );
        // The player's zone, area zone (1, 1) turned once, came from area
        // zone (2, 1): zone 402, 401 of level 0, turned once.
        assert_eq!(
            templates[6 * 13 + 6],
            Some((402 << 14) | (401 << 3) | (1 << 1))
        );
        assert_eq!(templates.iter().flatten().count(), 16);

        // The save keeps where the player stands, and the exit beside it.
        let came = CoordGrid::new(3222, 0, 3218);
        assert_eq!(
            engine.harness_saved_coord(client.pid),
            Some((inside, Some(came), came)),
            "no exit coord: the tile it came from"
        );
        assert_eq!(
            engine.harness_saved_exits(client.pid),
            Some((Some(came), Some(came))),
            "the save row and the .sav file keep it"
        );
        assert!(
            engine.harness_region_set_exit(uid, inside).is_err(),
            "no exit in region space"
        );
        let exit = CoordGrid::new(3222, 0, 3222);
        engine.harness_region_set_exit(uid, exit).unwrap();
        assert_eq!(
            engine.harness_saved_coord(client.pid),
            Some((inside, Some(exit), exit)),
            "the region's exit coord"
        );
        assert_eq!(
            engine.harness_saved_exits(client.pid),
            Some((Some(exit), Some(exit)))
        );
        let manager = engine.harness_region_coord(uid, 0, 0, 0).unwrap();
        assert!(engine.harness_controller_add(manager, 0));

        engine.harness_region_free(uid);
        let coord = engine.harness_player(client.pid).unwrap().pathing.coord;
        assert_eq!(coord, exit, "a freed region's players go to its exit");
        assert!(
            !rsmod::is_zone_allocated(inside.x(), inside.z(), 0),
            "nothing reaches a freed region later in its tick"
        );
        assert!(!engine.harness_controller_add(manager, 0));
        engine.cycle();
        assert!(
            client
                .packets()
                .iter()
                .any(|(prot, _)| *prot == ServerProt::RebuildNormal as u16)
        );
    });
}

/// A save in region space logs in at the exit it saved, placed before any
/// packet: never on the region tile, as its region is gone, and home when
/// there is no exit a player can stand on.
#[test]
fn a_save_in_region_space_logs_in_at_its_exit() {
    runner::on_loaded_engine(|engine| {
        let tile = CoordGrid::new(6500, 0, 100);
        for (name, exit, expected) in [
            ("exited", Some(CoordGrid::new(3222, 0, 3222)), (3222, 3222)),
            ("lost", None, (3221, 3218)),
            ("voided", Some(CoordGrid::new(6600, 0, 100)), (3221, 3218)),
        ] {
            let client = engine.harness_login_saved_at(name, tile, exit).unwrap();
            let coord = engine.harness_player(client.pid).unwrap().pathing.coord;
            assert_eq!((coord.x(), coord.z()), expected, "{name}");
        }
    });
}

/// Types `input` into the client's command box.
fn cheat(client: &mut HarnessClient, input: &str) {
    let mut p = ClientPacket::new();
    p.p1(0).p1(0).pjstr(input);
    client.send(ClientProt::ClientCheat, &p.finish());
}

/// `::instance` sets no exit coord, so leaving it and a save in it both go
/// back to the tile the player entered it from, not the map tile under where
/// it stands in the copy.
#[test]
fn leaving_an_instance_with_no_exit_goes_back_where_it_was_entered() {
    runner::on_loaded_engine(|engine| {
        let mut client = engine
            .harness_login_returning_at("leaver", 3222, 3218)
            .unwrap();
        engine
            .harness_player_mut(client.pid)
            .unwrap()
            .staff_mod_level = StaffModLevel::Maintainer;
        engine.cycle();
        let entered = engine.harness_player(client.pid).unwrap().pathing.coord;

        cheat(&mut client, "instance 0");
        engine.cycle();
        let inside = engine.harness_player(client.pid).unwrap().pathing.coord;
        assert!(inside.x() >= 6400, "in the instance at {inside:?}");
        engine.harness_telejump(client.pid, inside.x() + 3, 0, inside.z() + 2);
        engine.cycle();
        let (_, exit, login) = engine.harness_saved_coord(client.pid).unwrap();
        assert_eq!((exit, login), (Some(entered), entered));

        cheat(&mut client, "instanceleave");
        engine.cycle();
        assert_eq!(
            engine.harness_player(client.pid).unwrap().pathing.coord,
            entered
        );
        assert!(engine.script_errors.take().is_empty());
    });
}

/// Every collision flag of the area from `to` and a zone around it, on
/// every level.
fn area_flags(to: CoordGrid) -> Vec<u32> {
    let mut all = Vec::new();
    for level in 0..4 {
        for x in 0..TILES + 16 {
            for z in 0..TILES + 16 {
                all.push(flags(to.x() + x - 8, to.z() + z - 8, level));
            }
        }
    }
    all
}

#[test]
fn zones_set_over_others_build_what_fresh_ones_do() {
    runner::on_loaded_engine(|engine| {
        let zones = TILES >> 3;
        for rotation in 0..4 {
            let (fresh, to) = copy(engine, rotation);
            let expected = (area_flags(to), locs(engine, to.x(), to.z()));

            // Over a copy turned another way and in reverse, so each zone is
            // replaced beside zones still to come.
            let (over, to) = copy(engine, (rotation + 1) & 3);
            for a in (0..zones).rev() {
                for b in (0..zones).rev() {
                    let (i, j) = turned_zone(a, b, rotation);
                    let source = CoordGrid::new(SOURCE.0 + (a << 3), 0, SOURCE.1 + (b << 3));
                    engine
                        .harness_region_set(
                            over,
                            0,
                            2 + i as i32,
                            2 + j as i32,
                            source,
                            rotation as i32,
                        )
                        .unwrap();
                }
            }
            assert!(
                (area_flags(to), locs(engine, to.x(), to.z())) == expected,
                "rotation {rotation}"
            );

            engine.harness_region_free(fresh);
            engine.harness_region_free(over);
            engine.cycle();
        }
    });
}

/// Lumbridge's bridge over the river: a 4x4 zone area of mapsquare 50_50
/// whose bridge tiles, on level 1, are linked below.
const BRIDGE: (u16, u16) = (3232, 3216);

#[test]
fn zones_set_over_the_bridge_build_the_same_from_either_level() {
    runner::on_loaded_engine(|engine| {
        let every_level = |engine: &Engine, to: CoordGrid| {
            (0..4)
                .map(|level| locs_on(engine, to.x(), to.z(), level))
                .collect::<Vec<_>>()
        };
        for rotation in 0..4 {
            let up = engine
                .harness_region_create(4, 16, 16)
                .expect("a free region");
            fill(engine, up, BRIDGE, &[0, 1, 2, 3], rotation);
            let to = engine.harness_region_coord(up, 0, 16, 16).unwrap();
            let expected = (area_flags(to), every_level(engine, to));

            // From the top, each bridge lands under levels already in and
            // drops their locs and floors a level.
            let down = engine
                .harness_region_create(4, 16, 16)
                .expect("a free region");
            fill(engine, down, BRIDGE, &[3, 2, 1, 0], rotation);
            let to = engine.harness_region_coord(down, 0, 16, 16).unwrap();
            assert!(
                (area_flags(to), every_level(engine, to)) == expected,
                "rotation {rotation}"
            );

            engine.harness_region_free(up);
            engine.harness_region_free(down);
            engine.cycle();
        }
    });
}

// The same zones set onto two regions in different orders build the same
/// map: castle zones on levels 0 and 2 of every column, turned every way,
/// and on level 1 of most the bridge zones, which drop the castle walls and
/// floors above them a level. The second region gets its bridges last, when
/// the walls they drop already block the zones beside them, and nothing set
/// later builds those zones again.
#[test]
fn the_order_zones_are_set_in_does_not_matter() {
    runner::on_loaded_engine(|engine| {
        let zones = TILES >> 3;
        let mut seed = 0x2545_f491_u32;
        let mut next = move |n: u32| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed % n
        };
        let mut sets = Vec::new();
        for i in 0..zones {
            for j in 0..zones {
                for level in [0, 2] {
                    let (a, b) = (next(4) as u16, next(4) as u16);
                    let castle = CoordGrid::new(SOURCE.0 + (a << 3), 0, SOURCE.1 + (b << 3));
                    sets.push((level, i, j, castle, next(4)));
                }
                if next(4) != 0 {
                    let bridge = CoordGrid::new(3240 + ((next(2) as u16) << 3), 1, 3224);
                    sets.push((1, i, j, bridge, next(4)));
                }
            }
        }
        let every_level = |engine: &Engine, to: CoordGrid| {
            (0..4)
                .map(|level| locs_on(engine, to.x(), to.z(), level))
                .collect::<Vec<_>>()
        };
        let build = |engine: &mut Engine, sets: &[(u8, u16, u16, CoordGrid, u32)]| {
            let uid = engine
                .harness_region_create(4, 8, 8)
                .expect("a free region");
            for &(level, i, j, source, rotation) in sets {
                engine
                    .harness_region_set(
                        uid,
                        level as i32,
                        2 + i as i32,
                        2 + j as i32,
                        source,
                        rotation as i32,
                    )
                    .unwrap();
            }
            let to = engine.harness_region_coord(uid, 0, 16, 16).unwrap();
            (uid, (area_flags(to), every_level(engine, to)))
        };
        sets.sort_by_key(|&(level, ..)| level);
        let (first, bridges_first) = build(engine, &sets);
        sets.sort_by_key(|&(level, ..)| level == 1);
        let (second, bridges_last) = build(engine, &sets);
        assert!(bridges_first == bridges_last);
        engine.harness_region_free(first);
        engine.harness_region_free(second);
    });
}

#[test]
fn regions_keep_to_the_size_they_were_created_with() {
    runner::on_loaded_engine(|engine| {
        for (levels, east, north) in [(0, 1, 1), (5, 1, 1), (1, 0, 1), (1, 41, 1), (1, 1, 41)] {
            assert!(
                engine.harness_region_create(levels, east, north).is_none(),
                "{levels} levels of {east}x{north} zones"
            );
        }
        let uid = engine.harness_region_create(2, 3, 4).unwrap();
        let source = CoordGrid::new(SOURCE.0, 0, SOURCE.1);
        assert!(engine.harness_region_set(uid, 1, 2, 3, source, 3).is_ok());
        for (level, east, north, rotation) in [
            (2, 0, 0, 0),
            (-1, 0, 0, 0),
            (0, 3, 0, 0),
            (0, 0, 4, 0),
            (0, 0, 0, 4),
        ] {
            assert!(
                engine
                    .harness_region_set(uid, level, east, north, source, rotation)
                    .is_err(),
                "zone {east}, {north} on level {level}, turned {rotation}"
            );
        }
        let corner = engine.harness_region_coord(uid, 1, 127, 127).unwrap();
        assert!(
            engine.harness_region_set(uid, 0, 0, 0, corner, 0).is_err(),
            "a region tile is no source"
        );
        assert!(engine.harness_region_coord(uid, 4, 0, 0).is_none());
        assert!(engine.harness_region_coord(uid, 0, 128, 0).is_none());
        engine.harness_region_free(uid);
        assert!(engine.harness_region_set(uid, 0, 0, 0, source, 0).is_err());

        let large = engine.harness_region_create(1, 17, 1).unwrap();
        let origin = engine.harness_region_coord(large, 0, 0, 0).unwrap();
        assert!(origin.z() >= 5248, "a 320x320 slot");
        assert!(engine.harness_region_coord(large, 0, 319, 319).is_some());
        engine.harness_region_free(large);
    });
}
