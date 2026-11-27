//! 289's general-use scripts (`general_use/scripts`) on 2011's locs and objs:
//! the hat stand (loc 374, 28 places in the 786 map) and the hammer on a
//! gem. `OPLOCT` and `IFBUTTONT` are written as the 647 client's send sites
//! write them (see `targeted_ops.rs`, `held_ops.rs`).

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_test::talk::converse;
use rs_ver::TypeId;
use rs_vm::ops::test::{calls_for, forget_calls};
use std::fs;

/// Seers' Village's hat stand (`l42_54`: 0 22 26), and the tile south of it.
const HAT_STAND: (u16, u16) = (2710, 3482);
/// The well west of Lumbridge: 742 has its own.
#[cfg(before_742)]
const WELL: &str = "well";
#[cfg(since_742)]
const WELL: &str = "well_54639";
/// The Taverley Dungeon coffin, shut and open: 742 has its own.
#[cfg(before_742)]
const COFFIN_SHUT: &str = "di2_coffin";
#[cfg(before_742)]
const COFFIN_OPEN: &str = "di2_coffin_open";
#[cfg(since_742)]
const COFFIN_SHUT: &str = "coffin_74824";
#[cfg(since_742)]
const COFFIN_OPEN: &str = "coffin_74825";
const STAND_TILE: (u16, u16) = (2710, 3481);

fn obj(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
        .id
}

/// A fresh world with one player at `x, z` holding `objs` from slot 0.
fn holding(engine: &mut Engine, name: &str, x: u16, z: u16, objs: &[u16]) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.harness_telejump(client.pid, x, 0, z);
    engine.cycle();
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let backpack = engine
        .harness_player_mut(client.pid)
        .and_then(|p| p.invs.get_mut(&inv))
        .expect("backpack");
    for (slot, &o) in objs.iter().enumerate() {
        backpack.set(slot as u16, TypeId::new(o), 1);
    }
    engine.cycle();
    client.packets();
    forget_calls();
    client
}

/// The item in backpack slot 0 used on the hat stand.
fn use_on_hat_stand(client: &mut HarnessClient, item: u16) {
    client.op_loc_t(
        HAT_STAND.0,
        HAT_STAND.1,
        TypeId::from_int(374),
        BACKPACK,
        0,
        TypeId::from_int(item as i32),
    );
}

fn said(line: &str) -> bool {
    calls_for("mes").iter().any(|m| m == line)
}

/// A hat on the hat stand falls off: 289's joke, the hat dropped to the floor.
#[test]
fn a_hat_falls_off_the_hat_stand_and_nothing_else_goes_on_it() {
    runner::on_loaded_engine(|engine| {
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
        let hat = obj(engine, "bluewizhat");
        let mut client = holding(engine, "hatter", STAND_TILE.0, STAND_TILE.1, &[hat.g2()]);
        use_on_hat_stand(&mut client, hat.g2());
        for _ in 0..10 {
            engine.cycle();
        }
        let backpack = engine
            .harness_player(client.pid)
            .and_then(|p| p.invs.get(&inv))
            .unwrap();
        assert!(!backpack.has_at(0, hat), "the hat left the backpack");

        let logs = obj(engine, "logs");
        let mut client = holding(engine, "hatter2", STAND_TILE.0, STAND_TILE.1, &[logs.g2()]);
        use_on_hat_stand(&mut client, logs.g2());
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
        let mut client = holding(engine, "smasher", 3222, 3219, &[hammer.g2(), topaz.g2()]);
        // The topaz (slot 1) used on the hammer (slot 0): 289's [opheldu,hammer]
        // switches on the item used.
        client.if_button_t(BACKPACK, 1, topaz, BACKPACK, 0, hammer);
        for _ in 0..3 {
            engine.cycle();
        }
        assert!(said("You can't think of any reason to do this."));
    });
}

/// A loc's id by its name in the rev's `pack/loc.pack` (the server
/// loads its locs from the cache's data alone, which carries no names).
fn loc_id(name: &str) -> i32 {
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
        .unwrap_or_else(|| panic!("{} has no loc {name}", rs_pack::CONTENT_DIR))
}

