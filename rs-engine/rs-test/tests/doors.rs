//! 289's doors, gates and trapdoors at 2011's placements.
//!
//! Each case clicks as the 647 client does (`OPLOC1`: `p2_alt1(z) p2_alt1(loc)
//! p1(ctrl) p2_alt2(x)`; `OPLOC2`: `p2_alt1(loc) p1_alt2(ctrl) p2_alt2(z)
//! p2_alt3(x)`) and reads the zone's locs and the packets sent back.
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

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use std::fs;

/// 647 SYNTH_SOUND: `p2 synth; p1 loops; p2 delay; p1 volume; p2 speed`.
const SYNTH_SOUND: u16 = 50;
const DOOR_OPEN: u16 = 81;
const GATE_OPEN: u16 = 67;
const LOCKED: u16 = 2402;
const THIEVING: usize = 17;

fn op_loc(client: &mut HarnessClient, prot: ClientProt, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    match prot {
        ClientProt::OpLoc1 => p
            .p2_alt1(z as i32)
            .p2_alt1(loc as i32)
            .p1(0)
            .p2_alt2(x as i32),
        _ => p
            .p2_alt1(loc as i32)
            .p1_alt2(0)
            .p2_alt2(z as i32)
            .p2_alt3(x as i32),
    };
    client.send(prot, &p.finish());
}

/// A loc's 647 id by its name in `content/647/pack/loc.pack` (the server
/// loads 647 locs from the cache's data alone, which carries no names).
fn loc_id(_: &Engine, name: &str) -> u16 {
    let pack = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/647/pack/loc.pack"
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
    let idx = zone.get_loc(x, z, id)?;
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
        synths.extend(
            packets
                .iter()
                .filter(|(op, _)| *op == SYNTH_SOUND)
                .map(|(_, p)| u16::from_be_bytes([p[0], p[1]])),
        );
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
