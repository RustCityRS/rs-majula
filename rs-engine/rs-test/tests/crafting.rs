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
//!
//! A gold bar used on Lumbridge's furnace opens 2011's gold jewellery
//! interface (crafting_gold, 446) with each item the moulds and gems held can
//! make in its model (IF_SETOBJECT `ip4(com) p4_alt3(count) p2(obj)`) and its
//! button shown (IF_SETHIDE `p4_alt1(com) p1_alt1(hidden)`); a button's Make 1
//! (IF_BUTTON1 `p4(com) ip2_alt1(obj) ip2(slot)`) makes a sapphire ring for 40
//! xp. A silver bar opens 2011's silver casting (438): the Saradomin symbol's
//! row with its mould, the Zamorak symbol's "You need a ... mould" without;
//! Make All casts as many symbols as the bars make, 50 xp each. A ball of wool
//! on a gold amulet (IF_BUTTONT `p4(com) ip2(slot) ip2(slot) p2_alt1(obj)
//! ip4(com) p2(obj)`) strings it for 4 xp.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const CRAFTING: usize = 12;
const XP_COUNTER: u16 = 1801;
/// `inventory:inv`, the backpack component `[login]` binds `inv` to.
const BACKPACK: i32 = 149 << 16;
const NONE: i32 = 0xFFFF;
/// skillmulti2010's first item.
const FIRST_ITEM: i32 = 905 << 16 | 14;
/// Lumbridge's furnace, 2011's (loc 45310), and the tile it is used from.
const FURNACE: (u16, u16, u16) = (45310, 3226, 3256);
const AT_FURNACE: (u16, u8, u16) = (3226, 0, 3255);
const GOLD_IF: i32 = 446 << 16;
/// The sapphire ring's model and button, the emerald ring's button.
const SAPPHIRE_RING: i32 = GOLD_IF | 83;
const SAPPHIRE_RING_MAKE: i32 = GOLD_IF | 84;
const EMERALD_RING_MAKE: i32 = GOLD_IF | 86;
/// The ring row's "You need a ring mould" layer and its items' layer.
const RING_MOULD_TEXT: i32 = GOLD_IF | 17;
const RINGS: i32 = GOLD_IF | 18;
const SILVER_IF: i32 = 438 << 16;
/// The Saradomin symbol's row (its layer, model and button) and the Zamorak
/// symbol's two layers.
const HOLY_ROW: i32 = SILVER_IF | 15;
const HOLY_MODEL: i32 = SILVER_IF | 17;
const HOLY_MAKE: i32 = SILVER_IF | 16;
const HOLY_NEED: i32 = SILVER_IF | 19;
const UNHOLY_ROW: i32 = SILVER_IF | 22;
const UNHOLY_NEED: i32 = SILVER_IF | 26;

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

/// Each IF_SETOBJECT as `(com, obj)`.
fn objects(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16)> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::IfSetObject as u16)
        .map(|(_, p)| {
            (
                i32::from_le_bytes([p[0], p[1], p[2], p[3]]),
                u16::from_be_bytes([p[8], p[9]]),
            )
        })
        .collect()
}

/// Each IF_SETHIDE as `(com, hidden)`: `p4_alt1 com, p1_alt1 hidden`.
fn hides(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool)> {
    packets
        .iter()
        .filter(|(op, _)| *op == ServerProt::IfSetHide as u16)
        .map(|(_, p)| {
            let com = i32::from_be_bytes([p[1], p[0], p[3], p[2]]);
            (com, 0u8.wrapping_sub(p[4]) == 1)
        })
        .collect()
}

/// What the server sent, packet by packet, and said, over `n` ticks.
type Sent = (Vec<(u16, Vec<u8>)>, Vec<String>);

fn sent_over(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Sent {
    let mut sent = Vec::new();
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        let packets = client.packets();
        said.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
        sent.extend(packets);
    }
    (sent, said)
}

/// Op `op` (1-4: Make 1, 5, All and X) on the component `com` (IF_BUTTON1-4).
fn button(client: &mut HarnessClient, op: u8, com: i32) {
    let prot = match op {
        1 => ClientProt::IfButton1,
        2 => ClientProt::IfButton2,
        3 => ClientProt::IfButton3,
        _ => ClientProt::IfButton4,
    };
    let mut p = ClientPacket::new();
    p.p4(com).ip2_alt1(NONE).ip2(NONE);
    client.send(prot, &p.finish());
}

