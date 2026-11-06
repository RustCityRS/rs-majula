//! 289's ladders and stairs, and 2011's that 289's scripts do not reach.
//!
//! 2011 re-issued the Lumbridge castle cellar's ladder as 29355 and rebuilt the
//! castle's staircases (36773...), none of which 289 names: they climb by
//! void-634's record of each 2011 placement (`loc_tele_2011`).
//! `OPLOC1`: `p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

fn op_loc1(client: &mut HarnessClient, loc: u16, x: u16, z: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

/// `OPLOC2`: `p2_alt1(loc) p1_alt2(ctrl) p2_alt2(z) p2_alt3(x)`.
fn op_loc2(client: &mut HarnessClient, loc: u16, x: u16, z: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt1(loc as i32)
        .p1_alt2(0)
        .p2_alt2(z as i32)
        .p2_alt3(x as i32);
    client.send(ClientProt::OpLoc2, &p.finish());
}

fn coord(engine: &Engine, pid: u16) -> (u16, u8, u16) {
    let c = engine.harness_player(pid).unwrap().pathing.coord;
    (c.x(), c.y(), c.z())
}

fn climber(engine: &mut Engine, name: &str, x: u16, y: u8, z: u16) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, x, y, z);
    for _ in 0..4 {
        engine.cycle();
    }
    client.packets();
    client
}

#[test]
fn the_cook_s_cellar_ladder_climbs_back_to_the_kitchen() {
    runner::on_loaded_engine(|engine| {
        // Down through the kitchen's trapdoor (36687), as 2011 has it.
        let mut client = climber(engine, "cellarman", 3209, 0, 3217);
        op_loc1(&mut client, 36687, 3209, 3216);
        for _ in 0..8 {
            engine.cycle();
        }
        let (x, _, z) = coord(engine, client.pid);
        assert!(z > 9600, "in the cellar: at {x},{z}");

        // Up 2011's ladder 29355 at 3209,9616: 6400 north, by its default.
        op_loc1(&mut client, 29355, 3209, 9616);
        for _ in 0..10 {
            engine.cycle();
        }
        let (x, y, z) = coord(engine, client.pid);
        assert!(z < 6400 && y == 0, "back in the kitchen: at {x},{y},{z}");
        assert!(
            (3205..=3213).contains(&x) && (3212..=3220).contains(&z),
            "by the trapdoor: {x},{z}"
        );
    });
}

#[test]
fn lumbridge_castle_s_2011_staircase_climbs_a_floor() {
    runner::on_loaded_engine(|engine| {
        // 2011's staircase_36773 at 3204,3207 (void-634: up a floor).
        let mut client = climber(engine, "stairman", 3205, 0, 3209);
        op_loc1(&mut client, 36773, 3204, 3207);
        for _ in 0..10 {
            engine.cycle();
        }
        let (_, y, _) = coord(engine, client.pid);
        assert_eq!(y, 1, "on the first floor");
    });
}

/// Etceteria castle's stairs are spiral stairs, up (1738) and down (1740), on
/// the same tiles at 289 and 2011, and 289 climbs them as `spiralstairs` and
/// `spiralstairstop`.
#[test]
fn etceteria_castle_s_2011_spiral_stairs_climb_as_289_s() {
    runner::on_loaded_engine(|engine| {
        let mut client = climber(engine, "etceteria", 2613, 0, 3866);
        op_loc1(&mut client, 1738, 2613, 3867);
        for _ in 0..10 {
            engine.cycle();
        }
        assert_eq!(
            coord(engine, client.pid),
            (2615, 1, 3867),
            "up to 289's tile"
        );

        // From where the climb up lands.
        let mut client = climber(engine, "etceteria2", 2615, 1, 3867);
        op_loc1(&mut client, 1740, 2614, 3867);
        for _ in 0..10 {
            engine.cycle();
        }
        assert_eq!(
            coord(engine, client.pid),
            (2614, 0, 3866),
            "down to 289's tile"
        );
    });
}

/// Miscellania castle's middle stairs (1743) stand on 289's tiles at 2011:
/// Climb-up (op2) takes the player up a floor, as 289's script does.
#[test]
fn miscellania_castle_s_stairs_climb_up() {
    runner::on_loaded_engine(|engine| {
        let mut client = climber(engine, "miscellania", 2505, 1, 3847);
        op_loc2(&mut client, 1743, 2505, 3848);
        for _ in 0..10 {
            engine.cycle();
        }
        let (_, y, _) = coord(engine, client.pid);
        assert_eq!(y, 2, "a floor up");
    });
}
