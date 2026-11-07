//! Fishing from 289, at a 2011 spot.
//!
//! 289's spots are 2005's; the small-net spot on Lumbridge's rebuilt shore
//! (npc 4908) is 2011's, given 289's `saltfish` category because its ops and
//! its catch (void-634) are a saltfish spot's. The case clicks Net on it as
//! the client does (OPNPC1, `Statics1.do_action`: `p1_alt3(ctrl) p2(nid)`)
//! with a small fishing net in the backpack, and fishes until a shrimp comes
//! up. The catch's numbers come from 289's `fishing_struct_shrimps`, a
//! server-only struct, which only loads since structs past the cache's range
//! are appended at startup.
//!
//! 289's karambwan fishing (tbwt.rs2) at 2011's Karamja spots: the
//! karambwanji spot nets karambwanji or shrimps; a karambwan spot refuses a
//! player Lubufu has not taught, and takes the loaded vessel from one he has.
//! And 289's Fishing Guild master fisher turns away a player below 68.
//!
//! The PR's 289: its spots stand still (`wanderrange=0`) and hold their
//! fishers out to `maxrange` 50 (20 at the Fishing Contest and the lava eels),
//! the master fisher keeps to 3; a low Harpoon says "Harpoon" capitalised; and
//! Ape Atoll's two spots (2011's 1405 and 1406, 289's 0_43_42_memberfish and
//! 0_42_42_memberfish) move between 289's two tiles each.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const FISHING: usize = 10;
/// 2011's xp counter (varp 1801): `~givexp` adds ten to it for each xp it pays.
const XP_COUNTER: u16 = 1801;
const SPOT: u16 = 4908;
const KARAMBWANJI: u16 = 1174;
const KARAMBWAN: u16 = 1177;
const MASTER_FISHER: u16 = 308;
/// The Fishing Guild's Cage/Harpoon spot, 289's `0_40_53_rarefish`.
const GUILD_RAREFISH: u16 = 312;
/// Ape Atoll's spots and the one other tile of each one's 289 list (786's map
/// spawns each on the list's other tile).
const APE_ATOLL: [(u16, (u16, u16)); 2] = [(1405, (2771, 2733)), (1406, (2707, 2698))];

// The cases share one loaded engine and each starts with a reset, which would
// log the others' fishers out: one at a time.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn a_2011_spot_gives_shrimps() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let net = *engine
            .cache
            .objs
            .debugnames
            .get("small_fishing_net")
            .unwrap();
        let shrimps = *engine.cache.objs.debugnames.get("raw_shrimps").unwrap();
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();

        engine.harness_reset();
        let (nid, x, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .filter(|n| n.npc.uid.id() == SPOT)
            .map(|n| (n.npc.uid.nid(), n.npc.pathing.coord))
            .map(|(nid, c)| (nid, c.x(), c.z()))
            .next()
            .expect("Lumbridge's small-net spot is spawned");
        let mut client = engine.harness_login_returning("fisher").expect("login");
        engine.cycle();
        engine.harness_telejump(client.pid, x, 0, z + 2);
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&inv).unwrap().set(0, net, 1);
        let xp = player.stats.xp[FISHING];
        let counter = player.vars.get(XP_COUNTER).as_int();
        engine.cycle();
        client.packets();

        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2(nid as i32);
        client.send(ClientProt::OpNpc1, &p.finish());
        let mut said = Vec::new();
        for _ in 0..150 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            let player = engine.harness_player(client.pid).unwrap();
            if player.invs.get(&inv).unwrap().total(shrimps) > 0 {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.invs.get(&inv).unwrap().total(shrimps),
            1,
            "one raw shrimp; the game said {said:?}"
        );
        assert_eq!(player.stats.xp[FISHING] - xp, 100, "shrimps' productexp");
        assert_eq!(
            player.vars.get(XP_COUNTER).as_int() - counter,
            1000,
            "2011's xp counter counts the shrimps"
        );
        assert_eq!(
            said.first().map(String::as_str),
            Some("You cast out your net...")
        );
        assert_eq!(
            said.last().map(String::as_str),
            Some("You catch some shrimps.")
        );
    });
}

/// The spawned npc `id` nearest `near`: its nid and tile.
fn spot(engine: &Engine, id: u16, near: (u16, u16)) -> (u16, u16, u16) {
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == id)
        .map(|n| (n.npc.uid.nid(), n.npc.pathing.coord))
        .map(|(nid, c)| (nid, c.x(), c.z()))
        .min_by_key(|&(_, x, z)| x.abs_diff(near.0) as u32 + z.abs_diff(near.1) as u32)
        .unwrap_or_else(|| panic!("npc {id} is spawned"))
}

