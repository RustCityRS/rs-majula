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

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_ver::TypeId;
use std::fs;
use std::sync::Mutex;

const WOODCUTTING: usize = 8;

/// Lumbridge's oak, Draynor's willow and a Lumbridge tree with its stump:
/// 742's map has its own trees there.
#[cfg(all(since_530, before_647))]
const OAK: (&str, (u16, u16)) = ("oaktree", (3229, 3248));
#[cfg(all(since_647, before_742))]
const OAK: (&str, (u16, u16)) = ("oaktree", (3230, 3247));
#[cfg(since_742)]
const OAK: (&str, (u16, u16)) = ("oak_38731", (3192, 3213));
#[cfg(all(since_647, before_742))]
const WILLOW: (&str, (u16, u16)) = ("willowtree", (3114, 3488));
#[cfg(since_742)]
const WILLOW: (&str, (u16, u16)) = ("willow_38627", (3113, 3487));
#[cfg(all(since_530, before_647))]
const TREE: (&str, (u16, u16), &str) = ("tree", (3226, 3248), "treestump2");
#[cfg(all(since_647, before_742))]
const TREE: (&str, (u16, u16), &str) = ("tree", (3225, 3243), "treestump2");
/// A dead tree north of Draynor and the stump it falls to.
#[cfg(all(since_530, before_647))]
const DEAD_TREE: (&str, (u16, u16), &str) = ("deadtree1", (3074, 3357), "deadtree1_large_stump");
#[cfg(since_647)]
const DEAD_TREE: (&str, (u16, u16), &str) = ("dead_tree_47594", (3084, 3371), "tree_stump_47595");
#[cfg(since_742)]
const TREE: (&str, (u16, u16), &str) = ("tree_38787", (3226, 3244), "tree_40356");

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
        tree: OAK.0,
        at: OAK.1,
        level: 15,
        hatchet: "bronze_axe",
        product: Some("oak_logs"),
        exp: 375,
        said: "You get some oak logs.",
        falls_to: None,
    });
}

// 530's willows are not in its trees table.
#[cfg(since_647)]
#[test]
fn a_2011_willow_gives_willow_logs() {
    chop(Chop {
        tree: WILLOW.0,
        at: WILLOW.1,
        level: 30,
        hatchet: "mithril_axe",
        product: Some("willow_logs"),
        exp: 675,
        said: "You get some willow logs.",
        falls_to: None,
    });
}

#[test]
fn a_2011_dead_tree_gives_logs_and_falls() {
    chop(Chop {
        tree: DEAD_TREE.0,
        at: DEAD_TREE.1,
        level: 1,
        hatchet: "bronze_axe",
        product: Some("logs"),
        exp: 250,
        said: "You get some logs.",
        falls_to: Some(DEAD_TREE.2),
    });
}

#[test]
fn a_black_hatchet_chops_at_woodcutting_6() {
    chop(Chop {
        tree: TREE.0,
        at: TREE.1,
        level: 6,
        hatchet: "black_axe",
        product: Some("logs"),
        exp: 250,
        said: "You get some logs.",
        falls_to: Some(TREE.2),
    });
}

// 530 predates the ivy.
#[cfg(since_647)]
#[test]
fn choking_ivy_gives_xp_and_no_item() {
    chop(Chop {
        tree: "wc_update09_ivy_01",
        at: (3217, 3498),
        level: 68,
        hatchet: "dragon_axe",
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
        let hatchet = engine.cache.objs.get_by_debugname(case.hatchet).unwrap().id;
        let product = case
            .product
            .map(|p| engine.cache.objs.get_by_debugname(p).unwrap().id);
        let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;

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

        client.op_loc(1, x, z, tree);

        let mut said = Vec::new();
        for tick in 0..600 {
            // The client's keep-alive, as the 2011 client sends it: ivy that takes
            // over 100 ticks to give would log a silent woodcutter out
            // (TIMEOUT_NO_RESPONSE, phases/logout.rs).
            if tick % 20 == 19 {
                client.send(ClientProt::NoTimeout, &[]);
            }
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

/// A loc's id by its name in the rev's `pack/loc.pack` (the server loads its
/// locs from the cache's data alone, which carries their display names but
/// not these debug names).
fn loc_id(name: &str) -> TypeId {
    let pack = format!(
        "{}/../../{}/loc.pack",
        env!("CARGO_MANIFEST_DIR"),
        rs_pack::PACK_DIR
    );
    fs::read_to_string(pack)
        .expect("loc.pack")
        .lines()
        .find_map(|l| l.split_once('=').filter(|(_, n)| *n == name))
        .and_then(|(id, _)| id.parse().ok().map(TypeId::from_int))
        .unwrap_or_else(|| panic!("{} has no loc {name}", rs_pack::PACK_DIR))
}

fn loc_here(engine: &Engine, id: TypeId, x: u16, z: u16) -> bool {
    engine
        .zones
        .zone(x, 0, z)
        .and_then(|zone| zone.get_loc(x, z, id))
        .is_some()
}