/// Whether the loc `name` stands at `x`,`level`,`z` now.
fn loc_at(engine: &Engine, name: &str, x: u16, level: u8, z: u16) -> bool {
    let id = loc_id(name);
    engine
        .zones
        .zone(x, level, z)
        .is_some_and(|zone| zone.get_loc(x, z, TypeId::from_int(id)).is_some())
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

fn op_loc1(client: &mut HarnessClient, loc: i32, x: u16, z: u16) {
    client.op_loc(1, x, z, TypeId::from_int(loc));
}

/// Backpack slot 0's item used on the loc `loc` at `x`,`z` (`OPLOCT`).
fn use_on_loc(client: &mut HarnessClient, item: u16, loc: i32, x: u16, z: u16) {
    client.op_loc_t(
        x,
        z,
        TypeId::from_int(loc),
        BACKPACK,
        0,
        TypeId::from_int(item as i32),
    );
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
    forget_calls();
    client
}

fn cycles(engine: &mut Engine, ticks: usize) {
    for _ in 0..ticks {
        engine.cycle();
    }
}

fn slot0(engine: &Engine, pid: u16) -> Option<TypeId> {
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let backpack = engine.harness_player(pid).and_then(|p| p.invs.get(&inv))?;
    backpack.get(0).map(|item| item.obj)
}

/// The 2011 wiki keeps Morgan's garlic in drawers (Garlic, 2011-05-01), where
/// 289 had its garlic cupboard: drawers_46250, a tile north of 289's spot,
/// opens into drawers_12961 (cupboards.loc), whose Search gives 289's garlic.
// 742's Draynor has no drawers on Morgan's upper floor.
#[cfg(before_742)]
#[test]
fn morgan_s_drawers_hold_289_s_garlic() {
    runner::on_loaded_engine(|engine| {
        const DRAWERS: (u16, u8, u16) = (3096, 1, 3269);
        // 530's map has a cupboard there (33502, opening into 33503; cupboards.rs2).
        #[cfg(before_647)]
        let (shut, open) = (
            "xmas09_landofsnow_fence_02_mirror",
            "xmas09_landofsnow_fence_03",
        );
        #[cfg(since_647)]
        let (shut, open) = ("drawers_46250", "drawers_12961");
        let garlic = obj(engine, "garlic");
        let mut client = beside_loc(engine, "morgan", DRAWERS, &[]);
        op_loc1(&mut client, loc_id(shut), DRAWERS.0, DRAWERS.2);
        cycles(engine, 6);
        assert!(
            loc_at(engine, open, DRAWERS.0, 1, DRAWERS.2),
            "the drawers opened"
        );

        op_loc1(&mut client, loc_id(open), DRAWERS.0, DRAWERS.2);
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
            loc_id("fai_wiztower_bookcase"),
            BOOKCASE.0,
            BOOKCASE.2,
        );
        cycles(engine, 6);
        assert!(said("You search the bookcase..."), "289's search");

        // coffin_32290 opens into coffin_32291, which closes it again.
        const COFFIN: (u16, u8, u16) = (2878, 0, 9803);
        let mut client = beside_loc(engine, "undertaker", COFFIN, &[]);
        op_loc1(&mut client, loc_id(COFFIN_SHUT), COFFIN.0, COFFIN.2);
        cycles(engine, 6);
        assert!(said("The coffin creaks open..."));
        assert!(loc_at(engine, COFFIN_OPEN, COFFIN.0, 0, COFFIN.2), "opened");
        op_loc1(&mut client, loc_id(COFFIN_OPEN), COFFIN.0, COFFIN.2);
        cycles(engine, 6);
        assert!(said("You close the coffin."));
        assert!(loc_at(engine, COFFIN_SHUT, COFFIN.0, 0, COFFIN.2), "shut");
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
        let mut client = beside_loc(engine, "waiter", TABLE, &[logs.g2()]);
        use_on_loc(
            &mut client,
            logs.g2(),
            loc_id("table_11743"),
            TABLE.0,
            TABLE.2,
        );
        cycles(engine, 6);
        assert_eq!(
            slot0(engine, client.pid),
            None,
            "the logs left the backpack"
        );

        // barrelwithtap at 0_48_51_23_2 (cache 786 map); 742's Draynor has none.
        #[cfg(before_742)]
        {
            const BARREL: (u16, u8, u16) = (3095, 0, 3266);
            let (glass, beer) = (obj(engine, "beer_glass"), obj(engine, "beer"));
            let mut client = beside_loc(engine, "drinker", BARREL, &[glass.g2()]);
            use_on_loc(
                &mut client,
                glass.g2(),
                loc_id("barrelwithtap"),
                BARREL.0,
                BARREL.2,
            );
            cycles(engine, 6);
            assert_eq!(slot0(engine, client.pid), Some(beer), "beer");
            assert!(said("You fill the glass with beer."));
        }
    });
}

