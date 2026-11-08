//! Woodcutting from 289, at 2011's trees.
//!
//! Each case clicks Chop down as the client does (OPLOC1, `p2_alt1(z) p2_alt1(loc)
//! p1(ctrl) p2_alt2(x)`) from a walkable tile near the tree, with a hatchet
//! in the backpack, and cycles until the tree gives.
//!
//! - 289 names the logs it gives ("You get some oak logs.").
//! - A 2011 willow (Edgeville's, `willow_2210`; 289's willow id is a Target in
//!   2011) gives willow logs.
//! - A 2011 dead tree by Draynor Manor (`dead_tree_47594`, void-634's
//!   `dead_tree_37`) gives logs and falls to its stump at once, as 289's
//!   normal trees do.
//! - 2011's black hatchet needs Woodcutting 6, as steel does (2011 wiki,
//!   Hatchet).
//! - 2011's choking ivy on Varrock Palace's north wall gives 332.5 xp and no
//!   item (2011 wiki, Ivy).

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use std::fs;
use std::sync::Mutex;

const WOODCUTTING: usize = 8;

struct Chop {
    tree: &'static str,
    at: (u16, u16),
    level: u16,
    hatchet: &'static str,
    product: Option<&'static str>,
    exp: i32,
    said: &'static str,
    falls_to: Option<&'static str>,
}

#[test]
fn an_oak_gives_oak_logs_by_name() {
    chop(Chop {
        tree: "oak",
        at: (3230, 3247),
        level: 15,
        hatchet: "bronze_hatchet",
        product: Some("oak_logs"),
        exp: 375,
        said: "You get some oak logs.",
        falls_to: None,
    });
}

#[test]
fn a_2011_willow_gives_willow_logs() {
    chop(Chop {
        tree: "willow_2210",
        at: (3114, 3488),
        level: 30,
        hatchet: "mithril_hatchet",
        product: Some("willow_logs"),
        exp: 675,
        said: "You get some willow logs.",
        falls_to: None,
    });
}

#[test]
fn a_2011_dead_tree_gives_logs_and_falls() {
    chop(Chop {
        tree: "dead_tree_47594",
        at: (3084, 3371),
        level: 1,
        hatchet: "bronze_hatchet",
        product: Some("logs"),
        exp: 250,
        said: "You get some logs.",
        falls_to: Some("tree_stump_47595"),
    });
}

#[test]
fn a_black_hatchet_chops_at_woodcutting_6() {
    chop(Chop {
        tree: "tree_1276",
        at: (3225, 3243),
        level: 6,
        hatchet: "black_hatchet",
        product: Some("logs"),
        exp: 250,
        said: "You get some logs.",
        falls_to: Some("treestump2"),
    });
}

#[test]
fn choking_ivy_gives_xp_and_no_item() {
    chop(Chop {
        tree: "ivy_46318",
        at: (3217, 3498),
        level: 68,
        hatchet: "dragon_hatchet",
        product: None,
        exp: 3325,
        said: "You successfully chop away some ivy.",
        falls_to: None,
    });
}

fn chop(case: Chop) {
    // The cases share one loaded engine and each starts with a reset, which
    // would log the others' woodcutters out: one at a time.
    static TURN: Mutex<()> = Mutex::new(());
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(move |engine| {
        let tree = loc_id(case.tree);
        let (x, z) = case.at;
        assert!(
            loc_here(engine, tree, x, z),
            "{} is not at {x},{z}",
            case.tree
        );
        let hatchet = *engine.cache.objs.debugnames.get(case.hatchet).unwrap();
        let product = case
            .product
            .map(|p| *engine.cache.objs.debugnames.get(p).unwrap());
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

        engine.harness_reset();
        let mut client = engine
            .harness_login_returning(&format!("chopper{tree}"))
            .expect("login");
        engine.cycle();
        // A tile along a side of the tree with no loc or blocked floor.
        let ty = engine.cache.locs.get_by_id(tree).expect("loc type");
        let (w, l) = (ty.width.max(1) as i32, ty.length.max(1) as i32);
        let sides = (0..w)
            .map(|dx| (dx, l))
            .chain((0..l).map(|dz| (w, dz)))
            .chain((0..w).map(|dx| (dx, -1)))
            .chain((0..l).map(|dz| (-1, dz)));
        let (sx, sz) = sides
            .map(|(dx, dz)| {
                (
                    x.wrapping_add_signed(dx as i16),
                    z.wrapping_add_signed(dz as i16),
                )
            })
            .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
            .expect("a walkable tile beside the tree");
        engine.harness_telejump(client.pid, sx, 0, sz);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, hatchet, 1);
        player.stats.levels[WOODCUTTING] = case.level;
        player.stats.base_levels[WOODCUTTING] = case.level;
        let xp = player.stats.xp[WOODCUTTING];
        engine.cycle();
        client.packets();

        let mut p = ClientPacket::new();
        p.p2_alt1(z as i32)
            .p2_alt1(tree as i32)
            .p1(0)
            .p2_alt2(x as i32);
        client.send(ClientProt::OpLoc1, &p.finish());

        let mut said = Vec::new();
        for _ in 0..250 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            let player = engine.harness_player(client.pid).unwrap_or_else(|| {
                panic!(
                    "the woodcutter at {} is gone; the game said {said:?}",
                    case.tree
                )
            });
            if player.stats.xp[WOODCUTTING] > xp {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.stats.xp[WOODCUTTING] - xp,
            case.exp,
            "{}'s xp; the game said {said:?}",
            case.tree
        );
        let backpack = player.invs.get(&inv).unwrap();
        match product {
            Some(product) => assert_eq!(backpack.total(product), 1, "{said:?}"),
            None => assert_eq!(backpack.total(hatchet), 1, "{said:?}"),
        }
        assert_eq!(
            said.first().map(String::as_str),
            Some(if case.product.is_some() {
                "You swing your hatchet at the tree."
            } else {
                "You swing your hatchet at the ivy."
            })
        );
        assert_eq!(said.last().map(String::as_str), Some(case.said));
        if let Some(stump) = case.falls_to {
            assert!(!loc_here(engine, tree, x, z), "{} still stands", case.tree);
            assert!(
                loc_here(engine, loc_id(stump), x, z),
                "{} fell to {stump}",
                case.tree
            );
        }
    });
}

/// A loc's 647 id by its name in `content/647/pack/loc.pack` (the server
/// loads 647 locs from the cache's data alone, which carries their display
/// names but not these debug names).
fn loc_id(name: &str) -> u16 {
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

fn loc_here(engine: &Engine, id: u16, x: u16, z: u16) -> bool {
    engine
        .zones
        .zone(x, 0, z)
        .and_then(|zone| zone.get_loc(x, z, id))
        .is_some()
}
