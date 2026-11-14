//! The Elemental Workshop at 647 (`quests/quest_elemental_workshop`): 289's quest on 2011's
//! workshop. Cache 786 makes the wheel, the bellows, the valves and the furnace multilocs on the
//! quest's varbits, the elemental rocks npcs, and the cut book a second obj (the Slashed book,
//! as the 2011 wiki has it). The rewards are the 2011 wiki's and Sal's Realm's as 289's.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{Conversation, settexts, talk_to};
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const BACKPACK: i32 = 149 << 16;
const CRAFTING: usize = 12;
const SMITHING: usize = 13;
/// The life points' varp (bit 0 is another's).
const LIFE: u16 = 1240;

/// `%elemental_workshop_bits`: `^elemental_workshop_read_book` .. `^elemental_workshop_complete`
/// (cache 786's varbits on varp 299 are these same bits).
const READ_BOOK: i32 = 1 << 1;
const SLASHED_BOOK: i32 = 1 << 2;
const WATER_LEFT: i32 = 1 << 3;
const WATER_RIGHT: i32 = 1 << 4;
const WATER_FLOWING: i32 = 1 << 5;
const BELLOWS_REPAIRED: i32 = 1 << 6;
const FURNACE_LIT: i32 = 1 << 7;
const AIR_BLOWING: i32 = 1 << 9;
const ENTERED: i32 = 1 << 13;
const COMPLETE: i32 = 1 << 20;

/// Cache 786's map: Seers' Village above, the workshop below.
const BOOKCASE: (u16, u16) = (2716, 3481);
const ODD_WALL: (u16, u16) = (2709, 3495);
const STAIRS_TOP: (u16, u16) = (2710, 3497);
const WORKSHOP: (u16, u16) = (2716, 9888);
const BOWL_CRATE: (u16, u16) = (2717, 9894);
const NEEDLE_CRATE: (u16, u16) = (2700, 9887);
const LEATHER_CRATE: (u16, u16) = (2724, 9894);
const RIGHT_VALVE: (u16, u16) = (2726, 9908);
const LEFT_VALVE: (u16, u16) = (2713, 9908);
const WATER_LEVER: (u16, u16) = (2722, 9906);
const BELLOWS: (u16, u16) = (2734, 9882);
const AIR_LEVER: (u16, u16) = (2734, 9887);
const FURNACE: (u16, u16) = (2725, 9874);
const TROUGH: (u16, u16) = (2718, 9871);
const WORKBENCH: (u16, u16) = (2717, 9887);

fn var(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let v = id(engine, "varp", name);
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(v)
        .as_int()
}

fn set_var(engine: &mut Engine, client: &HarnessClient, name: &str, value: i32) {
    let v = id(engine, "varp", name);
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .vars
        .set(v, VarValue::Int(value));
}

fn login(engine: &mut Engine, name: &str, stage: i32) -> HarnessClient {
    let _ = engine.script_errors.take();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    settle(engine, &mut client, 10);
    set_var(engine, &client, "elemental_workshop_bits", stage);
    client
}

fn no_errors(engine: &mut Engine) {
    let errors = engine.script_errors.take();
    assert!(errors.is_empty(), "no script errors: {errors:?}");
}

fn walkable(x: u16, level: u8, z: u16) -> bool {
    !rsmod::is_flagged(x, z, level, 0x100 | 0x200000)
}

fn tele(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16), level: u8) {
    assert!(
        walkable(x, level, z),
        "({x}, {z}) on level {level} is walkable"
    );
    engine.harness_telejump(client.pid, x, level, z);
    settle(engine, client, 3);
}

/// Every spawned npc of that name: its nid, tile and level.
fn npcs(engine: &Engine, name: &str) -> Vec<(u16, u16, u16, u8)> {
    let want = id(engine, "npc", name);
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == want)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.z(), c.y())
        })
        .collect()
}

fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
    let map = match kind {
        "obj" => &engine.cache.objs.debugnames,
        "npc" => &engine.cache.npcs.debugnames,
        "loc" => &engine.cache.locs.debugnames,
        "varp" => &engine.cache.varps.debugnames,
        _ => unreachable!(),
    };
    *map.get(name).unwrap_or_else(|| panic!("no {kind} {name}"))
}

fn give(engine: &mut Engine, client: &HarnessClient, slot: u16, name: &str) {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap()
        .set(slot, o, 1);
}

fn held(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(o) as i32
}

