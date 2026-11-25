//! 289's doors, gates and trapdoors at 2011's placements.
//!
//! Each case clicks as the 647 client does and reads the zone's locs and the
//! packets sent back.
//!
//! - A single door (2011's `door_1530` in Yanille, 289's `loc_1530`) swings a
//!   quarter turn onto the next tile as `door_1531` and plays 289's door_open,
//!   2011's synth 81 (void-634's id; 289's sound is not in 2011's cache).
//! - A double gate (`gate_1551` and `gate_1553`) opens both leaves together and
//!   plays void-634's gate_open (synth 67), where 289 played door_open.
//! - A double door (`large_door_1516` and `large_door_1519`) opens both leaves.
//! - 289's lockeddoor1 refuses in two messages a tick apart.
//! - 2011's East Ardougne level-1 locked door (`door_34812`, 289's `loc_2550`):
//!   from outside it is locked and plays 289's `locked`; from inside the
//!   player goes through.
//! - Edgeville's trapdoor (2011's `trapdoor_26933`/`_26934`, on 289's
//!   `trapdoor` spot) opens, then climbs down to the dungeon, 6399 north as
//!   void-634 has it.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
#[cfg(since_647)]
use rs_test::life;
use rs_test::runner;
use rs_test::server;
use rs_ver::TypeId;
use std::fs;

const DOOR_OPEN: u16 = 81;
const GATE_OPEN: u16 = 67;
const LOCKED: u16 = 2402;
const THIEVING: usize = 17;
#[cfg(since_647)]
const HITPOINTS: usize = 3;
/// The locked side of `picklock6` at 2610,3316.
#[cfg(since_647)]
const SIDE: (u16, u16) = (2610, 3316);

fn op_loc(client: &mut HarnessClient, prot: ClientProt, (loc, x, z): (u16, u16, u16)) {
    let op = match prot {
        ClientProt::OpLoc1 => 1,
        _ => 2,
    };
    client.op_loc(op, x, z, TypeId::new(loc));
}

/// A loc's id by its name in the rev's `pack/loc.pack` (the server
/// loads 647 locs from the cache's data alone, which carries no names).
fn loc_id(_: &Engine, name: &str) -> u16 {
    let pack = &format!(
        "{}/../../{}/pack/loc.pack",
        env!("CARGO_MANIFEST_DIR"),
        rs_pack::CONTENT_DIR
    );
    fs::read_to_string(pack)
        .expect("loc.pack")
        .lines()
        .find_map(|l| l.split_once('=').filter(|(_, n)| *n == name))
        .and_then(|(id, _)| id.parse().ok())
        .unwrap_or_else(|| panic!("647 has no loc {name}"))
}

/// The angle of the visible loc `name` at `x`,`z` on level 0, if it is there.
fn loc_at(engine: &Engine, name: &str, x: u16, z: u16) -> Option<u8> {
    let id = loc_id(engine, name);
    let zone = engine.zones.zone(x, 0, z)?;
    let idx = zone.get_loc(x, z, TypeId::from(id))?;
    Some(zone.locs[idx].angle() as u8)
}

/// A logged-in player standing at `x`,`z`, its login packets drained.
fn visitor(engine: &mut Engine, name: &str, x: u16, z: u16) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, x, 0, z);
    for _ in 0..4 {
        engine.cycle();
    }
    client.packets();
    client
}

