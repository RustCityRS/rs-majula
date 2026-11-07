//! Ernest the Chicken at 647 (`quests/quest_haunted`, `areas/area_draynor`):
//! 289's quest in 2011's Draynor Manor. The basement's levers and doors are
//! per player (multilocs on %ernestlever and %ernest_doors), the secret room
//! opens from a candle sconce, and the front doors open from outside only.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{explore, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

const LEVER_A: (u16, u16, u16) = (146, 3108, 9745);
const LEVER_B: (u16, u16, u16) = (147, 3118, 9752);
/// 289's 4to7, 2011's loc_144 on bit 7 of %ernest_doors.
const DOOR_4TO7: (u16, u16, u16) = (144, 3108, 9758);
/// 289's 1to2, 2011's loc_137 on bit 0.
const DOOR_1TO2: (u16, u16, u16) = (137, 3105, 9765);

fn varp(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .varps
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no varp {name}"))
}

fn loc(engine: &Engine, name: &str) -> u16 {
    let pack = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/647/pack/loc.pack"
    );
    let _ = engine;
    std::fs::read_to_string(pack)
        .unwrap()
        .lines()
        .find_map(|l| {
            let (id, n) = l.split_once('=')?;
            (n == name).then(|| id.parse().unwrap())
        })
        .unwrap_or_else(|| panic!("loc.pack names {name}"))
}

/// `OPLOC1`: `p2_alt1(z) p2_alt1(loc) p1(ctrl) p2_alt2(x)`.
fn op_loc1(client: &mut HarnessClient, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

/// `OPLOC2`: `p2_alt1(loc) p1_alt2(ctrl) p2_alt2(z) p2_alt3(x)`.
fn op_loc2(client: &mut HarnessClient, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt1(loc as i32)
        .p1_alt2(0)
        .p2_alt2(z as i32)
        .p2_alt3(x as i32);
    client.send(ClientProt::OpLoc2, &p.finish());
}

/// A walkable tile beside (x, z), one step away.
fn next_to(x: u16, level: u8, z: u16) -> (u16, u16) {
    [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(x, z)| !rsmod::is_flagged(x, z, level, 0x100 | 0x200000))
        .unwrap_or_else(|| panic!("a walkable tile beside {x},{z}"))
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) {
    for _ in 0..n {
        engine.cycle();
        client.packets();
    }
}

fn login(engine: &mut Engine, name: &str, (x, level, z): (u16, u8, u16)) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, x, level, z);
    ticks(engine, &mut client, 3);
    rs_vm::ops::test::set_recording(true);
    client
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

fn said(text: &str) -> bool {
    rs_vm::ops::test::calls_for("mes").iter().any(|m| m == text)
}

/// Levers start down (2011 wiki: "All levers start in the down position");
/// pulling B then A unlocks 289's 4to7 alone, which then lets the player
/// through; a door the levers keep locked offers no Open, so a click on it does
/// nothing.
#[test]
fn the_basement_levers_unlock_2011_s_doors() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let levers = varp(engine, "ernestlever");
        let doors = varp(engine, "ernest_doors");
        assert_eq!((levers, doors), (33, 668), "the cache's varps");
        let (bx, bz) = next_to(LEVER_B.1, 0, LEVER_B.2);
        let mut client = login(engine, "ernest_levers", (bx, 0, bz));
        let _ = engine.script_errors.take();
        let get = |engine: &Engine, client: &HarnessClient, id: u16| {
            engine
                .harness_player(client.pid)
                .unwrap()
                .vars
                .get(id)
                .as_int()
        };
        assert_eq!(
            get(engine, &client, doors),
            0,
            "no door unlocked at the start"
        );

        op_loc2(&mut client, LEVER_B);
        ticks(engine, &mut client, 6);
        assert!(
            said("The lever is down."),
            "{:?}",
            rs_vm::ops::test::calls_for("mes")
        );
        op_loc1(&mut client, LEVER_B);
        ticks(engine, &mut client, 6);
        assert!(
            said("You pull lever B up."),
            "{:?}",
            rs_vm::ops::test::calls_for("mes")
        );
        assert_eq!(get(engine, &client, levers), 1 << 2, "lever B is bit 2");

        let (ax, az) = next_to(LEVER_A.1, 0, LEVER_A.2);
        engine.harness_telejump(client.pid, ax, 0, az);
        ticks(engine, &mut client, 3);
        op_loc1(&mut client, LEVER_A);
        ticks(engine, &mut client, 6);
        assert!(said("You pull lever A up."));
        assert_eq!(get(engine, &client, levers), (1 << 1) | (1 << 2));
        assert_eq!(
            get(engine, &client, doors),
            1 << 7,
            "4to7 alone is unlocked"
        );

        // 4to7 lets the player through to the room beyond.
        let (dx, dz) = next_to(DOOR_4TO7.1, 0, DOOR_4TO7.2);
        engine.harness_telejump(client.pid, dx, 0, dz);
        ticks(engine, &mut client, 3);
        op_loc1(&mut client, DOOR_4TO7);
        ticks(engine, &mut client, 8);
        let beyond = (2 * DOOR_4TO7.1 - dx, 2 * DOOR_4TO7.2 - dz);
        assert_eq!(
            coord(engine, &client),
            beyond,
            "through 4to7 from {dx},{dz}"
        );

        // 1to2 stays locked: no Open to click.
        let (lx, lz) = next_to(DOOR_1TO2.1, 0, DOOR_1TO2.2);
        engine.harness_telejump(client.pid, lx, 0, lz);
        ticks(engine, &mut client, 3);
        op_loc1(&mut client, DOOR_1TO2);
        ticks(engine, &mut client, 8);
        assert_eq!(
            coord(engine, &client),
            (lx, lz),
            "a locked door does not open"
        );

        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}