fn settle(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) {
    for _ in 0..ticks {
        engine.cycle();
    }
    client.packets();
}

fn tells(t: &Conversation, line: &str) {
    assert!(
        t.messages.iter().any(|m| m == line),
        "289 has {line:?}; chat {:?} (texts {:?})",
        t.messages,
        t.texts
    );
}

fn coord(engine: &Engine, client: &HarnessClient) -> (u16, u16) {
    let c = engine.harness_player(client.pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

fn slot_of(engine: &Engine, client: &HarnessClient, name: &str) -> i32 {
    let o = id(engine, "obj", name);
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let player = engine.harness_player(client.pid).unwrap();
    let inv = player.invs.get(&inv).unwrap();
    (0..28)
        .find(|&s| inv.has_at(s, o))
        .unwrap_or_else(|| panic!("{name} is held")) as i32
}

/// Stands the player on a walkable tile within two of a loc's tile, nearest first.
fn stand_by(engine: &mut Engine, client: &mut HarnessClient, (x, z): (u16, u16), level: u8) {
    let mut tiles: Vec<(i16, i16)> = (-2i16..=2)
        .flat_map(|dx| (-2i16..=2).map(move |dz| (dx, dz)))
        .filter(|&(dx, dz)| (dx, dz) != (0, 0))
        .collect();
    tiles.sort_by_key(|&(dx, dz)| dx.abs() + dz.abs());
    let at = tiles
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(ax, az)| walkable(ax, level, az))
        .unwrap_or_else(|| panic!("a tile by ({x}, {z})"));
    tele(engine, client, at, level);
}

/// What the chat is told over `ticks` (an op that opens no box).
fn watch(engine: &mut Engine, client: &mut HarnessClient, ticks: usize) -> Conversation {
    let mut t = Conversation::default();
    for _ in 0..ticks {
        engine.cycle();
        let packets = client.packets();
        t.texts
            .extend(settexts(&packets).into_iter().map(|(_, text)| text));
        t.messages.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, m)| m),
        );
    }
    t
}

