//! Runecrafting from 289, at 2011's ruins, altars and portals.
//!
//! 289's runecraft keys every temple on a `runecraft_table` row: the ruins a
//! talisman opens, the tile inside, the altar's rune and the way back out.
//! 2011 moved some of them -- the air ruins stand west of Varrock by the
//! bridge to Gunnarsgrunn instead of south of Falador (2011 wiki, Air altar:
//! moved 17 September 2009) -- so the rows carry 2011's tiles, from
//! the 2011 map's placements and `void-634`. The cases use a talisman on the
//! air ruins as the client does (OPLOCT), bind essence at the air altar
//! (OPLOC1), and step back out through the temple's portal. The essence mine's
//! portal, a loc at 289, is one of four npcs in 2011 (OPNPC1).

#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_ver::TypeId;

const RUNECRAFT: usize = 20;

/// 2011's air ruins (`airtemple_ruined`, 3x3), west of Varrock; 530's are 289's, south of
/// Falador.
#[cfg(before_647)]
const AIR_RUINS: (u16, u16, u16) = (2452, 2984, 3291);
#[cfg(since_647)]
const AIR_RUINS: (u16, u16, u16) = (2452, 3126, 3404);
/// The air altar (`air_altar`, 3x3) and the temple's portal out.
const AIR_ALTAR: (u16, u16, u16) = (2478, 2843, 4833);
const AIR_PORTAL: (u16, u16, u16) = (2465, 2841, 4828);
/// Where `runecraft_air` puts a player: inside the temple, and back outside.
const AIR_INSIDE: (u16, u16) = (2841, 4830);
#[cfg(before_647)]
const AIR_OUTSIDE: (u16, u16) = (2983, 3288);
#[cfg(since_647)]
const AIR_OUTSIDE: (u16, u16) = (3129, 3406);

/// The four corners of the essence mine, where 2011 leaves 2492's snow.
#[cfg(since_647)]
const MINE_PORTALS: [(u16, u16); 4] = [(2885, 4850), (2889, 4813), (2932, 4854), (2933, 4815)];
/// `^essence_mine_to_sedridor`, the way back for a player with no other.
#[cfg(since_647)]
const SEDRIDOR: (u16, u16) = (3106, 9572);

fn obj(engine: &Engine, name: &str) -> TypeId {
    engine
        .cache
        .objs
        .get_by_debugname(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
        .id
}

fn inv(engine: &Engine) -> TypeId {
    engine.cache.invs.get_by_debugname("inv").unwrap().id
}

/// A fresh world with one player logged in and standing at `x, z`.
fn player_at(engine: &mut Engine, name: &str, x: u16, z: u16) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine.harness_login_returning(name).expect("login");
    engine.cycle();
    engine.harness_telejump(client.pid, x, 0, z);
    engine.cycle();
    client.packets();
    client
}

fn coord(engine: &Engine, pid: u16) -> (u16, u16) {
    let c = engine.harness_player(pid).unwrap().pathing.coord;
    (c.x(), c.z())
}

/// Runs ticks until the player stands somewhere else, collecting what the
/// game says on the way.
fn until_moved_to(
    engine: &mut Engine,
    client: &mut HarnessClient,
    want: (u16, u16),
) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..30 {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
        if coord(engine, client.pid) == want {
            return said;
        }
    }
    panic!(
        "never reached {want:?}; at {:?}, the game said {said:?}",
        coord(engine, client.pid)
    );
}

fn oploc1(client: &mut HarnessClient, (loc, x, z): (u16, u16, u16)) {
    client.op_loc(1, x, z, TypeId::from_int(loc as i32));
}