#[test]
fn a_gold_bar_on_lumbridge_s_furnace_makes_a_sapphire_ring() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let bar = obj(engine, "gold_bar");
        let mould = obj(engine, "ring_mould");
        let sapphire = obj(engine, "sapphire");
        let ring = obj(engine, "sapphire_ring");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let mut client = beside(engine, "goldsmith", AT_FURNACE, &[bar, mould, sapphire]);
        let player = engine.harness_player_mut(client.pid).unwrap();
        // Level 20, 4,470 xp (in tenths): 40 xp more is no level.
        player.stats.levels[CRAFTING] = 20;
        player.stats.base_levels[CRAFTING] = 20;
        player.stats.xp[CRAFTING] = 44700;
        let xp = player.stats.xp[CRAFTING];
        let counted = player.vars.get(XP_COUNTER).as_int();

        use_on(&mut client, bar, 0, FURNACE);
        let (sent, said) = sent_over(engine, &mut client, 6);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(446),
            "2011's gold interface; the game said {said:?}"
        );
        assert!(
            objects(&sent).contains(&(SAPPHIRE_RING, ring)),
            "the sapphire ring in its model; sent {:?}",
            objects(&sent)
        );
        let hidden = hides(&sent);
        assert!(
            hidden.contains(&(RING_MOULD_TEXT, true)),
            "a ring mould is held"
        );
        assert!(hidden.contains(&(RINGS, false)));
        assert!(
            hidden.contains(&(SAPPHIRE_RING_MAKE, false)),
            "a sapphire is held"
        );
        assert!(hidden.contains(&(EMERALD_RING_MAKE, true)), "no emerald");

        button(&mut client, 1, SAPPHIRE_RING_MAKE);
        let said = ticks(engine, &mut client, 6);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(ring), 1, "the game said {said:?}");
        assert_eq!(backpack.total(bar) + backpack.total(sapphire), 0);
        assert_eq!(backpack.total(mould), 1);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 400, "40 xp");
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 4000);
        assert_eq!(said, ["You make a sapphire ring."]);
        assert!(!player.is_interface_open(446), "the interface closes");
    });
}

#[test]
fn a_silver_bar_on_lumbridge_s_furnace_casts_holy_symbols() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let bar = obj(engine, "silver_bar");
        let mould = obj(engine, "holy_mould");
        let symbol = obj(engine, "unstrung_symbol");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let mut client = beside(engine, "silversmith", AT_FURNACE, &[bar, mould, bar]);
        let player = engine.harness_player_mut(client.pid).unwrap();
        // Level 16, 2,411 xp (in tenths): 100 xp more is no level.
        player.stats.levels[CRAFTING] = 16;
        player.stats.base_levels[CRAFTING] = 16;
        player.stats.xp[CRAFTING] = 24110;
        let xp = player.stats.xp[CRAFTING];

        use_on(&mut client, bar, 0, FURNACE);
        let (sent, said) = sent_over(engine, &mut client, 6);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(438),
            "2011's silver casting; the game said {said:?}"
        );
        assert!(
            objects(&sent).contains(&(HOLY_MODEL, symbol)),
            "the symbol in its row; sent {:?}",
            objects(&sent)
        );
        let hidden = hides(&sent);
        assert!(hidden.contains(&(HOLY_ROW, false)), "a holy mould is held");
        assert!(hidden.contains(&(HOLY_NEED, true)));
        assert!(hidden.contains(&(UNHOLY_ROW, true)), "no unholy mould");
        assert!(hidden.contains(&(UNHOLY_NEED, false)));

        button(&mut client, 3, HOLY_MAKE);
        let said = ticks(engine, &mut client, 10);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(symbol), 2, "the game said {said:?}");
        assert_eq!(backpack.total(bar), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 1000, "50 xp each");
        assert_eq!(
            said,
            [
                "You make a Holy Symbol of Saradomin",
                "You make a Holy Symbol of Saradomin"
            ],
            "no word of running out"
        );
    });
}

#[test]
fn a_ball_of_wool_strings_a_gold_amulet() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let wool = obj(engine, "ball_of_wool");
        let amulet = obj(engine, "gold_amulet");
        let strung = obj(engine, "strung_gold_amulet");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let mut client = beside(engine, "stringer", AT_FURNACE, &[wool, amulet]);
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[CRAFTING];

        use_on_obj(&mut client, (wool, 0), (amulet, 1));
        let said = ticks(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(strung), 1, "the game said {said:?}");
        assert_eq!(backpack.total(wool) + backpack.total(amulet), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 40, "4 xp");
        assert_eq!(said, ["You put some string on your amulet."]);
    });
}

/// Uses the obj `used` in its slot on the obj `target` in its (IF_BUTTONT).
fn use_on_obj(client: &mut HarnessClient, used: (u16, i32), target: (u16, i32)) {
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .ip2(target.1)
        .ip2(used.1)
        .p2_alt1(used.0 as i32)
        .ip4(BACKPACK)
        .p2(target.0 as i32);
    client.send(ClientProt::IfButtonT, &p.finish());
}