/// 2011's secret room opens from the candle sconce on the west room's wall
/// (2011 wiki: "searching the candles hanging on the wall in the westernmost
/// room"), to where 289's bookcase sent the player.
#[test]
fn the_candle_sconce_opens_the_secret_room() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let sconce = (loc(engine, "candle_sconce"), 3097, 3360);
        let mut client = login(engine, "ernest_sconce", (3098, 0, 3360));
        op_loc1(&mut client, sconce);
        ticks(engine, &mut client, 8);
        assert!(said("You've found a secret door!"));
        assert_eq!(coord(engine, &client), (3096, 3359), "in the secret room");
    });
}

/// The front doors open from outside (south) only (2011 wiki, Vampire Slayer:
/// "To exit Draynor Manor you will not be able to use the main entrance").
#[test]
fn the_front_doors_open_from_outside_only() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let door = (loc(engine, "large_door_47421"), 3108, 3353);
        let mut client = login(engine, "ernest_doors_out", (3108, 0, 3352));
        op_loc1(&mut client, door);
        ticks(engine, &mut client, 10);
        assert!(
            said("The doors slam shut behind you."),
            "{:?}",
            rs_vm::ops::test::calls_for("mes")
        );
        assert!(
            coord(engine, &client).1 > 3353,
            "inside: {:?}",
            coord(engine, &client)
        );

        let mut client = login(engine, "ernest_doors_in", (3108, 0, 3354));
        op_loc1(&mut client, door);
        ticks(engine, &mut client, 10);
        assert!(said("The doors won't open."));
        assert_eq!(coord(engine, &client), (3108, 3354));
    });
}

/// A spawn of npc `name`, and a tile beside it that one step from the npc
/// reaches.
fn beside(engine: &Engine, name: &str) -> (u16, u16, u8, u16) {
    let id = *engine.cache.npcs.debugnames.get(name).unwrap();
    let (nid, nx, y, nz) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.y(), c.z())
        })
        .unwrap_or_else(|| panic!("{name} is spawned"));
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|&(dx, dz)| rsmod::can_travel(y, nx, nz, dx, dz, 1, 0, CollisionType::Normal))
        .map(|(dx, dz)| {
            (
                nx.wrapping_add_signed(dx as i16),
                nz.wrapping_add_signed(dz as i16),
            )
        })
        .unwrap_or_else(|| panic!("a tile beside {name}"));
    (nid, sx, y, sz)
}

/// Veronica and Professor Oddenstein: every way through their Talk-to at each
/// stage (%haunted 0 to 3) says something and fails no script.
#[test]
fn veronica_and_oddenstein_answer_every_branch() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let haunted = varp(engine, "haunted");
        for npc in ["veronica", "professor_oddenstein"] {
            for stage in 0..=3 {
                let _ = engine.script_errors.take();
                let branches = explore(
                    engine,
                    |engine| {
                        engine.harness_reset();
                        let mut client = engine
                            .harness_login_returning("ernest_talker")
                            .expect("login");
                        for _ in 0..10 {
                            engine.cycle();
                        }
                        let (nid, x, y, z) = beside(engine, npc);
                        engine.harness_telejump(client.pid, x, y, z);
                        engine
                            .harness_player_mut(client.pid)
                            .unwrap()
                            .vars
                            .set(haunted, VarValue::Int(stage));
                        ticks(engine, &mut client, 3);
                        talk_to(&mut client, nid);
                        client
                    },
                    24,
                );
                let errors: Vec<String> = engine
                    .script_errors
                    .take()
                    .into_iter()
                    .map(|e| format!("{e:?}"))
                    .collect();
                assert!(errors.is_empty(), "{npc} at %haunted={stage}: {errors:?}");
                for branch in &branches {
                    assert!(
                        !branch.conversation.texts.is_empty(),
                        "{npc} at %haunted={stage}, path {:?}: nothing said",
                        branch.path
                    );
                }
            }
        }
    });
}