/// Backpack slot `slot`'s item's op `op` (647 sends an inventory op as an
/// `IF_BUTTON` on `inventory:inv`, which `[if_button,inventory:inv]` hands to
/// the item's `opheld` script).
fn op_held(client: &mut HarnessClient, op: u8, obj: u16, slot: u16) {
    client.if_button(op, BACKPACK, slot, TypeId::from_int(obj as i32));
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
        let seeds = obj(engine, "mithril_seed");
        let (x, z) = (3222, 3219);
        let mut client = holding(engine, "gardener", x, z, &[seeds.g2()]);
        op_held(&mut client, 1, seeds.g2(), 0);
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
        // 530's organ is not reached from the north: it is played from the south.
        #[cfg(before_647)]
        {
            engine.harness_telejump(client.pid, ORGAN.0, ORGAN.1, ORGAN.2 - 1);
            cycles(engine, 2);
            client.packets();
        }
        op_loc1(&mut client, loc_id("church_organ_30754"), ORGAN.0, ORGAN.2);
        cycles(engine, 6);
        let songs = calls_for("midi_song");
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
        op_loc1(&mut client, loc_id("mournerstewfence"), FENCE.0, FENCE.2);
        cycles(engine, 8);
        let through = coord(engine, client.pid);
        assert_ne!(through, start, "through the fence");
        assert_eq!(
            through.0.abs_diff(start.0) + through.2.abs_diff(start.2),
            1,
            "a tile across: {start:?} -> {through:?}"
        );
        op_loc1(&mut client, loc_id("mournerstewfence"), FENCE.0, FENCE.2);
        cycles(engine, 8);
        assert_eq!(coord(engine, client.pid), start, "and back");
    });
}

fn op_loc2(client: &mut HarnessClient, loc: i32, x: u16, z: u16) {
    client.op_loc(2, x, z, TypeId::from_int(loc));
}

/// The manhole at 0_50_54_37_2 opens, closes again on the open manhole's own
/// Close (786's manholeopen has op2 Close), and climbs down to void-634's
/// recorded landing.
#[test]
fn varrock_s_manhole_opens_closes_and_climbs_down() {
    runner::on_loaded_engine(|engine| {
        const MANHOLE: (u16, u8, u16) = (3237, 0, 3458);
        let mut client = beside_loc(engine, "sewerman", MANHOLE, &[]);
        op_loc1(&mut client, loc_id("manholeclosed"), MANHOLE.0, MANHOLE.2);
        cycles(engine, 6);
        assert!(said("You pull back the cover from over the manhole."));
        assert!(
            loc_at(engine, "manholeopen", MANHOLE.0, 0, MANHOLE.2),
            "open"
        );

        op_loc2(&mut client, loc_id("manholeopen"), MANHOLE.0, MANHOLE.2);
        cycles(engine, 6);
        assert!(said("You place the cover back over the manhole."));
        assert!(
            loc_at(engine, "manholeclosed", MANHOLE.0, 0, MANHOLE.2),
            "shut"
        );

        op_loc1(&mut client, loc_id("manholeclosed"), MANHOLE.0, MANHOLE.2);
        cycles(engine, 6);
        op_loc1(&mut client, loc_id("manholeopen"), MANHOLE.0, MANHOLE.2);
        // 2011 asks first (2011-04-20 recording, 319.3-320.3s).
        let talk = converse(engine, &mut client, &["Yes, please."], 30);
        assert!(
            talk.said("This sewer looks quite dangerous, are you sure you want to enter?"),
            "{:?}",
            talk.texts
        );
        assert!(talk.said("Enter the sewer?"), "{:?}", talk.texts);
        cycles(engine, 6);
        assert!(said("You climb down through the manhole."));
        let (x, _, z) = coord(engine, client.pid);
        assert!(z > 9600, "in the sewer: at {x},{z}");
    });
}

