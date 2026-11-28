//! Crafting from 289, at 2011's spinning wheels, furnaces and potter's wheels.
//!
//! An item used on a loc goes as the client sends it (OPLOCT `p2_alt2(obj)
//! p2_alt2(slot) p2(z) p4_alt2(com) p2_alt2(x) p1_alt1(ctrl) p2_alt1(loc)`).
//! Wool on Lumbridge castle's spinning wheel (loc 36970, upstairs) spins a ball
//! of wool for 2.5 xp; a bucket of sand with soda ash on Lumbridge's furnace
//! (loc 45310) makes molten glass for 20 xp; soft clay on the Crafting Guild's
//! potter's wheel (loc 2642) asks in 2011's make-X dialogue (skillmulti2010,
//! 905) and a pot (its first item, a pause button) is shaped for 6.3 xp.
//! 2011's xp counter (varp 1801, in tenths) counts each.
//!
//! A gold bar used on Lumbridge's furnace opens 2011's gold jewellery
//! interface (crafting_gold, 446) with each item the moulds and gems held can
//! make in its model (IF_SETOBJECT `p4_alt1(com) p4_alt3(count) p2(obj)`) and its
//! button shown (IF_SETHIDE `p4_alt3(com) p1_alt2(hidden)`); a button's Make 1
//! (IF_BUTTON1 `p4(com) p2_alt3(obj) p2_alt1(slot)`) makes a sapphire ring for 40
//! xp. A silver bar opens 2011's silver casting (438): the Saradomin symbol's
//! row with its mould, the Zamorak symbol's "You need a ... mould" without;
//! Make All casts as many symbols as the bars make, 50 xp each. A ball of wool
//! on a gold amulet (IF_BUTTONT `p4(com) p2_alt1(slot) p2_alt1(slot) p2_alt2(obj)
//! p4_alt1(com) p2(obj)`) strings it for 4 xp.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::names::BACKPACK;
#[cfg(since_647)]
use rs_test::names::com;
use rs_test::runner;
#[cfg(since_647)]
use rs_test::server;
use rs_ver::TypeId;
use std::sync::Mutex;

const CRAFTING: usize = 12;
#[cfg(since_647)]
const XP_COUNTER: u16 = 1801;
#[cfg(since_647)]
const NONE: i32 = 0xFFFF;
/// skillmulti2010's first item.
#[cfg(since_647)]
fn first_item() -> i32 {
    com("skillmulti2010:a")
}

/// Lumbridge's furnace, 2011's (loc 45310), and the tile it is used from.
#[cfg(since_647)]
const FURNACE: (u16, u16, u16) = (45310, 3226, 3256);
#[cfg(since_647)]
const AT_FURNACE: (u16, u8, u16) = (3226, 0, 3255);
/// Where the cases that use no loc start: beside the furnace, or on 530, whose furnace
/// stands elsewhere, Lumbridge's spawn.
#[cfg(all(since_530, before_647))]
const START: (u16, u8, u16) = (3222, 0, 3218);
#[cfg(since_647)]
const START: (u16, u8, u16) = AT_FURNACE;
/// The sapphire ring's model and button, the emerald ring's button.
#[cfg(since_647)]
fn sapphire_ring() -> i32 {
    com("crafting_gold:sapphire_dark1")
}

#[cfg(since_647)]
fn sapphire_ring_make() -> i32 {
    com("crafting_gold:sapphire_ring")
}

#[cfg(since_647)]
fn emerald_ring_make() -> i32 {
    com("crafting_gold:emerald_ring")
}

/// The ring row's "You need a ring mould" layer and its items' layer.
#[cfg(since_647)]
fn ring_mould_text() -> i32 {
    com("crafting_gold:com_17")
}

#[cfg(since_647)]
fn rings() -> i32 {
    com("crafting_gold:com_18")
}

/// The Saradomin symbol's row (its layer, model and button) and the Zamorak
/// symbol's two layers.
#[cfg(since_647)]
fn holy_row() -> i32 {
    com("crafting_silver_casting:com_15")
}

