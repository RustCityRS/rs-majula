//! 289's Lumbridge npcs at 647 (areas/area_lumbridge/scripts): every way
//! through each one's Talk-to answers, with no script error, at each stage of
//! the quest it depends on.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_test::runner;
use rs_test::talk::{explore, talk_to};

/// The nearest spawn of npc `name` to (x, z): its nid, and a walkable tile
/// beside it.
fn beside(engine: &Engine, name: &str) -> (u16, u16, u16) {
    let id = *engine
        .cache
        .npcs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("npc.pack names {name}"));
    let (nid, x, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .unwrap_or_else(|| panic!("{name} is spawned"));
    let (sx, sz) = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1)]
        .into_iter()
        .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
        .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
        .unwrap_or_else(|| panic!("a walkable tile beside {name}"));
    (nid, sx, sz)
}

/// Explores every branch of `npc`'s Talk-to with varp `varp` at `value`, and
/// checks each said something and no script failed.
fn every_branch(engine: &mut Engine, npc: &str, varp: &str, value: i32) {
    let varp_id = *engine.cache.varps.debugnames.get(varp).unwrap();
    let _ = engine.script_errors.take();
    let branches = explore(
        engine,
        |engine| {
            engine.harness_reset();
            let mut client: HarnessClient = engine
                .harness_login_returning("lumbridge_visitor")
                .expect("login");
            for _ in 0..10 {
                engine.cycle();
            }
            let (nid, x, z) = beside(engine, npc);
            engine.harness_telejump(client.pid, x, 0, z);
            engine
                .harness_player_mut(client.pid)
                .unwrap()
                .vars
                .set(varp_id, VarValue::Int(value));
            for _ in 0..3 {
                engine.cycle();
            }
            client.packets();
            talk_to(&mut client, nid);
            client
        },
        24,
    );
    let errors: Vec<String> = engine
        .script_errors
        .take()
        .into_iter()
        .map(|e| format!("{e:?}"))
        .collect();
    assert!(errors.is_empty(), "{npc} at %{varp}={value}: {errors:?}");
    assert!(!branches.is_empty());
    for branch in &branches {
        assert!(
            !branch.conversation.texts.is_empty(),
            "{npc} at %{varp}={value}, path {:?}: nothing said",
            branch.path
        );
    }
}

/// The Lost City adventurers by the Lumbridge Swamp, before, during and after
/// the quest (289's archer, warrior, monk and wizard; %zanaris 0, 1, 6).
#[test]
fn the_lost_city_adventurers_answer_every_branch() {
    runner::on_loaded_engine(|engine| {
        for npc in [
            "archeradventurerpg",
            "warrioradventurerpg",
            "monkadventurerpg",
            "wizardadventuterpg",
        ] {
            for stage in [0, 1, 6] {
                every_branch(engine, npc, "zanaris", stage);
            }
        }
    });
}