/// Cycles `ticks` times: the game messages said and the synths played.
fn run(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> (Vec<String>, Vec<u16>) {
    let (mut said, mut synths) = (Vec::new(), Vec::new());
    for _ in 0..ticks {
        engine.cycle();
        let packets = client.packets();
        said.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
        synths.extend(server::synths(&packets).into_iter().map(|(s, _)| s));
    }
    (said, synths)
}

fn coord(engine: &Engine, pid: u16) -> (u16, u8, u16) {
    let c = engine.harness_player(pid).unwrap().pathing.coord;
    (c.x(), c.y(), c.z())
}

#[test]
fn a_single_door_swings_onto_the_next_tile_with_2011s_door_sound() {
    runner::on_loaded_engine(|engine| {
        // Yanille: door_1530 on the south edge of 2551,3098 (angle 3).
        assert_eq!(loc_at(engine, "poordoor", 2551, 3098), Some(3));
        let mut client = visitor(engine, "doorman", 2551, 3099);
        op_loc(
            &mut client,
            ClientProt::OpLoc1,
            (loc_id(engine, "poordoor"), 2551, 3098),
        );
        let (said, synths) = run(engine, &mut client, 6);
        assert_eq!(loc_at(engine, "poordoor", 2551, 3098), None, "{said:?}");
        // 289's ~door_open: south swings to (0,-1), a quarter turn on.
        assert_eq!(loc_at(engine, "poordooropen", 2551, 3097), Some(0));
        assert_eq!(synths, [DOOR_OPEN]);
    });
}

#[test]
fn a_double_gate_opens_both_leaves_with_2011s_gate_sound() {
    runner::on_loaded_engine(|engine| {
        // gate_1551 (main) at 2513,3494 and gate_1553 (outer) at 2513,3495,
        // both facing west.
        assert_eq!(loc_at(engine, "fencegate_l", 2513, 3494), Some(0));
        assert_eq!(loc_at(engine, "fencegate_r", 2513, 3495), Some(0));
        let mut client = visitor(engine, "gatekeeper", 2512, 3494);
        op_loc(
            &mut client,
            ClientProt::OpLoc1,
            (loc_id(engine, "fencegate_l"), 2513, 3494),
        );
        let (said, synths) = run(engine, &mut client, 6);
        assert_eq!(loc_at(engine, "fencegate_l", 2513, 3494), None, "{said:?}");
        assert_eq!(
            loc_at(engine, "fencegate_r", 2513, 3495),
            None,
            "the outer leaf opened too"
        );
        let zone = |x: u16, z: u16, name: &str| loc_at(engine, name, x, z).is_some();
        let opened = |name: &str| (2510..2517).any(|x| (3490..3500).any(|z| zone(x, z, name)));
        assert!(opened("gate_1552"), "the main leaf's open loc");
        assert!(opened("openfencegate_r"), "the outer leaf's open loc");
        assert_eq!(synths, [GATE_OPEN]);
    });
}

#[test]
fn a_double_door_opens_both_leaves() {
    runner::on_loaded_engine(|engine| {
        // large_door_1516 (left) at 2613,3481 and large_door_1519 (right) at
        // 2612,3481, both facing south.
        assert_eq!(loc_at(engine, "castledoubledoorl", 2613, 3481), Some(3));
        assert_eq!(loc_at(engine, "castledoubledoorr", 2612, 3481), Some(3));
        let mut client = visitor(engine, "doubledoor", 2613, 3482);
        op_loc(
            &mut client,
            ClientProt::OpLoc1,
            (loc_id(engine, "castledoubledoorl"), 2613, 3481),
        );
        let (said, synths) = run(engine, &mut client, 6);
        assert_eq!(
            loc_at(engine, "castledoubledoorl", 2613, 3481),
            None,
            "{said:?}"
        );
        assert_eq!(
            loc_at(engine, "castledoubledoorr", 2612, 3481),
            None,
            "the other leaf"
        );
        let opened = |name: &str| {
            (2609..2617).any(|x| (3478..3485).any(|z| loc_at(engine, name, x, z).is_some()))
        };
        assert!(opened("opencastledoubledoorl"));
        assert!(opened("opencastledoubledoorr"));
        assert_eq!(synths, [DOOR_OPEN], "289's default open_sound");
    });
}

#[test]
fn lumbridge_dining_room_doors_let_the_player_through_then_shut() {
    runner::on_loaded_engine(|engine| {
        let mut client = visitor(engine, "diningroom", 3213, 3221);
        for (door, (x, z), across, side) in [
            (
                "hundred_lumbridge_doubledoorl",
                (3213, 3221),
                (3212, 0, 3221),
                (3213, 3220),
            ),
            (
                "hundred_lumbridge_doubledoorr",
                (3213, 3222),
                (3212, 0, 3222),
                (3213, 3223),
            ),
            (
                "hundred_lumbridge_door",
                (3207, 3217),
                (3207, 0, 3218),
                (3206, 3217),
            ),
        ] {
            engine.harness_telejump(client.pid, side.0, 0, side.1);
            engine.cycle();
            let id = TypeId::from(loc_id(engine, door));
            for destination in [across, (x, 0, z)] {
                client.op_loc(1, x, z, id);
                if destination == across {
                    let (_, approach_sounds) = run(engine, &mut client, 2);
                    assert!(loc_at(engine, door, x, z).is_some(), "{door}");
                    assert!(
                        approach_sounds.is_empty(),
                        "{door} played {approach_sounds:?} during approach"
                    );
                }
                let (_, opening_sounds) = run(engine, &mut client, 1);
                assert_eq!(opening_sounds, [DOOR_OPEN], "{door}");
                assert_eq!(coord(engine, client.pid), destination, "{door}");
                assert!(loc_at(engine, door, x, z).is_none(), "{door}");
                if x == 3213 {
                    let (other, other_z) = if z == 3221 {
                        ("hundred_lumbridge_doubledoorr", 3222)
                    } else {
                        ("hundred_lumbridge_doubledoorl", 3221)
                    };
                    assert_eq!(loc_at(engine, other, x, other_z), Some(0));
                }
                run(engine, &mut client, 4);
                assert!(loc_at(engine, door, x, z).is_some(), "{door}");
            }
        }
    });
}

#[test]
fn a_permanently_locked_door_refuses() {
    runner::on_loaded_engine(|engine| {
        // lockeddoor1 at 2794,3199 (angle 1, a straight wall).
        assert!(loc_at(engine, "lockeddoor1", 2794, 3199).is_some());
        let mut client = visitor(engine, "lockedout", 2794, 3198);
        op_loc(
            &mut client,
            ClientProt::OpLoc1,
            (loc_id(engine, "lockeddoor1"), 2794, 3199),
        );
        let (said, _) = run(engine, &mut client, 6);
        assert_eq!(
            said,
            [
                "You try and open the door...",
                "The door is locked tight, I can't open it."
            ]
        );
        assert!(
            loc_at(engine, "lockeddoor1", 2794, 3199).is_some(),
            "still shut"
        );
    });
}

#[test]
fn ardougnes_level_one_locked_door_is_locked_from_outside_and_open_from_in() {
    runner::on_loaded_engine(|engine| {
        // door_34812 on the east edge of 2672,3301 (angle 2). On its own tile
        // a player is outside (289's ~check_axis_locactive: on the door's
        // column), as void-634 has it (`tile.x <= target.tile.x` is locked).
        assert_eq!(
            loc_at(engine, "area_ardougne_door2_locked", 2672, 3301),
            Some(2)
        );
        let door = (loc_id(engine, "area_ardougne_door2_locked"), 2672, 3301);
        let mut client = visitor(engine, "outsider", 2672, 3301);
        op_loc(&mut client, ClientProt::OpLoc1, door);
        let (said, synths) = run(engine, &mut client, 6);
        assert_eq!(said, ["This door is locked."]);
        assert_eq!(synths, [LOCKED]);
        assert_eq!(coord(engine, client.pid), (2672, 0, 3301));

        let mut client = visitor(engine, "insider", 2673, 3301);
        op_loc(&mut client, ClientProt::OpLoc1, door);
        let (said, synths) = run(engine, &mut client, 6);
        assert_eq!(said, ["You go through the door."]);
        assert_eq!(synths, [DOOR_OPEN], "289 plays `locked` only going in");
        assert_eq!(
            coord(engine, client.pid),
            (2672, 0, 3301),
            "stepped out west"
        );
    });
}

#[test]
fn ardougnes_level_one_locked_door_picks_from_outside() {
    runner::on_loaded_engine(|engine| {
        // The row sets no `trap_successchance`, its table's last column: 289
        // counts it as no fields, and so must a 647 row that stops short of it.
        // The door's other placement, 2672,3308: the test world is shared.
        assert_eq!(
            loc_at(engine, "area_ardougne_door2_locked", 2672, 3308),
            Some(2)
        );
        let door = (loc_id(engine, "area_ardougne_door2_locked"), 2672, 3308);
        let mut client = visitor(engine, "picklock", 2672, 3308);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.stats.levels[THIEVING] = 99;
        player.stats.base_levels[THIEVING] = 99;
        // 289's roll is 32-250 of 256 at 99: retry the odd failure.
        let mut said = Vec::new();
        for _ in 0..5 {
            op_loc(&mut client, ClientProt::OpLoc2, door);
            said = run(engine, &mut client, 6).0;
            if said.iter().any(|s| s == "You manage to pick the lock.") {
                break;
            }
        }
        assert_eq!(
            said,
            [
                "You attempt to pick the lock.",
                "You manage to pick the lock."
            ]
        );
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[THIEVING];
        assert_eq!(xp, 38, "3.8 xp");
        assert_eq!(coord(engine, client.pid), (2673, 0, 3308), "through, east");
    });
}

/// 289's trap on a lock costs a tenth of the current hitpoints
/// (`divide(stat(hitpoints), 10)`); 2011 counts life points, ten to a
/// hitpoint, so 990 life points lose 90. `picklock6` (289's `loc_2556`) at
/// 2610,3316 springs its trap on 289's 32-250 of 256 roll: 7 picks in 8 at
/// 1 Thieving. 89 Hitpoints cap the natural recovery under the 990 set here,
/// so the trap is the only change.
#[cfg(since_647)]
#[test]
fn a_trapped_lock_costs_a_tenth_of_the_life_points() {
    runner::on_loaded_engine(|engine| {
        assert_eq!(loc_at(engine, "picklock6", 2610, 3316), Some(2));
        let door = (loc_id(engine, "picklock6"), 2610, 3316);
        let mut client = visitor(engine, "trapped", SIDE.0, SIDE.1);
        let life = |engine: &Engine, pid| life::get(engine, pid);
        let mut lost = None;
        for _ in 0..12 {
            let player = engine.harness_player_mut(client.pid).unwrap();
            player.stats.levels[THIEVING] = 1;
            player.stats.base_levels[THIEVING] = 1;
            player.stats.levels[HITPOINTS] = 89;
            player.stats.base_levels[HITPOINTS] = 89;
            life::set(player, 99 * life::SCALE);
            engine.cycle();
            client.packets();
            op_loc(&mut client, ClientProt::OpLoc2, door);
            let pid = client.pid;
            let (mut was, mut drops, mut said) = (life(engine, pid), Vec::new(), Vec::new());
            for _ in 0..6 {
                said.extend(run(engine, &mut client, 1).0);
                let now = life(engine, pid);
                if now < was {
                    drops.push(was - now);
                }
                was = now;
            }
            assert_eq!(
                said.first().map(String::as_str),
                Some("You attempt to pick the lock.")
            );
            if said
                .iter()
                .any(|s| s == "You have activated a trap on the lock.")
            {
                lost = Some(drops);
                break;
            }
        }
        assert_eq!(
            lost,
            Some(vec![90]),
            "a trap in twelve picks, for 90 of 990"
        );
    });
}

#[test]
fn edgevilles_trapdoor_opens_and_climbs_down_to_the_dungeon() {
    runner::on_loaded_engine(|engine| {
        assert!(loc_at(engine, "trapdoor_26933", 3097, 3468).is_some());
        let mut client = visitor(engine, "dungeoneer", 3097, 3469);
        op_loc(
            &mut client,
            ClientProt::OpLoc1,
            (loc_id(engine, "trapdoor_26933"), 3097, 3468),
        );
        let (said, synths) = run(engine, &mut client, 6);
        assert_eq!(said, ["The trapdoor opens..."]);
        assert_eq!(synths, [DOOR_OPEN]);
        assert!(loc_at(engine, "edge_trapdoor_open", 3097, 3468).is_some());

        let (x, _, z) = coord(engine, client.pid);
        op_loc(
            &mut client,
            ClientProt::OpLoc1,
            (loc_id(engine, "edge_trapdoor_open"), 3097, 3468),
        );
        let (said, _) = run(engine, &mut client, 6);
        assert_eq!(said, ["You climb down through the trapdoor..."]);
        assert_eq!(coord(engine, client.pid), (x, 0, z + 6399));
    });
}