#[cfg(since_647)]
fn holy_model() -> i32 {
    com("crafting_silver_casting:com_17")
}

#[cfg(since_647)]
fn holy_make() -> i32 {
    com("crafting_silver_casting:com_16")
}

#[cfg(since_647)]
fn holy_need() -> i32 {
    com("crafting_silver_casting:com_19")
}

#[cfg(since_647)]
fn unholy_row() -> i32 {
    com("crafting_silver_casting:com_22")
}

#[cfg(since_647)]
fn unholy_need() -> i32 {
    com("crafting_silver_casting:com_26")
}

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: Mutex<()> = Mutex::new(());

fn obj(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
        .id
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
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
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
        backpack.set(slot as u16, TypeId::new(o), 1);
    }
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    client
}

/// Uses the obj in `slot` on the loc `loc` at `(x, z)` (OPLOCT).
fn use_on(client: &mut HarnessClient, o: u16, slot: i32, (loc, x, z): (u16, u16, u16)) {
    client.op_loc_t(
        x,
        z,
        TypeId::from_int(loc as i32),
        BACKPACK,
        slot as u16,
        TypeId::from_int(o as i32),
    );
}

#[test]
fn wool_on_lumbridge_castle_s_wheel_spins_a_ball_of_wool() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let wool = obj(engine, "wool");
        let ball = obj(engine, "ball_of_wool");
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let mut client = beside(engine, "spinner", (3209, 1, 3213), &[wool.g2()]);
        let player = engine.harness_player(client.pid).unwrap();
        let xp = player.stats.xp[CRAFTING];
        #[cfg(since_647)]
        let counted = player.vars.get(XP_COUNTER).as_int();

        use_on(&mut client, wool.g2(), 0, (36970, 3209, 3212));
        let said = ticks(engine, &mut client, 8);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(ball), 1, "the game said {said:?}");
        assert_eq!(backpack.total(wool), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 25, "2.5 xp");
        #[cfg(since_647)]
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 250);
        assert_eq!(said, ["You spin the wool into a nice ball of wool."]);
    });
}

#[cfg(since_647)]
#[test]
fn sand_and_soda_ash_on_lumbridge_s_furnace_make_molten_glass() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let sand = obj(engine, "bucket_sand");
        let ash = obj(engine, "soda_ash");
        let glass = obj(engine, "molten_glass");
        let bucket = obj(engine, "bucket_empty");
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let mut client = beside(
            engine,
            "glassmaker",
            (3226, 0, 3255),
            &[sand.g2(), ash.g2()],
        );
        let player = engine.harness_player(client.pid).unwrap();
        let xp = player.stats.xp[CRAFTING];
        #[cfg(since_647)]
        let counted = player.vars.get(XP_COUNTER).as_int();

        use_on(&mut client, sand.g2(), 0, (45310, 3226, 3256));
        let said = ticks(engine, &mut client, 8);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(glass), 1, "the game said {said:?}");
        assert_eq!(backpack.total(bucket), 1);
        assert_eq!(backpack.total(sand) + backpack.total(ash), 0);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 200, "20 xp");
        #[cfg(since_647)]
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 2000);
        assert_eq!(
            said,
            ["You heat the sand and soda ash in the furnace to make glass."]
        );
    });
}

#[cfg(since_647)]
#[test]
fn soft_clay_on_a_potter_s_wheel_shapes_a_pot() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let clay = obj(engine, "softclay");
        let pot = obj(engine, "pot_unfired");
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let wheel = (2642, 2935, 3289);
        let mut client = beside(engine, "potter", (2935, 0, 3288), &[clay.g2()]);
        let player = engine.harness_player(client.pid).unwrap();
        let xp = player.stats.xp[CRAFTING];

        use_on(&mut client, clay.g2(), 0, wheel);
        let said = ticks(engine, &mut client, 6);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(905),
            "2011's make-X asks which; the game said {said:?}"
        );
        client.resume_pausebutton(first_item(), NONE as u16);
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
#[cfg(since_647)]
fn objects(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16)> {
    server::objects(packets)
        .into_iter()
        .map(|(c, o)| (c, o as u16))
        .collect()
}