/// A player beside `(x, z)` at Fishing `level`, holding `held`, its login
/// packets drained.
fn fisher(
    engine: &mut Engine,
    name: &str,
    (x, z): (u16, u16),
    level: u16,
    held: &[&str],
) -> HarnessClient {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let held: Vec<u16> = held
        .iter()
        .map(|o| *engine.cache.objs.debugnames.get(*o).unwrap())
        .collect();
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    let (sx, sz) = (1..=3i32)
        .flat_map(|r| (-r..=r).flat_map(move |dx| (-r..=r).map(move |dz| (dx, dz))))
        .map(|(dx, dz)| {
            (
                x.wrapping_add_signed(dx as i16),
                z.wrapping_add_signed(dz as i16),
            )
        })
        .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
        .expect("a walkable tile beside the spot");
    engine.harness_telejump(client.pid, sx, 0, sz);
    let player = engine.harness_player_mut(client.pid).unwrap();
    for (slot, obj) in (0u16..).zip(held.iter()) {
        player.invs.get_mut(&inv).unwrap().set(slot, *obj, 1);
    }
    player.stats.levels[FISHING] = level;
    player.stats.base_levels[FISHING] = level;
    engine.cycle();
    client.packets();
    client
}

fn op_npc1(client: &mut HarnessClient, nid: u16) {
    let mut p = ClientPacket::new();
    p.p1_alt3(0).p2(nid as i32);
    client.send(ClientProt::OpNpc1, &p.finish());
}

/// Every IF_SETTEXT's text in `packets`: what a mesbox or chat line says.
fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    let opcode = ServerProt::IfSetText as u16;
    packets
        .iter()
        .filter(|(op, _)| *op == opcode)
        .map(|(_, p)| {
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            String::from_utf8_lossy(&p[4..end]).into_owned()
        })
        .collect()
}

/// Cycles up to `n` ticks, or until `done`: the game messages and the texts set.
fn run(
    engine: &mut Engine,
    client: &mut HarnessClient,
    n: usize,
    done: impl Fn(&Engine) -> bool,
) -> (Vec<String>, Vec<String>) {
    let (mut said, mut texts) = (Vec::new(), Vec::new());
    for _ in 0..n {
        engine.cycle();
        let packets = client.packets();
        said.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
        texts.extend(settexts(&packets));
        if done(engine) {
            break;
        }
    }
    (said, texts)
}

/// 289's tbwt.rs2 at 2011's Karamja spots: npc 1174 nets karambwanji (or
/// shrimps), as 289's `category_632`.
#[test]
fn a_karambwanji_spot_nets_karambwanji_or_shrimps() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let karambwanji = *engine.cache.objs.debugnames.get("raw_karambwanji").unwrap();
        let shrimps = *engine.cache.objs.debugnames.get("raw_shrimps").unwrap();
        let (nid, x, z) = spot(engine, KARAMBWANJI, (2791, 3019));
        let mut client = fisher(engine, "netter", (x, z), 5, &["small_fishing_net"]);
        let pid = client.pid;
        let xp = engine.harness_player(pid).unwrap().stats.xp[FISHING];
        op_npc1(&mut client, nid);
        let caught = |e: &Engine| {
            let b = e.harness_player(pid).unwrap().invs.get(&inv).unwrap();
            b.total(karambwanji) + b.total(shrimps) > 0
        };
        let (said, _) = run(engine, &mut client, 200, caught);
        let player = engine.harness_player(pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(
            said.first().map(String::as_str),
            Some("You cast out your net..."),
            "{said:?}"
        );
        if backpack.total(karambwanji) == 1 {
            assert_eq!(player.stats.xp[FISHING] - xp, 50);
            assert_eq!(
                said.last().map(String::as_str),
                Some("You catch a Karambwanji.")
            );
        } else {
            assert_eq!(
                backpack.total(shrimps),
                1,
                "a catch; the game said {said:?}"
            );
            assert_eq!(player.stats.xp[FISHING] - xp, 100);
            assert_eq!(
                said.last().map(String::as_str),
                Some("You catch some shrimp.")
            );
        }
    });
}

/// A karambwan spot (npc 1177) turns a player away until Lubufu has taught
/// the vessel (Tai Bwo Wannai Trio, not ported: 289's `%tbwt_lubufu`).
#[test]
fn a_karambwan_spot_wants_lubufus_teaching() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (nid, x, z) = spot(engine, KARAMBWAN, (2899, 3119));
        let mut client = fisher(
            engine,
            "vesseller",
            (x, z),
            65,
            &["tbwt_karambwan_vessel_loaded_with_karambwanji"],
        );
        op_npc1(&mut client, nid);
        let (said, texts) = run(engine, &mut client, 20, |_| false);
        assert!(
            texts
                .iter()
                .any(|t| t == "You don't even know how to use the vessel!"),
            "the mesbox; texts {texts:?}, said {said:?}"
        );
    });
}