#[test]
fn runecrafting_at_2011s_air_temple() {
    runner::on_loaded_engine(|engine| {
        let talisman = obj(engine, "air_talisman");
        let essence = obj(engine, "blankrune");
        let air = obj(engine, "airrune");
        let inv = inv(engine);

        // The talisman, held to 2011's air ruins, takes the player inside.
        // Beside the ruins already: Barbarian Village's npcs wander onto the approach,
        // which made the walk there flaky.
        let mut client = player_at(engine, "runecrafter", AIR_RUINS.1 + 1, AIR_RUINS.2 - 1);
        let player = engine.harness_player_mut(client.pid).unwrap();
        let backpack = player.invs.get_mut(&inv).unwrap();
        backpack.set(0, talisman, 1);
        backpack.set(1, essence, 5);
        let xp = player.stats.xp[RUNECRAFT];

        let (loc, x, z) = AIR_RUINS;
        client.op_loc_t(x, z, TypeId::from_int(loc as i32), BACKPACK, 0, talisman);
        let said = until_moved_to(engine, &mut client, AIR_INSIDE);
        assert_eq!(
            said,
            [
                "You hold the Air Talisman towards the mysterious ruins.",
                "You feel a powerful force take hold of you...",
            ]
        );

        // Five essence bind into five air runes at level 1.
        oploc1(&mut client, AIR_ALTAR);
        let mut said = Vec::new();
        for _ in 0..20 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            let player = engine.harness_player(client.pid).unwrap();
            if player.invs.get(&inv).unwrap().total(air) > 0 {
                break;
            }
        }
        let player = engine.harness_player(client.pid).unwrap();
        let backpack = player.invs.get(&inv).unwrap();
        assert_eq!(backpack.total(air), 5, "the game said {said:?}");
        assert_eq!(backpack.total(essence), 0);
        assert_eq!(player.stats.xp[RUNECRAFT] - xp, 250, "5 xp an essence");
        // Entering the temple's square also unlocks its track (Serene), from
        // 2011's area music (music/scripts/move.rs2), which this case doesn't check.
        said.retain(|m| !m.contains("You have unlocked a new music track"));
        assert_eq!(said, ["You bind the temple's power into Air Runes."]);

        // The portal leads back out beside 2011's ruins, once the binding's
        // delay is over.
        for _ in 0..4 {
            engine.cycle();
        }
        client.packets();
        oploc1(&mut client, AIR_PORTAL);
        let said = until_moved_to(engine, &mut client, AIR_OUTSIDE);
        assert_eq!(said, ["You step through the portal..."]);
    });
}

#[cfg(since_647)]
#[test]
fn the_essence_mine_portals_are_2011s_npcs() {
    runner::on_loaded_engine(|engine| {
        let portal = engine
            .cache
            .npcs
            .get_by_debugname("essence_mine_exit_1")
            .expect("the first portal npc")
            .id;
        let mut client = player_at(engine, "miner_of_essence", 2932, 4816);

        let spawned: Vec<(TypeId, u16, u16)> = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .map(|n| (n.npc.uid.id(), n.npc.pathing.coord))
            .filter(|&(id, _)| (portal.raw()..portal.raw() + 4).contains(&id.raw()))
            .map(|(id, c)| (id, c.x(), c.z()))
            .collect();
        let mut corners: Vec<(u16, u16)> = spawned.iter().map(|&(_, x, z)| (x, z)).collect();
        corners.sort();
        assert_eq!(corners, MINE_PORTALS, "one portal at each corner");

        let nid = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .find(|n| n.npc.pathing.coord.x() == 2933 && n.npc.pathing.coord.z() == 4815)
            .map(|n| n.npc.uid.nid())
            .unwrap();
        client.op_npc(1, nid);
        let mut said = Vec::new();
        for _ in 0..30 {
            engine.cycle();
            said.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
            let (x, z) = coord(engine, client.pid);
            if x.abs_diff(SEDRIDOR.0) <= 2 && z.abs_diff(SEDRIDOR.1) <= 2 {
                assert_eq!(said, ["You step through the portal..."]);
                return;
            }
        }
        panic!(
            "never left the mine; at {:?}, the game said {said:?}",
            coord(engine, client.pid)
        );
    });
}