/// Each IF_SETHIDE as `(com, hidden)`: `p4_alt3 com, p1_alt2 hidden`.
#[cfg(since_647)]
fn hides(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool)> {
    server::hides(packets)
}

/// What the server sent, packet by packet, and said, over `n` ticks.
#[cfg(since_647)]
type Sent = (Vec<(u16, Vec<u8>)>, Vec<String>);

#[cfg(since_647)]
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
#[cfg(since_647)]
fn button(client: &mut HarnessClient, op: u8, com: i32) {
    client.if_button(op.min(4), com, NONE as u16, TypeId::NULL);
}

#[cfg(since_647)]
#[test]
fn a_gold_bar_on_lumbridge_s_furnace_makes_a_sapphire_ring() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let bar = obj(engine, "gold_bar");
        let mould = obj(engine, "ring_mould");
        let sapphire = obj(engine, "sapphire");
        let ring = obj(engine, "sapphire_ring");
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let mut client = beside(
            engine,
            "goldsmith",
            AT_FURNACE,
            &[bar.g2(), mould.g2(), sapphire.g2()],
        );
        let player = engine.harness_player_mut(client.pid).unwrap();
        // Level 20, 4,470 xp (in tenths): 40 xp more is no level.
        player.stats.levels[CRAFTING] = 20;
        player.stats.base_levels[CRAFTING] = 20;
        player.stats.xp[CRAFTING] = 44700;
        let xp = player.stats.xp[CRAFTING];
        #[cfg(since_647)]
        let counted = player.vars.get(XP_COUNTER).as_int();

        use_on(&mut client, bar.g2(), 0, FURNACE);
        let (sent, said) = sent_over(engine, &mut client, 6);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(446),
            "2011's gold interface; the game said {said:?}"
        );
        assert!(
            objects(&sent).contains(&(sapphire_ring(), ring.g2())),
            "the sapphire ring in its model; sent {:?}",
            objects(&sent)
        );
        let hidden = hides(&sent);
        assert!(
            hidden.contains(&(ring_mould_text(), true)),
            "a ring mould is held"
        );
        assert!(hidden.contains(&(rings(), false)));
        assert!(
            hidden.contains(&(sapphire_ring_make(), false)),
            "a sapphire is held"
        );
        assert!(hidden.contains(&(emerald_ring_make(), true)), "no emerald");

        button(&mut client, 1, sapphire_ring_make());
        let said = ticks(engine, &mut client, 6);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(ring), 1, "the game said {said:?}");
        assert_eq!(backpack.total(bar) + backpack.total(sapphire), 0);
        assert_eq!(backpack.total(mould), 1);
        assert_eq!(player.stats.xp[CRAFTING] - xp, 400, "40 xp");
        #[cfg(since_647)]
        assert_eq!(player.vars.get(XP_COUNTER).as_int() - counted, 4000);
        assert_eq!(said, ["You make a sapphire ring."]);
        assert!(!player.is_interface_open(446), "the interface closes");
    });
}

