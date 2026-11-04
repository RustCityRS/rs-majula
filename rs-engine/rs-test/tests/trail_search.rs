//! 289's Treasure Trail steps on 2011's scenery: searching drawers, crates
//! and chests, and digging with a spade.
//!
//! 289's `general_use` hooks check a held clue's spot against the loc
//! searched or the tile dug. 2011 remade much of the scenery: its drawers and
//! chests open to a loc whose Search is op1 or op2
//! (`general_use/configs/searchables_2011.loc`). Each case holds a clue at a
//! spot and does what a player does there; the clue goes, and the trail moves
//! on (a new clue, or a casket). `OPLOC1`: `ip2(z) ip2(loc) p1(ctrl)
//! p2_alt1(x)`; `OPLOC2`: `ip2(loc) p1_alt1(ctrl) p2_alt1(z) ip2_alt1(x)`;
//! a backpack op is IF_BUTTON1 on `inventory:inv`.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;

const BACKPACK: i32 = 149 << 16;

/// An obj's id from the cache's names; a loc's from 647's registry, since
/// 647's locs carry no debugname.
fn id(engine: &Engine, kind: &str, name: &str) -> u16 {
    if kind == "obj" {
        return *engine
            .cache
            .objs
            .debugnames
            .get(name)
            .unwrap_or_else(|| panic!("no obj {name}"));
    }
    let path = format!(
        "{}/../../content/647/pack/loc.pack",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path)
        .expect(&path)
        .lines()
        .find_map(|l| l.split_once('=').filter(|(_, n)| *n == name))
        .and_then(|(i, _)| i.parse().ok())
        .unwrap_or_else(|| panic!("no loc {name}"))
}

/// A player at `(level, x, z)` holding `items`, one to a slot.
fn holder(engine: &mut Engine, name: &str, at: (u8, u16, u16), items: &[&str]) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let objs: Vec<u16> = items.iter().map(|i| id(engine, "obj", i)).collect();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let backpack = engine
        .harness_player_mut(client.pid)
        .and_then(|p| p.invs.get_mut(&inv))
        .unwrap();
    for (slot, o) in objs.into_iter().enumerate() {
        backpack.set(slot as u16, o, 1);
    }
    engine.harness_telejump(client.pid, at.1, at.0, at.2);
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    client
}

fn ticks(engine: &mut Engine, n: usize) {
    for _ in 0..n {
        engine.cycle();
    }
}

fn has(engine: &Engine, pid: u16, obj: u16) -> bool {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let backpack = engine.harness_player(pid).unwrap().invs.get(&inv).unwrap();
    (0..backpack.capacity as u16).any(|s| backpack.get(s).is_some_and(|i| i.obj == obj))
}

/// Any clue or casket of a trail in the backpack, other than `except`.
fn trail_item(engine: &Engine, pid: u16, except: u16) -> Option<String> {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let backpack = engine.harness_player(pid).unwrap().invs.get(&inv).unwrap();
    (0..backpack.capacity as u16)
        .filter_map(|s| backpack.get(s))
        .filter(|i| i.obj != except)
        .filter_map(|i| {
            engine
                .cache
                .objs
                .debugnames
                .iter()
                .find(|&(_, &v)| v == i.obj)
                .map(|(k, _)| k.to_string())
        })
        .find(|n| n.starts_with("trail_clue_") || n.starts_with("casket"))
}

/// Stands the player on each tile beside `spot` in turn and does `op` until
/// `done` holds, as a player walks round a loc reached from one side only
/// (2011's drawers and 289's chests force their approach). Returns what the
/// game said.
fn beside(
    engine: &mut Engine,
    client: &mut HarnessClient,
    (y, x, z): (u8, u16, u16),
    op: impl Fn(&mut HarnessClient),
    done: impl Fn(&Engine) -> bool,
) -> Vec<String> {
    let mut said = Vec::new();
    for (dx, dz) in [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
        engine.harness_telejump(
            client.pid,
            (x as i32 + dx) as u16,
            y,
            (z as i32 + dz) as u16,
        );
        for _ in 0..2 {
            engine.cycle();
        }
        client.packets();
        op(client);
        for _ in 0..8 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, m)| m),
            );
        }
        if done(engine) {
            break;
        }
    }
    said
}

/// The loc on `(x, z)` of `level`, if one is there with id `loc`.
fn loc_there(engine: &Engine, (y, x, z): (u8, u16, u16), loc: u16) -> bool {
    engine
        .zones
        .zone(x, y, z)
        .is_some_and(|zone| zone.get_loc(x, z, loc).is_some())
}

fn op_loc1(client: &mut HarnessClient, loc: u16, (_, x, z): (u8, u16, u16)) {
    let mut p = ClientPacket::new();
    p.ip2(z as i32).ip2(loc as i32).p1(0).p2_alt1(x as i32);
    client.send(ClientProt::OpLoc1, &p.finish());
}