/// `OPLOC1` on a loc, and what the chat is told over `ticks`.
fn search(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    (x, z): (u16, u16),
    ticks: usize,
) -> Conversation {
    let loc = id(engine, "loc", loc);
    let mut p = ClientPacket::new();
    p.p2_alt1(z as i32)
        .p2_alt1(loc as i32)
        .p1(0)
        .p2_alt2(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
    watch(engine, client, ticks)
}

/// Uses the backpack's `obj` on a loc (`OPLOCT`: `p2_alt2(obj) p2_alt2(slot) p2(z)
/// p4_alt2(component) p2_alt2(x) p1_alt1(ctrl) p2_alt1(loc)`), and what the chat is told over
/// `ticks`.
fn use_on(
    engine: &mut Engine,
    client: &mut HarnessClient,
    obj: &str,
    loc: &str,
    (x, z): (u16, u16),
    ticks: usize,
) -> Conversation {
    let slot = slot_of(engine, client, obj);
    let (obj, loc) = (id(engine, "obj", obj), id(engine, "loc", loc));
    let mut p = ClientPacket::new();
    p.p2_alt2(obj as i32)
        .p2_alt2(slot)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(loc as i32);
    client.send(ClientProt::OpLocT, &p.finish());
    watch(engine, client, ticks)
}

/// Level 99 in every skill (its xp, in tenths) and full life: no level-up box follows a reward,
/// and a fall is survived.
fn seasoned(engine: &mut Engine, client: &HarnessClient) {
    let player = engine.harness_player_mut(client.pid).unwrap();
    for skill in 0..player.stats.levels.len() {
        player.stats.levels[skill] = 99;
        player.stats.base_levels[skill] = 99;
        player.stats.xp[skill] = 130_344_310;
    }
    let bit0 = player.vars.get(LIFE).as_int() & 1;
    player.vars.set(LIFE, VarValue::Int((990 << 1) | bit0));
}

fn xp(engine: &Engine, client: &HarnessClient, skill: usize) -> i32 {
    engine.harness_player(client.pid).unwrap().stats.xp[skill]
}

/// Stands the player by a loc and searches or operates it.
fn at(
    engine: &mut Engine,
    client: &mut HarnessClient,
    loc: &str,
    tile: (u16, u16),
    ticks: usize,
) -> Conversation {
    stand_by(engine, client, tile, 0);
    search(engine, client, loc, tile, ticks)
}

/// Uses the backpack obj `used` on the backpack obj `target` (`IF_BUTTONT`).
fn use_on_obj(
    engine: &mut Engine,
    client: &mut HarnessClient,
    used: &str,
    target: &str,
) -> Conversation {
    let (us, ts) = (
        slot_of(engine, client, used),
        slot_of(engine, client, target),
    );
    let (uo, to) = (id(engine, "obj", used), id(engine, "obj", target));
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .p2_alt1(ts)
        .p2_alt1(us)
        .p2_alt2(uo as i32)
        .p4_alt1(BACKPACK)
        .p2(to as i32);
    client.send(ClientProt::IfButtonT, &p.finish());
    watch(engine, client, 3)
}

fn bits(engine: &Engine, client: &HarnessClient) -> i32 {
    var(engine, client, "elemental_workshop_bits")
}

/// The bookcase in Seers' Village has the Battered book; read, a blade cuts a key from its spine
/// and leaves the Slashed book (2011); the key opens the odd wall by the anvils, and the stairs
/// lead down.
#[test]
fn the_battered_book_hides_a_key() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "ew_book", 0);
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_bookcase",
            BOOKCASE,
            5,
        );
        tells(&t, "You find a book titled 'The elemental shield'.");
        assert_eq!(held(engine, &client, "elemental_workshop_shield_book"), 1);

        // Unread, the book is not to be damaged.
        give(engine, &client, 5, "knife");
        let t = use_on_obj(
            engine,
            &mut client,
            "knife",
            "elemental_workshop_shield_book",
        );
        tells(&t, "You don't want to damage the book.");

        let slot = slot_of(engine, &client, "elemental_workshop_shield_book");
        let book = id(engine, "obj", "elemental_workshop_shield_book");
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).p2_alt3(book as i32).p2_alt1(slot);
        client.send(ClientProt::IfButton1, &p.finish());
        let t = watch(engine, &mut client, 4);
        tells(
            &t,
            "The book has two parts an introduction and an instruction section.",
        );
        assert_eq!(bits(engine, &client), READ_BOOK);
        engine.harness_close_modal(client.pid);
        settle(engine, &mut client, 2);

        let t = use_on_obj(
            engine,
            &mut client,
            "knife",
            "elemental_workshop_shield_book",
        );
        tells(&t, "Inside you find a small old battered key.");
        assert_eq!(held(engine, &client, "elemental_workshop_key"), 1);
        assert_eq!(held(engine, &client, "elemental_workshop_shield_book"), 0);
        assert_eq!(
            held(engine, &client, "elemental_workshop_shield_book_slashed"),
            1,
            "2011's Slashed book"
        );
        assert_eq!(bits(engine, &client), READ_BOOK | SLASHED_BOOK);

        // With either book in hand the bookcase has nothing; without, it gives the slashed one.
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_bookcase",
            BOOKCASE,
            5,
        );
        tells(&t, "You search the bookcase but find nothing of interest.");
        let slot = slot_of(engine, &client, "elemental_workshop_shield_book_slashed") as u16;
        give(engine, &client, slot, "bread");
        at(
            engine,
            &mut client,
            "elemental_workshop_bookcase",
            BOOKCASE,
            5,
        );
        assert_eq!(
            held(engine, &client, "elemental_workshop_shield_book_slashed"),
            1
        );
        assert_eq!(held(engine, &client, "elemental_workshop_shield_book"), 0);

        // The odd wall: a hole and no way in, until the key.
        tele(engine, &mut client, (ODD_WALL.0, ODD_WALL.1), 0);
        let t = search(
            engine,
            &mut client,
            "elemental_workshop_oddwall_l",
            ODD_WALL,
            4,
        );
        tells(
            &t,
            "You see a small hole in the wall but no way to open it.",
        );
        let t = use_on(
            engine,
            &mut client,
            "bread",
            "elemental_workshop_oddwall_l",
            ODD_WALL,
            4,
        );
        tells(&t, "You can't unlock the door with that.");
        use_on(
            engine,
            &mut client,
            "elemental_workshop_key",
            "elemental_workshop_oddwall_l",
            ODD_WALL,
            6,
        );
        assert_eq!(
            coord(engine, &client),
            (ODD_WALL.0, ODD_WALL.1 + 1),
            "through the odd wall"
        );

        search(
            engine,
            &mut client,
            "elemental_workshop_spiralstairstop",
            STAIRS_TOP,
            6,
        );
        assert_eq!(coord(engine, &client), WORKSHOP);
        assert_eq!(bits(engine, &client), READ_BOOK | SLASHED_BOOK | ENTERED);
        no_errors(engine);
    });
}