/// 289's karamjashipplank_on boards to the east and a level up, as 289's did,
/// though 786 lays it turned half round (shape 22 angle 2; 289's shape 10
/// angle 0).
#[test]
fn port_sarim_s_karamja_gangplank_boards_the_ship_to_the_east() {
    runner::on_loaded_engine(|engine| {
        let mut client = holding(engine, "deckhand", 3029, 3217, &[]);
        op_loc1(&mut client, loc_id("karamjashipplank_on"), 3030, 3217);
        cycles(engine, 6);
        assert!(said("You board the ship."));
        assert_eq!(coord(engine, client.pid), (3032, 1, 3217), "on the deck");
    });
}

/// 289's locked chest (786 keeps chestlocked, 2995: "Closed chest", op1 Open)
/// says it is locked; 289's locked gate says it is tried and no more, as
/// upstream's 289 now has it.
#[test]
fn a_locked_chest_and_a_locked_gate_stay_shut() {
    runner::on_loaded_engine(|engine| {
        // chestlocked at 0_40_60_60_53, from the tile west of it (walls run
        // along the tiles south of it). 742 walls its tile in.
        #[cfg(before_742)]
        {
            // 530's map has it a tile east.
            #[cfg(before_647)]
            const CHEST: (u16, u16) = (2621, 3893);
            #[cfg(since_647)]
            const CHEST: (u16, u16) = (2620, 3893);
            let mut client = holding(engine, "lockpick", CHEST.0 - 1, CHEST.1, &[]);
            op_loc1(&mut client, loc_id("chestlocked"), CHEST.0, CHEST.1);
            cycles(engine, 6);
            assert!(said("The chest is locked."));
        }

        // lockedmetalgatel at 1_42_54_38_38.
        const GATE: (u16, u8, u16) = (2726, 1, 3494);
        let mut client = beside_loc(engine, "gatecrasher", GATE, &[]);
        op_loc1(&mut client, loc_id("lockedmetalgatel"), GATE.0, GATE.2);
        cycles(engine, 8);
        assert!(said("You try to open the gate..."));
        assert!(
            !said("The gate is locked securely, it won't open!"),
            "289 says no more"
        );
    });
}

/// 289's water sources (water_sources.rs2): a bucket used on Lumbridge
/// castle's 2011 kitchen sink (sink_36971 at 0_50_50_5_15, a water source by
/// void-634's Filling.kt) fills, and on the well west of Lumbridge (289's
/// well_884 at 0_49_50_25_20) fills as 289's wells do.
#[test]
fn a_bucket_fills_at_a_sink_and_a_well() {
    runner::on_loaded_engine(|engine| {
        let (bucket, water) = (obj(engine, "bucket_empty"), obj(engine, "bucket_water"));
        for (name, loc, (x, z), line) in [
            (
                "sinker",
                "sink_36971",
                (3205, 3215),
                "You fill the bucket from the sink.",
            ),
            (
                "weller",
                WELL,
                (3161, 3220),
                "You fill the bucket from the well.",
            ),
        ] {
            let mut client = beside_loc(engine, name, (x, 0, z), &[bucket.g2()]);
            use_on_loc(&mut client, bucket.g2(), loc_id(loc), x, z);
            cycles(engine, 8);
            assert!(said(line), "{loc}: {:?}", calls_for("mes"));
            assert_eq!(
                slot0(engine, client.pid),
                Some(water),
                "{loc} filled the bucket"
            );
        }
    });
}