fn op_loc2(client: &mut HarnessClient, loc: u16, (_, x, z): (u8, u16, u16)) {
    let mut p = ClientPacket::new();
    p.ip2(loc as i32)
        .p1_alt1(0)
        .p2_alt1(z as i32)
        .ip2_alt1(x as i32);
    client.send(ClientProt::OpLoc2, &p.finish());
}

#[test]
fn a_2011_drawer_opens_and_its_search_moves_the_trail() {
    runner::on_loaded_engine(|engine| {
        // "Search the drawers, upstairs in the bank to the East of Varrock" --
        // simple011, at
        // 2011's drawers 24294 (op1 Open), which open to 24295 (op1 Search,
        // op2 Close: empty_drawer_2011).
        let spot = (1, 3250, 3420);
        let clue = id(engine, "obj", "trail_clue_easy_simple011");
        let (closed, open) = (
            id(engine, "loc", "drawers_24294"),
            id(engine, "loc", "drawers_24295"),
        );
        let mut client = holder(
            engine,
            "drawers",
            (1, 3250, 3418),
            &["trail_clue_easy_simple011"],
        );
        assert!(
            loc_there(engine, spot, closed),
            "2011's drawers at the spot"
        );
        let said = beside(
            engine,
            &mut client,
            spot,
            |c| op_loc1(c, closed, spot),
            |e| loc_there(e, spot, open),
        );
        assert!(
            loc_there(engine, spot, open),
            "the drawers opened; said {said:?}"
        );
        op_loc1(&mut client, open, spot);
        ticks(engine, 8);
        assert!(!has(engine, client.pid, clue), "the clue is used");
        assert!(
            trail_item(engine, client.pid, clue).is_some(),
            "the trail goes on: a new clue or a casket"
        );
    });
}

#[test]
fn a_2011_crate_search_moves_the_trail() {
    runner::on_loaded_engine(|engine| {
        // "Search the crates in the most north-western house in Al Kharid" --
        // simple006, at 2011's crate 46238 (op1 Search: empty_crate).
        let spot = (0, 3289, 3202);
        let clue = id(engine, "obj", "trail_clue_easy_simple006");
        let crate_ = id(engine, "loc", "crate_46238");
        let mut client = holder(
            engine,
            "crates",
            (0, 3289, 3200),
            &["trail_clue_easy_simple006"],
        );
        assert!(loc_there(engine, spot, crate_), "2011's crate at the spot");
        let pid = client.pid;
        let said = beside(
            engine,
            &mut client,
            spot,
            |c| op_loc1(c, crate_, spot),
            |e| !has(e, pid, clue),
        );
        assert!(
            !has(engine, client.pid, clue),
            "the clue is used; said {said:?}"
        );
        assert!(
            trail_item(engine, client.pid, clue).is_some(),
            "the trail goes on"
        );
    });
}

#[test]
fn a_289_chest_opens_and_its_search_moves_the_trail() {
    runner::on_loaded_engine(|engine| {
        // "Search through chests found in the upstairs of houses in eastern
        // Falador" -- vague013, at 289's own chest (375, op1 Open), which opens to 378
        // (op2 Search, op3 Shut).
        let spot = (1, 3041, 3364);
        let clue = id(engine, "obj", "trail_clue_easy_vague013");
        let (closed, open) = (
            id(engine, "loc", "chestclosed"),
            id(engine, "loc", "chestopen"),
        );
        let mut client = holder(
            engine,
            "chests",
            (1, 3041, 3362),
            &["trail_clue_easy_vague013"],
        );
        assert!(loc_there(engine, spot, closed), "289's chest at the spot");
        let said = beside(
            engine,
            &mut client,
            spot,
            |c| op_loc1(c, closed, spot),
            |e| loc_there(e, spot, open),
        );
        assert!(
            loc_there(engine, spot, open),
            "the chest opened; said {said:?}"
        );
        op_loc2(&mut client, open, spot);
        ticks(engine, 8);
        assert!(!has(engine, client.pid, clue), "the clue is used");
        assert!(
            trail_item(engine, client.pid, clue).is_some(),
            "the trail goes on"
        );
    });
}

#[test]
fn a_spade_on_a_map_clues_spot_digs_up_its_casket() {
    runner::on_loaded_engine(|engine| {
        // map004's spot is 0_40_54_52_26; digging there gives its casket.
        let clue = id(engine, "obj", "trail_clue_easy_map004");
        let casket = id(engine, "obj", "trail_clue_easy_map004_casket");
        let spade = id(engine, "obj", "spade");
        let mut client = holder(
            engine,
            "digger",
            (0, 40 * 64 + 52, 54 * 64 + 26),
            &["spade", "trail_clue_easy_map004"],
        );
        let mut p = ClientPacket::new();
        p.p4(BACKPACK).ip2_alt1(spade as i32).ip2(0);
        client.send(ClientProt::IfButton1, &p.finish());
        ticks(engine, 6);
        assert!(!has(engine, client.pid, clue), "the map is used");
        assert!(has(engine, client.pid, casket), "its casket");
    });
}