/// Once taught, the loaded vessel goes down and comes up five ticks later with
/// a karambwan or without its bait, empty either way.
#[test]
fn a_taught_player_lowers_the_karambwan_vessel() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let empty = *engine
            .cache
            .objs
            .debugnames
            .get("karambwan_vessel")
            .unwrap();
        let karambwan = *engine.cache.objs.debugnames.get("raw_karambwan").unwrap();
        let (nid, x, z) = spot(engine, KARAMBWAN, (2899, 3119));
        let mut client = fisher(
            engine,
            "apprentice",
            (x, z),
            65,
            &["tbwt_karambwan_vessel_loaded_with_karambwanji"],
        );
        let pid = client.pid;
        let lubufu = *engine.cache.varps.debugnames.get("tbwt_lubufu").unwrap();
        engine
            .harness_player_mut(pid)
            .unwrap()
            .vars
            .set(lubufu, rs_pack::cache::VarValue::Int(31));
        op_npc1(&mut client, nid);
        let emptied = |e: &Engine| {
            e.harness_player(pid)
                .unwrap()
                .invs
                .get(&inv)
                .unwrap()
                .total(empty)
                > 0
        };
        let (said, _) = run(engine, &mut client, 30, emptied);
        let backpack = engine.harness_player(pid).unwrap().invs.get(&inv).unwrap();
        assert_eq!(
            backpack.total(empty),
            1,
            "the vessel comes back; said {said:?}"
        );
        assert_eq!(
            said.first().map(String::as_str),
            Some("You lower the Karambwan vessel.")
        );
        let last = said.last().map(String::as_str);
        if backpack.total(karambwan) == 1 {
            assert_eq!(last, Some("You catch a Karambwan!"));
        } else {
            assert_eq!(
                last,
                Some("A Karambwan deftly snatches the Karambwanji from your vessel!")
            );
        }
    });
}

/// 289's Fishing Guild master fisher, at 2011's guild: below 68 he turns the
/// player away.
#[test]
fn the_master_fisher_turns_away_a_low_level() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (nid, x, z) = spot(engine, MASTER_FISHER, (2613, 3394));
        let mut client = fisher(engine, "hopeful", (x, z), 1, &[]);
        op_npc1(&mut client, nid);
        let (_, texts) = run(engine, &mut client, 20, |_| false);
        assert!(
            texts
                .iter()
                .any(|t| t.contains("only the top fishers are allowed")),
            "texts {texts:?}"
        );
    });
}

/// The PR's 289 gives every spot `wanderrange=0` and a `maxrange` (50; 20 at
/// the Fishing Contest and the lava eels), and the master fisher `maxrange=3`.
#[test]
fn spots_and_the_master_fisher_take_289s_ranges() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for (name, wander, max) in [
            ("0_40_53_rarefish", 0, 50),
            ("0_41_57_memberfish", 0, 50),
            ("fishing_spot_1405", 0, 50),
            ("fishing_spot", 0, 20),
            ("master_fisher", 3, 3),
        ] {
            let id = *engine.cache.npcs.debugnames.get(name).expect(name);
            let t = engine.cache.npcs.get_by_id(id).expect(name);
            assert_eq!((t.wanderrange, t.maxrange), (wander, max), "{name}");
        }
        let lava = *engine
            .cache
            .npcs
            .debugnames
            .get("0_45_152_lavafish")
            .unwrap();
        assert_eq!(engine.cache.npcs.get_by_id(lava).unwrap().maxrange, 20);
    });
}

/// Below 35, a Harpoon at a Cage/Harpoon spot is refused with 289's
/// capitalised "Harpoon".
#[test]
fn a_low_harpoon_says_harpoon() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        // The guild's spots make their first move (timer=1) in the first ticks;
        // the next is 280 or more ticks away.
        for _ in 0..10 {
            engine.cycle();
        }
        let (nid, x, z) = spot(engine, GUILD_RAREFISH, (2602, 3415));
        let mut client = fisher(engine, "lowharpoon", (x, z), 1, &["harpoon"]);
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt2(nid as i32);
        client.send(ClientProt::OpNpc3, &p.finish());
        let (said, texts) = run(engine, &mut client, 20, |_| false);
        assert!(
            texts
                .iter()
                .any(|t| t == "You need at least 35 Fishing to Harpoon these fish."),
            "the mesbox; texts {texts:?}, said {said:?}"
        );
    });
}

/// Ape Atoll's spots run 289's timer and lists: the first move takes each to
/// its list's other tile, the one it was not spawned on.
#[test]
fn ape_atoll_spots_move_between_289s_tiles() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        for _ in 0..20 {
            engine.cycle();
        }
        for (id, to) in APE_ATOLL {
            let (_, x, z) = spot(engine, id, to);
            assert_eq!((x, z), to, "npc {id} moves to its other tile");
        }
    });
}