/// Logs a crafter in with `objs` at crafting level `level` (and its xp), and
/// returns the client and the crafting xp it starts with.
fn crafter(
    engine: &mut Engine,
    name: &str,
    objs: &[u16],
    level: usize,
    xp: i32,
) -> (HarnessClient, i32) {
    let client = beside(engine, name, AT_FURNACE, objs);
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.stats.levels[CRAFTING] = level as _;
    player.stats.base_levels[CRAFTING] = level as _;
    player.stats.xp[CRAFTING] = xp as _;
    (client, xp)
}

#[test]
fn a_needle_on_leather_asks_in_2011_s_make_x_and_makes_gloves() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let needle = obj(engine, "needle");
        let thread = obj(engine, "thread");
        let leather = obj(engine, "leather");
        let gloves = obj(engine, "leather_gloves");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let (mut client, xp) = crafter(
            engine,
            "leatherworker",
            &[needle, thread, leather, leather],
            1,
            0,
        );

        use_on_obj(&mut client, (needle, 0), (leather, 2));
        let said = ticks(engine, &mut client, 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(905),
            "2011's make-X asks which; the game said {said:?}"
        );
        // The second item, gloves (289's order: body, gloves, boots, ...).
        let mut p = ClientPacket::new();
        p.p2(NONE).p4_alt2(FIRST_ITEM + 1);
        client.send(ClientProt::ResumePauseButton, &p.finish());
        let said = ticks(engine, &mut client, 6);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(gloves), 1, "the game said {said:?}");
        assert_eq!(backpack.total(leather), 1, "one made");
        assert_eq!(player.stats.xp[CRAFTING] - xp, 138, "13.8 xp");
        assert_eq!(said, ["You make a pair of Leather Gloves."]);
    });
}

#[test]
fn a_chisel_cuts_a_sapphire() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let chisel = obj(engine, "chisel");
        let uncut = obj(engine, "uncut_sapphire");
        let sapphire = obj(engine, "sapphire");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        // Level 20, 4,470 xp: 50 xp more is no level.
        let (mut client, xp) = crafter(engine, "gemcutter", &[chisel, uncut], 20, 44700);

        use_on_obj(&mut client, (chisel, 0), (uncut, 1));
        let said = ticks(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(sapphire), 1, "the game said {said:?}");
        assert_eq!(backpack.total(uncut), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 500, "50 xp");
        assert_eq!(said, ["You cut the Sapphire."]);
    });
}

#[test]
fn studs_on_a_leather_body_make_a_studded_body() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let studs = obj(engine, "steel_studs");
        let body = obj(engine, "leather_body");
        let studded = obj(engine, "studded_body");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        // Level 41, 22,406 xp.
        let (mut client, xp) = crafter(engine, "studder", &[studs, body], 41, 224060);

        use_on_obj(&mut client, (studs, 0), (body, 1));
        let said = ticks(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(studded), 1, "the game said {said:?}");
        assert_eq!(backpack.total(studs) + backpack.total(body), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 400, "40 xp");
        assert_eq!(said, ["You make a studded body."]);
    });
}

#[test]
fn a_fire_orb_on_a_battlestaff_makes_a_fire_battlestaff() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let staff = obj(engine, "battlestaff");
        let orb = obj(engine, "fire_orb");
        let fire_staff = obj(engine, "fire_battlestaff");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        // Level 62, 333,804 xp.
        let (mut client, xp) = crafter(engine, "staffmaker", &[orb, staff], 62, 3338040);

        use_on_obj(&mut client, (orb, 0), (staff, 1));
        let said = ticks(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(fire_staff), 1, "the game said {said:?}");
        assert_eq!(backpack.total(staff) + backpack.total(orb), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 1250, "125 xp");
    });
}

#[test]
fn red_and_yellow_dye_make_orange_which_dyes_a_cape() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let red = obj(engine, "red_dye");
        let yellow = obj(engine, "yellow_dye");
        let orange = obj(engine, "orange_dye");
        let black_cape = obj(engine, "black_cape");
        let orange_cape = obj(engine, "orange_cape");
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let (mut client, xp) = crafter(engine, "dyer", &[red, yellow, black_cape], 1, 0);

        use_on_obj(&mut client, (red, 0), (yellow, 1));
        let mut said = ticks(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        let slot = (0..28)
            .find(|&s| backpack.get(s).is_some_and(|o| o.obj == orange))
            .unwrap_or_else(|| panic!("an orange dye; the game said {said:?}"));
        use_on_obj(&mut client, (orange, slot as i32), (black_cape, 2));
        said.extend(ticks(engine, &mut client, 3));
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(orange_cape), 1, "the game said {said:?}");
        assert_eq!(backpack.total(black_cape) + backpack.total(orange), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 25, "2.5 xp for the cape");
        assert_eq!(
            said,
            [
                "You mix the two dyes and make an orange one.",
                "You dye the black cape orange."
            ]
        );
    });
}
