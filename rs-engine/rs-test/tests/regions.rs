//! Regions: static zones copied into region space, built as the 647 client
//! builds them from the same templates, sent with REBUILD_REGION, saved as
//! their controller's type and torn down.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_grid::CoordGrid;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;
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

/// Every active loc lying wholly in the area from (`x`, `z`), as
/// `(id, x, z, angle, shape, width, length)` relative to (`x`, `z`).
fn locs(engine: &Engine, x: u16, z: u16) -> Vec<(u16, u16, u16, u8, u8, u16, u16)> {
    let mut locs = Vec::new();
    for zx in (0..TILES + 16).step_by(8) {
        for zz in (0..TILES + 16).step_by(8) {
            let Some(zone) = engine.zones.zone(x + zx - 8, 0, z + zz - 8) else {
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

fn copy(engine: &mut Engine, rotation: usize) -> (u32, CoordGrid) {
    let uid = engine.harness_region_create(0).expect("a free region");
    let to = engine.harness_region_coord(uid, 16, 0, 16).unwrap();
    engine
        .harness_region_copy(
            uid,
            CoordGrid::new(SOURCE.0, 0, SOURCE.1),
            CoordGrid::new(SOURCE.0 + TILES - 1, 0, SOURCE.1 + TILES - 1),
            to,
            rotation as i32,
        )
        .unwrap();
    (uid, to)
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

            engine.harness_region_del(uid);
            engine.cycle();
            assert!(!rsmod::is_zone_allocated(to.x(), to.z(), 0));
        }
    });
}

#[test]
fn copied_zones_keep_their_sources_ground_and_free_mark() {
    runner::on_loaded_engine(|engine| {
        let uid = engine.harness_region_create(0).expect("a free region");
        let to = engine.harness_region_coord(uid, 16, 0, 16).unwrap();
        engine
            .harness_region_copy(
                uid,
                CoordGrid::new(SOURCE.0, 0, SOURCE.1),
                CoordGrid::new(SOURCE.0 + TILES - 1, 3, SOURCE.1 + TILES - 1),
                to,
                0,
            )
            .unwrap();
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
        engine.harness_region_del(uid);
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
        let inside = engine.harness_region_coord(uid, 24, 0, 24).unwrap();
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

        let (saved, login) = engine.harness_saved_coord(client.pid).unwrap();
        assert_eq!(
            saved, inside,
            "no controller: the region tile, never a destination"
        );
        assert_eq!((login.x(), login.z()), (3221, 3218));
        let manager = engine.harness_region_coord(uid, 0, 0, 0).unwrap();
        assert!(engine.harness_controller_add(manager, 0));
        let (saved, login) = engine.harness_saved_coord(client.pid).unwrap();
        assert_eq!(saved, CoordGrid::new(6400, 0, 0), "the controller's type");
        assert_eq!(
            (login.x(), login.z()),
            (3221, 3218),
            "controller_0 gives no exit: home"
        );

        engine.harness_region_del(uid);
        let coord = engine.harness_player(client.pid).unwrap().pathing.coord;
        assert_eq!((coord.x(), coord.z()), (3221, 3218));
        assert!(
            !rsmod::is_zone_allocated(inside.x(), inside.z(), 0),
            "nothing reaches a deleted region later in its tick"
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
