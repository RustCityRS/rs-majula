//! Lumbridge's npcs that stand still at 647 stay on their spawn tiles while the
//! world runs, and its ducks, goblins and rats still wander.
//!
//! Bob and the general store's keeper are walk mode 1 in cache 786 (npc opcode
//! 119; void-634's `ModeType.EMPTY`, an npc that does not move), and 289 gives
//! Bob and the castle's cook `wanderrange=0`. Before their configs said so,
//! they took rs-pack's default `wanderrange` of 5 and walked about.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_test::runner;

fn spawns(engine: &Engine, name: &str, near: (u16, u16), within: u16) -> Vec<(u16, (u16, u16))> {
    let id = *engine
        .cache
        .npcs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("npc.pack names {name}"));
    engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == id)
        .map(|n| {
            (
                n.npc.uid.nid(),
                (n.npc.pathing.coord.x(), n.npc.pathing.coord.z()),
            )
        })
        .filter(|&(_, (x, z))| x.abs_diff(near.0) <= within && z.abs_diff(near.1) <= within)
        .collect()
}

fn at(engine: &Engine, nid: u16) -> (u16, u16) {
    let n = engine.npc_list.npcs[nid as usize]
        .as_ref()
        .expect("still spawned");
    (n.npc.pathing.coord.x(), n.npc.pathing.coord.z())
}

#[test]
fn lumbridge_s_keepers_stand_and_its_wanderers_wander() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let client = engine.harness_login_returning("wanderer").expect("login");
        // Lumbridge, between the general store, Bob's axes and the castle.
        let centre = (3222, 3222);
        engine.harness_telejump(client.pid, centre.0, 0, centre.1);
        engine.cycle();
        let mut still = Vec::new();
        for name in ["bob", "shopkeeper", "cook"] {
            let found = spawns(engine, name, centre, 32);
            assert!(!found.is_empty(), "{name} spawns in Lumbridge");
            still.extend(found.into_iter().map(|(nid, c)| (name, nid, c)));
        }
        // Lumbridge's wanderers at 647: its ducks (wanderrange 35), goblins and rats (15).
        let mut walkers = Vec::new();
        for name in ["duck", "goblin_12357", "rat"] {
            walkers.extend(spawns(engine, name, centre, 40));
        }
        assert!(
            !walkers.is_empty(),
            "ducks, goblins or rats spawn by Lumbridge"
        );
        let mut moved = 0;
        for _ in 0..300 {
            engine.cycle();
            for &(name, nid, spawn) in &still {
                assert_eq!(
                    at(engine, nid),
                    spawn,
                    "{name} (nid {nid}) left its spawn tile"
                );
            }
        }
        for &(nid, spawn) in &walkers {
            if at(engine, nid) != spawn {
                moved += 1;
            }
        }
        assert!(
            moved > 0,
            "no duck, goblin or rat moved in 300 ticks: npcs do not wander at all"
        );
    });
}