/// The workshop is set going: the two valves and the lever turn the wheel, the bellows are
/// stitched and pumped, and a bowl of lava lights the furnace. Each sets the bit cache 786's
/// multilocs show it by.
#[test]
fn the_workshop_is_set_going() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let start = READ_BOOK | SLASHED_BOOK | ENTERED;
        let mut client = login(engine, "ew_machines", start);
        seasoned(engine, &client);

        // The air lever does nothing yet, nor the water lever with the valves shut.
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_air_lever",
            AIR_LEVER,
            5,
        );
        tells(&t, "Nothing happens; the lever resets itself.");
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_water_lever",
            WATER_LEVER,
            5,
        );
        tells(&t, "You hear the sound of the flow gates resetting.");
        settle(engine, &mut client, 4);

        let t = at(
            engine,
            &mut client,
            "elemental_workshop_valve_1",
            RIGHT_VALVE,
            5,
        );
        tells(&t, "You turn the handle.");
        assert_eq!(bits(engine, &client), start | WATER_RIGHT);
        at(
            engine,
            &mut client,
            "elemental_workshop_valve_2",
            LEFT_VALVE,
            5,
        );
        assert_eq!(bits(engine, &client), start | WATER_RIGHT | WATER_LEFT);
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_water_lever",
            WATER_LEVER,
            5,
        );
        tells(&t, "You hear the sound of a water wheel starting up.");
        let water = start | WATER_RIGHT | WATER_LEFT | WATER_FLOWING;
        assert_eq!(bits(engine, &client), water, "the wheel's varbit");
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_valve_1",
            RIGHT_VALVE,
            5,
        );
        tells(
            &t,
            "Now that the water wheel is running, the valve seems locked off.",
        );

        // The crates: a needle, leather and the stone bowl.
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_box_2",
            NEEDLE_CRATE,
            5,
        );
        tells(&t, "You find a needle.");
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_box_4",
            LEATHER_CRATE,
            5,
        );
        tells(&t, "You find some leather.");
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_box_4",
            LEATHER_CRATE,
            5,
        );
        tells(&t, "It's empty.");
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_box_1",
            BOWL_CRATE,
            5,
        );
        tells(&t, "You find a stone bowl.");

        // The bellows want thread too.
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_bellows_multiloc",
            BELLOWS,
            6,
        );
        tells(
            &t,
            "You need a piece of leather, some thread and a needle to fix this.",
        );
        give(engine, &client, 10, "thread");
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_bellows_multiloc",
            BELLOWS,
            6,
        );
        tells(&t, "You stitch the leather over the hole in the bellows.");
        assert_eq!(held(engine, &client, "leather"), 0);
        assert_eq!(held(engine, &client, "needle"), 1, "the needle is kept");
        let t = at(
            engine,
            &mut client,
            "elemental_workshop_air_lever",
            AIR_LEVER,
            5,
        );
        tells(&t, "The bellows pump air down the pipe.");
        let air = water | 1 << 14 | 1 << 15 | BELLOWS_REPAIRED | AIR_BLOWING;
        assert_eq!(bits(engine, &client), air, "the bellows' varbit");

        // Lava from the trough lights the furnace.
        stand_by(engine, &mut client, TROUGH, 0);
        let t = use_on(
            engine,
            &mut client,
            "elemental_workshop_lava_bowl",
            "elemental_workshop_trough_1",
            TROUGH,
            5,
        );
        tells(&t, "You fill the bowl with hot lava.");
        assert_eq!(
            held(engine, &client, "elemental_workshop_lava_bowl_full"),
            1
        );
        stand_by(engine, &mut client, FURNACE, 0);
        let t = use_on(
            engine,
            &mut client,
            "elemental_workshop_lava_bowl_full",
            "elemental_workshop_furnace",
            FURNACE,
            10,
        );
        tells(&t, "The furnace bursts in to life.");
        assert_eq!(
            bits(engine, &client),
            air | FURNACE_LIT,
            "the furnace's varbit"
        );
        assert_eq!(held(engine, &client, "elemental_workshop_lava_bowl"), 1);
        no_errors(engine);
    });
}

