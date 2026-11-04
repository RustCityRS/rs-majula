//! 289's general-use scripts (`general_use/scripts`) on 2011's locs and objs:
//! the hat stand (loc 374, 28 places in the 786 map) and the hammer on a
//! gem. `OPLOCT` and `IFBUTTONT` are written as the 647 client's send sites
//! write them (see `targeted_ops.rs`, `held_ops.rs`).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

/// `inventory:inv`.
const BACKPACK: i32 = 149 << 16;
/// Seers' Village's hat stand (`l42_54`: 0 22 26), and the tile south of it.
const HAT_STAND: (u16, u16) = (2710, 3482);
const STAND_TILE: (u16, u16) = (2710, 3481);

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

/// A fresh world with one player at `x, z` holding `objs` from slot 0.
fn holding(engine: &mut Engine, name: &str, x: u16, z: u16, objs: &[u16]) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.harness_telejump(client.pid, x, 0, z);
    engine.cycle();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let backpack = engine
        .harness_player_mut(client.pid)
        .and_then(|p| p.invs.get_mut(&inv))
        .expect("backpack");
    for (slot, &o) in objs.iter().enumerate() {
        backpack.set(slot as u16, o, 1);
    }
    engine.cycle();
    client.packets();
    rs_vm::ops::test::forget_calls();
    client
}

/// The item in backpack slot 0 used on the hat stand.
fn use_on_hat_stand(client: &mut HarnessClient, item: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt1(item as i32)
        .p2_alt1(0)
        .p2(HAT_STAND.1 as i32)
        .p4_alt2(BACKPACK)
        .p2_alt1(HAT_STAND.0 as i32)
        .p1_alt3(0)
        .ip2(374);
    client.send(ClientProt::OpLocT, &p.finish());
}

fn said(line: &str) -> bool {
    rs_vm::ops::test::calls_for("mes").iter().any(|m| m == line)
}

/// A hat on the hat stand falls off: 289's joke, the hat dropped to the floor.
#[test]
fn a_hat_falls_off_the_hat_stand_and_nothing_else_goes_on_it() {
    runner::on_loaded_engine(|engine| {
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let hat = obj(engine, "wizard_hat");
        let mut client = holding(engine, "hatter", STAND_TILE.0, STAND_TILE.1, &[hat]);
        use_on_hat_stand(&mut client, hat);
        for _ in 0..10 {
            engine.cycle();
        }
        let backpack = engine
            .harness_player(client.pid)
            .and_then(|p| p.invs.get(&inv))
            .unwrap();
        assert!(!backpack.has_at(0, hat), "the hat left the backpack");

        let logs = obj(engine, "logs");
        let mut client = holding(engine, "hatter2", STAND_TILE.0, STAND_TILE.1, &[logs]);
        use_on_hat_stand(&mut client, logs);
        for _ in 0..10 {
            engine.cycle();
        }
        assert!(said("You can't put that on a hat stand."), "289's refusal");
    });
}

/// A hammer used on a red topaz, opal or jade (cut or not): 289's line.
#[test]
fn a_hammer_on_a_semi_precious_gem() {
    runner::on_loaded_engine(|engine| {
        let (hammer, topaz) = (obj(engine, "hammer"), obj(engine, "red_topaz"));
        let mut client = holding(engine, "smasher", 3222, 3219, &[hammer, topaz]);
        let mut p = ClientPacket::new();
        // The topaz (slot 1) used on the hammer (slot 0): 289's [opheldu,hammer]
        // switches on the item used.
        p.p4(BACKPACK)
            .ip2(0)
            .ip2(1)
            .p2_alt1(topaz as i32)
            .ip4(BACKPACK)
            .p2(hammer as i32);
        client.send(ClientProt::IfButtonT, &p.finish());
        for _ in 0..3 {
            engine.cycle();
        }
        assert!(said("You can't think of any reason to do this."));
    });
}

/// A loc's 647 id by its name in `content/647/pack/loc.pack` (the server
/// loads 647 locs from the cache's data alone, which carries no names).
fn loc_id(name: &str) -> u16 {
    let pack = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/647/pack/loc.pack"
    );
    std::fs::read_to_string(pack)
        .expect("loc.pack")
        .lines()
        .find_map(|l| l.split_once('=').filter(|(_, n)| *n == name))
        .and_then(|(id, _)| id.parse().ok())
        .unwrap_or_else(|| panic!("647 has no loc {name}"))
}

/// Whether the loc `name` stands at `x`,`level`,`z` now.
fn loc_at(engine: &Engine, name: &str, x: u16, level: u8, z: u16) -> bool {
    let id = loc_id(name);
    engine
        .zones
        .zone(x, level, z)
        .is_some_and(|zone| zone.get_loc(x, z, id).is_some())
}

/// A walkable tile beside `x`,`z` on `level`.
fn beside(x: u16, level: u8, z: u16) -> (u16, u16) {
    [
        (0, 1),
        (1, 0),
        (0, -1),
        (-1, 0),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
    ]
    .into_iter()
    .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
    .find(|&(tx, tz)| !rsmod::is_flagged(tx, tz, level, 0x100 | 0x200000))
    .expect("a walkable tile beside the loc")
}

