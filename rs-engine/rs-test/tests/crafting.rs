//! Crafting from 289, at 2011's spinning wheels, furnaces and potter's wheels.
//!
//! An item used on a loc goes as the client sends it (OPLOCT `p2_alt1(obj)
//! p2_alt1(slot) p2(z) p4_alt2(com) p2_alt1(x) p1_alt3(ctrl) ip2(loc)`).
//! Wool on Lumbridge castle's spinning wheel (loc 36970, upstairs) spins a ball
//! of wool for 2.5 xp; a bucket of sand with soda ash on Lumbridge's furnace
//! (loc 45310) makes molten glass for 20 xp; soft clay on the Crafting Guild's
//! potter's wheel (loc 2642) asks in 2011's make-X dialogue (skillmulti2010,
//! 905) and a pot (its first item, a pause button) is shaped for 6.3 xp.
//! 2011's xp counter (varp 1801, in tenths) counts each.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const CRAFTING: usize = 12;
const XP_COUNTER: u16 = 1801;
/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;
const NONE: i32 = 0xFFFF;
/// skillmulti2010's first item.
const FIRST_ITEM: i32 = 905 << 16 | 14;

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

/// Logs a crafter in at `(x, level, z)`, a walkable tile the loc is used from,
/// with `objs` in the backpack's first slots.
fn beside(
    engine: &mut Engine,
    name: &str,
    (x, level, z): (u16, u8, u16),
    objs: &[u16],
) -> HarnessClient {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    assert!(
        !rsmod::is_flagged(x, z, level, 0x100 | 0x200000),
        "{x},{z} is walkable"
    );
    engine.harness_telejump(client.pid, x, level, z);
    let player = engine.harness_player_mut(client.pid).unwrap();
    let backpack = player.invs.get_mut(&inv).unwrap();
    for (slot, &o) in objs.iter().enumerate() {
        backpack.set(slot as u16, o, 1);
    }
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    client
}

/// Uses the obj in `slot` on the loc `loc` at `(x, z)` (OPLOCT).
fn use_on(client: &mut HarnessClient, o: u16, slot: i32, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt1(o as i32)
        .p2_alt1(slot)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt1(x as i32)
        .p1_alt3(0)
        .ip2(loc as i32);
    client.send(ClientProt::OpLocT, &p.finish());
}

#[test]
fn wool_on_lumbridge_castle_s_wheel_spins_a_ball_of_wool() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let wool = obj(engine, "wool");
        let ball = obj(engine, "ball_of_wool");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let mut client = beside(engine, "spinner", (3209, 1, 3213), &[wool]);
        let player = engine.harness_player(client.pid).unwrap();
        let xp = player.stats.xp[CRAFTING];
        let counted = player.vars.get(XP_COUNTER).as_int();

        use_on(&mut client, wool, 0, (36970, 3209, 3212));
        let said = ticks(engine, &mut client, 8);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(ball), 1, "the game said {said:?}");
        assert_eq!(backpack.total(wool), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 25, "2.5 xp");
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 250);
        assert_eq!(said, ["You spin the wool into a nice ball of wool."]);
    });
}

#[test]
fn sand_and_soda_ash_on_lumbridge_s_furnace_make_molten_glass() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let sand = obj(engine, "bucket_of_sand");
        let ash = obj(engine, "soda_ash");
        let glass = obj(engine, "molten_glass");
        let bucket = obj(engine, "bucket");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let mut client = beside(engine, "glassmaker", (3226, 0, 3255), &[sand, ash]);
        let player = engine.harness_player(client.pid).unwrap();
        let xp = player.stats.xp[CRAFTING];
        let counted = player.vars.get(XP_COUNTER).as_int();

        use_on(&mut client, sand, 0, (45310, 3226, 3256));
        let said = ticks(engine, &mut client, 8);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(glass), 1, "the game said {said:?}");
        assert_eq!(backpack.total(bucket), 1);
        assert_eq!(backpack.total(sand) + backpack.total(ash), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 200, "20 xp");
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 2000);
        assert_eq!(
            said,
            ["You heat the sand and soda ash in the furnace to make glass."]
        );
    });
}

#[test]
fn soft_clay_on_a_potter_s_wheel_shapes_a_pot() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let clay = obj(engine, "soft_clay");
        let pot = obj(engine, "pot__unfired");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let wheel = (2642, 2935, 3289);
        let mut client = beside(engine, "potter", (2935, 0, 3288), &[clay]);
        let player = engine.harness_player(client.pid).unwrap();
        let xp = player.stats.xp[CRAFTING];

        use_on(&mut client, clay, 0, wheel);
        let said = ticks(engine, &mut client, 6);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(905),
            "2011's make-X asks which; the game said {said:?}"
        );
        let mut p = ClientPacket::new();
        p.p2(NONE).p4_alt2(FIRST_ITEM);
        client.send(ClientProt::ResumePauseButton, &p.finish());
        let said = ticks(engine, &mut client, 8);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(pot), 1, "the game said {said:?}");
        assert_eq!(backpack.total(clay), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 63, "6.3 xp");
        assert_eq!(said, ["You make the clay into a pot."]);
    });
}