/// An elemental rock (an npc in 2011) springs to life when mined; its ore and four coal make a
/// bar in the lit furnace, and the bar a shield on the workbench with the book at hand: 1 quest
/// point, 5,000 Crafting and 5,000 Smithing experience (2011 wiki and Sal's Realm, as 289).
#[test]
fn the_elemental_shield_is_made() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let going = READ_BOOK
            | SLASHED_BOOK
            | ENTERED
            | WATER_LEFT
            | WATER_RIGHT
            | WATER_FLOWING
            | BELLOWS_REPAIRED
            | AIR_BLOWING
            | FURNACE_LIT;
        let mut client = login(engine, "ew_shield", going);
        seasoned(engine, &client);
        give(engine, &client, 0, "bronze_pickaxe");
        give(engine, &client, 1, "hammer");
        give(engine, &client, 2, "elemental_workshop_shield_book_slashed");
        for slot in 3..7 {
            give(engine, &client, slot, "coal");
        }

        // Mine a rock: the elemental rises beside the player.
        let (rock, rx, rz, _) = npcs(engine, "elem1_qip_earth_elemental_rock_version_rock")
            .into_iter()
            .find(|&(_, x, z, _)| {
                [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
                    .into_iter()
                    .any(|(dx, dz)| {
                        walkable(x.wrapping_add_signed(dx), 0, z.wrapping_add_signed(dz))
                    })
            })
            .expect("an elemental rock with a tile beside it");
        stand_by(engine, &mut client, (rx, rz), 0);
        talk_to(&mut client, rock);
        watch(engine, &mut client, 6);
        let (nid, ..) = npcs(engine, "elem1_qip_earth_elemental_rock_version")
            .into_iter()
            .next()
            .expect("the elemental rises");

        let mut fell = None;
        for tick in 0..200 {
            let Some(n) = engine.get_npc_mut(nid) else {
                break;
            };
            let c = n.npc.pathing.coord;
            let stats = &mut n.npc.stats.levels;
            let gone = stats[rs_pack::types::NpcStat::Hitpoints as usize] == 0;
            stats[rs_pack::types::NpcStat::Hitpoints as usize] =
                stats[rs_pack::types::NpcStat::Hitpoints as usize].min(1);
            stats[rs_pack::types::NpcStat::Defence as usize] = 1;
            if gone {
                fell = Some((c.x(), c.z()));
            }
            if tick % 6 == 0 {
                let mut p = ClientPacket::new();
                p.p1_alt2(0).p2_alt3(nid as i32);
                client.send(ClientProt::OpNpc2, &p.finish());
            }
            engine.cycle();
            client.packets();
            if fell.is_some() {
                break;
            }
        }
        let fell = fell.expect("the elemental is killed");
        settle(engine, &mut client, 8);
        let ore = id(engine, "obj", "elemental_workshop_ore");
        let mut p = ClientPacket::new();
        p.p2_alt1(ore as i32)
            .p2_alt2(fell.1 as i32)
            .p1(0)
            .p2(fell.0 as i32);
        client.send(ClientProt::OpObj3, &p.finish());
        settle(engine, &mut client, 6);
        assert_eq!(
            held(engine, &client, "elemental_workshop_ore"),
            1,
            "the elemental leaves its ore"
        );

        stand_by(engine, &mut client, FURNACE, 0);
        let t = use_on(
            engine,
            &mut client,
            "elemental_workshop_ore",
            "elemental_workshop_furnace",
            FURNACE,
            10,
        );
        tells(&t, "You retrieve a bar of elemental metal.");
        assert_eq!(held(engine, &client, "elemental_workshop_bar"), 1);
        assert_eq!(held(engine, &client, "coal"), 0, "four coal go in");

        let (crafting, smithing) = (xp(engine, &client, CRAFTING), xp(engine, &client, SMITHING));
        stand_by(engine, &mut client, WORKBENCH, 0);
        let t = use_on(
            engine,
            &mut client,
            "elemental_workshop_bar",
            "elemental_workshop_workbench",
            WORKBENCH,
            10,
        );
        tells(&t, "You make an Elemental Shield.");
        assert_eq!(held(engine, &client, "elemental_shield"), 1);
        assert_eq!(bits(engine, &client), going | COMPLETE);
        assert_eq!(
            xp(engine, &client, CRAFTING) - crafting,
            50_000,
            "5,000 Crafting xp"
        );
        assert_eq!(
            xp(engine, &client, SMITHING) - smithing,
            50_200,
            "5,000 Smithing xp and the shield's 20"
        );
        no_errors(engine);
    });
}