/// `OPLOC1`: `ip2(z) ip2(loc) p1(ctrl) p2_alt1(x)`.
fn op_loc1(client: &mut HarnessClient, loc: u16, x: u16, z: u16) {
    let mut p = ClientPacket::new();
    p.ip2(z as i32).ip2(loc as i32).p1(0).p2_alt1(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

/// Backpack slot 0's item used on the loc `loc` at `x`,`z` (`OPLOCT`).
fn use_on_loc(client: &mut HarnessClient, item: u16, loc: u16, x: u16, z: u16) {
    let mut p = ClientPacket::new();
    p.p2_alt1(item as i32)
        .p2_alt1(0)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt1(x as i32)
        .p1_alt3(0)
        .ip2(loc as i32);
    client.send(ClientProt::OpLocT, &p.finish());
}

/// A player beside the loc at `x`,`level`,`z`, holding `objs` from slot 0.
fn beside_loc(
    engine: &mut Engine,
    name: &str,
    (x, level, z): (u16, u8, u16),
    objs: &[u16],
) -> HarnessClient {
    let (sx, sz) = beside(x, level, z);
    let mut client = holding(engine, name, sx, sz, objs);
    engine.harness_telejump(client.pid, sx, level, sz);
    for _ in 0..2 {
        engine.cycle();
    }
    client.packets();
    rs_vm::ops::test::forget_calls();
    client
}

fn cycles(engine: &mut Engine, ticks: usize) {
    for _ in 0..ticks {
        engine.cycle();
    }
}

fn slot0(engine: &Engine, pid: u16) -> Option<u16> {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let backpack = engine.harness_player(pid).and_then(|p| p.invs.get(&inv))?;
    backpack.get(0).map(|item| item.obj)
}

/// The 2011 wiki keeps Morgan's garlic in drawers (Garlic, 2011-05-01), where
/// 289 had its garlic cupboard: drawers_46250, a tile north of 289's spot,
/// opens into drawers_12961 (cupboards.loc), whose Search gives 289's garlic.
#[test]
fn morgan_s_drawers_hold_289_s_garlic() {
    runner::on_loaded_engine(|engine| {
        const DRAWERS: (u16, u8, u16) = (3096, 1, 3269);
        let garlic = obj(engine, "garlic");
        let mut client = beside_loc(engine, "morgan", DRAWERS, &[]);
        op_loc1(&mut client, loc_id("drawers_46250"), DRAWERS.0, DRAWERS.2);
        cycles(engine, 6);
        assert!(
            loc_at(engine, "drawers_12961", DRAWERS.0, 1, DRAWERS.2),
            "the drawers opened"
        );

        op_loc1(&mut client, loc_id("drawers_12961"), DRAWERS.0, DRAWERS.2);
        cycles(engine, 6);
        assert_eq!(slot0(engine, client.pid), Some(garlic), "garlic taken");
        assert!(said("You find a clove of garlic that you take."));
    });
}

/// 2011's own bookcases and coffins on 289's spots (bookcases_2011.loc,
/// coffins.loc) are searched, opened and shut as 289's.
#[test]
fn two_thousand_eleven_s_bookcases_and_coffins_on_289_s_spots() {
    runner::on_loaded_engine(|engine| {
        // bookcase_12539 at 0_48_49_36_19, on a 289 bookcase spot.
        const BOOKCASE: (u16, u8, u16) = (3108, 0, 3155);
        let mut client = beside_loc(engine, "reader", BOOKCASE, &[]);
        op_loc1(
            &mut client,
            loc_id("bookcase_12539"),
            BOOKCASE.0,
            BOOKCASE.2,
        );
        cycles(engine, 6);
        assert!(said("You search the bookcase..."), "289's search");

        // coffin_32290 opens into coffin_32291, which closes it again.
        const COFFIN: (u16, u8, u16) = (2878, 0, 9803);
        let mut client = beside_loc(engine, "undertaker", COFFIN, &[]);
        op_loc1(&mut client, loc_id("coffin_32290"), COFFIN.0, COFFIN.2);
        cycles(engine, 6);
        assert!(said("The coffin creaks open..."));
        assert!(
            loc_at(engine, "coffin_32291", COFFIN.0, 0, COFFIN.2),
            "opened"
        );
        op_loc1(&mut client, loc_id("coffin_32291"), COFFIN.0, COFFIN.2);
        cycles(engine, 6);
        assert!(said("You close the coffin."));
        assert!(
            loc_at(engine, "coffin_32290", COFFIN.0, 0, COFFIN.2),
            "shut"
        );
    });
}

/// An item on 2011's table on 289's table spot goes on the table; a beer
/// glass on a barrel with a tap fills.
#[test]
fn a_table_takes_an_item_and_a_tap_barrel_fills_a_glass() {
    runner::on_loaded_engine(|engine| {
        // table_11743 at 0_40_51_42_22, on a 289 usable table's spot.
        const TABLE: (u16, u8, u16) = (2602, 0, 3286);
        let logs = obj(engine, "logs");
        let mut client = beside_loc(engine, "waiter", TABLE, &[logs]);
        use_on_loc(&mut client, logs, loc_id("table_11743"), TABLE.0, TABLE.2);
        cycles(engine, 6);
        assert_eq!(
            slot0(engine, client.pid),
            None,
            "the logs left the backpack"
        );

        // barrelwithtap at 0_48_51_23_2 (cache 786 map).
        const BARREL: (u16, u8, u16) = (3095, 0, 3266);
        let (glass, beer) = (obj(engine, "beer_glass"), obj(engine, "beer"));
        let mut client = beside_loc(engine, "drinker", BARREL, &[glass]);
        use_on_loc(
            &mut client,
            glass,
            loc_id("barrelwithtap"),
            BARREL.0,
            BARREL.2,
        );
        cycles(engine, 6);
        assert_eq!(slot0(engine, client.pid), Some(beer), "beer");
        assert!(said("You fill the glass with beer."));
    });
}

/// Backpack slot `slot`'s item's op `op` (647 sends an inventory op as an
/// `IF_BUTTON` on `inventory:inv`, which `[if_button,inventory:inv]` hands to
/// the item's `opheld` script).
fn op_held(client: &mut HarnessClient, op: u8, obj: u16, slot: u16) {
    let prot = match op {
        1 => ClientProt::IfButton1,
        _ => ClientProt::IfButton2,
    };
    let mut p = ClientPacket::new();
    p.p4(BACKPACK).ip2_alt1(obj as i32).ip2(slot as i32);
    client.send(prot, &p.finish());
}

/// 2011's own searchable hay, on 289's hay spots, searches as 289's.
#[test]
fn hay_searches_as_289_s() {
    runner::on_loaded_engine(|engine| {
        // hay_bale_36892 at 0_49_51_11_40, on 289's haybail spot.
        const HAY_BALE: (u16, u8, u16) = (3147, 0, 3304);
        let mut client = beside_loc(engine, "farmhand", HAY_BALE, &[]);
        op_loc1(
            &mut client,
            loc_id("hay_bale_36892"),
            HAY_BALE.0,
            HAY_BALE.2,
        );
        cycles(engine, 6);
        assert!(said("You search the hay bale..."), "2011's hay_bale_36892");
    });
}

/// The mithril seeds plant one of 289's flowers at the player's feet, and
/// church_organ_30754 (organs.rs2) plays 289's church music.
#[test]
fn mithril_seeds_plant_flowers_and_the_paterdomus_organ_plays() {
    runner::on_loaded_engine(|engine| {
        let seeds = obj(engine, "mithril_seeds");
        let (x, z) = (3222, 3219);
        let mut client = holding(engine, "gardener", x, z, &[seeds]);
        op_held(&mut client, 1, seeds, 0);
        cycles(engine, 3);
        assert!(said("You drop a seed by your feet."));
        let flowers = [
            "flower_waterfall_quest",
            "flower_waterfall_quest_red",
            "flower_waterfall_quest_blue",
            "flower_waterfall_quest_yellow",
            "flower_waterfall_quest_purple",
            "flower_waterfall_quest_orange",
            "flower_waterfall_quest_mixed",
            "flower_waterfall_quest_white",
            "flower_waterfall_quest_black",
        ];
        assert!(
            flowers.iter().any(|f| loc_at(engine, f, x, 0, z)),
            "flowers where the seed fell"
        );

        const ORGAN: (u16, u8, u16) = (3414, 0, 3494);
        let mut client = beside_loc(engine, "organist", ORGAN, &[]);
        op_loc1(&mut client, loc_id("church_organ_30754"), ORGAN.0, ORGAN.2);
        cycles(engine, 6);
        let songs = rs_vm::ops::test::calls_for("midi_song");
        assert!(
            songs.iter().any(|s| s == "39" || s == "40"),
            "church music 1 or 2: {songs:?}"
        );
    });
}

fn coord(engine: &Engine, pid: u16) -> (u16, u8, u16) {
    let c = engine.harness_player(pid).unwrap().pathing.coord;
    (c.x(), c.y(), c.z())
}

/// 289's mournerstewfence, 2011's fence (2068, op1 Squeeze-through, on 289's
/// spot), takes the player through to the other side and back.
#[test]
fn the_fence_squeezes_through_both_ways() {
    runner::on_loaded_engine(|engine| {
        const FENCE: (u16, u8, u16) = (2541, 0, 3331);
        let mut client = holding(engine, "squeezer", FENCE.0, FENCE.2, &[]);
        let start = coord(engine, client.pid);
        op_loc1(&mut client, loc_id("fence"), FENCE.0, FENCE.2);
        cycles(engine, 8);
        let through = coord(engine, client.pid);
        assert_ne!(through, start, "through the fence");
        assert_eq!(
            through.0.abs_diff(start.0) + through.2.abs_diff(start.2),
            1,
            "a tile across: {start:?} -> {through:?}"
        );
        op_loc1(&mut client, loc_id("fence"), FENCE.0, FENCE.2);
        cycles(engine, 8);
        assert_eq!(coord(engine, client.pid), start, "and back");
    });
}