#[cfg(since_647)]
#[test]
fn a_silver_bar_on_lumbridge_s_furnace_casts_holy_symbols() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let bar = obj(engine, "silver_bar");
        let mould = obj(engine, "holy_symbol_mould");
        let symbol = obj(engine, "nostringstar");
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let mut client = beside(
            engine,
            "silversmith",
            AT_FURNACE,
            &[bar.g2(), mould.g2(), bar.g2()],
        );
        let player = engine.harness_player_mut(client.pid).unwrap();
        // Level 16, 2,411 xp (in tenths): 100 xp more is no level.
        player.stats.levels[CRAFTING] = 16;
        player.stats.base_levels[CRAFTING] = 16;
        player.stats.xp[CRAFTING] = 24110;
        let xp = player.stats.xp[CRAFTING];

        use_on(&mut client, bar.g2(), 0, FURNACE);
        let (sent, said) = sent_over(engine, &mut client, 6);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(438),
            "2011's silver casting; the game said {said:?}"
        );
        assert!(
            objects(&sent).contains(&(holy_model(), symbol.g2())),
            "the symbol in its row; sent {:?}",
            objects(&sent)
        );
        let hidden = hides(&sent);
        assert!(
            hidden.contains(&(holy_row(), false)),
            "a holy mould is held"
        );
        assert!(hidden.contains(&(holy_need(), true)));
        assert!(hidden.contains(&(unholy_row(), true)), "no unholy mould");
        assert!(hidden.contains(&(unholy_need(), false)));

        button(&mut client, 3, holy_make());
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
        let amulet = obj(engine, "unstrung_gold_amulet");
        let strung = obj(engine, "strung_gold_amulet");
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let mut client = beside(engine, "stringer", START, &[wool.g2(), amulet.g2()]);
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[CRAFTING];

        use_on_obj(&mut client, (wool.g2(), 0), (amulet.g2(), 1));
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
    client.if_button_t(
        BACKPACK,
        used.1 as u16,
        TypeId::from_int(used.0 as i32),
        BACKPACK,
        target.1 as u16,
        TypeId::from_int(target.0 as i32),
    );
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
    let client = beside(engine, name, START, objs);
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.stats.levels[CRAFTING] = level as _;
    player.stats.base_levels[CRAFTING] = level as _;
    player.stats.xp[CRAFTING] = xp as _;
    (client, xp)
}

#[cfg(since_647)]
#[test]
fn a_needle_on_leather_asks_in_2011_s_make_x_and_makes_gloves() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let needle = obj(engine, "needle");
        let thread = obj(engine, "thread");
        let leather = obj(engine, "leather");
        let gloves = obj(engine, "leather_gloves");
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let (mut client, xp) = crafter(
            engine,
            "leatherworker",
            &[needle.g2(), thread.g2(), leather.g2(), leather.g2()],
            1,
            0,
        );

        use_on_obj(&mut client, (needle.g2(), 0), (leather.g2(), 2));
        let said = ticks(engine, &mut client, 3);
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(905),
            "2011's make-X asks which; the game said {said:?}"
        );
        // The second item, gloves (289's order: body, gloves, boots, ...).
        client.resume_pausebutton(com("skillmulti2010:b"), NONE as u16);
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
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        // Level 20, 4,470 xp: 50 xp more is no level.
        let (mut client, xp) = crafter(engine, "gemcutter", &[chisel.g2(), uncut.g2()], 20, 44700);

        use_on_obj(&mut client, (chisel.g2(), 0), (uncut.g2(), 1));
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
        let studs = obj(engine, "studs");
        let body = obj(engine, "leather_armour");
        let studded = obj(engine, "studded_body");
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        // Level 41, 22,406 xp.
        let (mut client, xp) = crafter(engine, "studder", &[studs.g2(), body.g2()], 41, 224060);

        use_on_obj(&mut client, (studs.g2(), 0), (body.g2(), 1));
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
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        // Level 62, 333,804 xp.
        let (mut client, xp) = crafter(engine, "staffmaker", &[orb.g2(), staff.g2()], 62, 3338040);

        use_on_obj(&mut client, (orb.g2(), 0), (staff.g2(), 1));
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
        let red = obj(engine, "reddye");
        let yellow = obj(engine, "yellowdye");
        let orange = obj(engine, "orangedye");
        let black_cape = obj(engine, "black_cape");
        let orange_cape = obj(engine, "orange_cape");
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let (mut client, xp) = crafter(
            engine,
            "dyer",
            &[red.g2(), yellow.g2(), black_cape.g2()],
            1,
            0,
        );

        use_on_obj(&mut client, (red.g2(), 0), (yellow.g2(), 1));
        let mut said = ticks(engine, &mut client, 3);
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        let slot = (0..28)
            .find(|&s| backpack.get(s).is_some_and(|o| o.obj == orange))
            .unwrap_or_else(|| panic!("an orange dye; the game said {said:?}"));
        use_on_obj(
            &mut client,
            (orange.g2(), slot as i32),
            (black_cape.g2(), 2),
        );
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
