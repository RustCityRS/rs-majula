//! 289's Varrock at 647 (`areas/area_varrock`): every way through its
//! shopkeepers' and townsfolk's Talk-to says something and fails no script.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_test::runner;
use rs_test::talk::{explore, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: Mutex<()> = Mutex::new(());

/// A spawn of npc `name`: its nid, and a tile beside it that one step from
/// the npc reaches (npcs wander).
fn beside(engine: &Engine, name: &str) -> (u16, u16, u8, u16) {
    let id = *engine
        .cache
        .npcs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("npc.pack names {name}"));
    let (nid, nx, y, nz) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == id)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.y(), c.z())
        })
        .unwrap_or_else(|| panic!("{name} is spawned"));
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|&(dx, dz)| {
            rsmod::can_travel(y, nx, nz, dx, dz, 1, 0, CollisionType::Normal)
                && !rsmod::is_flagged(
                    nx.wrapping_add_signed(dx as i16),
                    nz.wrapping_add_signed(dz as i16),
                    y,
                    0x100 | 0x200000,
                )
        })
        .map(|(dx, dz)| {
            (
                nx.wrapping_add_signed(dx as i16),
                nz.wrapping_add_signed(dz as i16),
            )
        })
        .unwrap_or_else(|| panic!("a tile beside {name} at {nx},{nz}"));
    (nid, sx, y, sz)
}

#[test]
fn varrock_s_npcs_answer_every_branch() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for npc in [
            "varrock_rune_seller",
            "baraek",
            "tailorp",
            "guildmaster",
            "horvik_the_armourer",
            "kanel",
            "katrine",
            "lowe",
            "philop",
            "scavvo",
            "swordshop1",
            "swordshop2",
            "straven",
            "tea_seller",
            "tramppg",
            "valaine",
            "weaponsmaster",
            "zaff",
        ] {
            let _ = engine.script_errors.take();
            let branches = explore(
                engine,
                |engine| {
                    engine.harness_reset();
                    let mut client: HarnessClient = engine
                        .harness_login_returning("varrock_visitor")
                        .expect("login");
                    for _ in 0..10 {
                        engine.cycle();
                    }
                    let (nid, x, y, z) = beside(engine, npc);
                    engine.harness_telejump(client.pid, x, y, z);
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
            assert!(errors.is_empty(), "{npc}: {errors:?}");
            assert!(!branches.is_empty(), "{npc}");
            for branch in &branches {
                // A box, or a game message: Kanel only says "The boy is busy
                // playing." (289's).
                assert!(
                    !branch.conversation.texts.is_empty()
                        || !branch.conversation.messages.is_empty(),
                    "{npc}, path {:?}: nothing said ({:?})",
                    branch.path,
                    branch.conversation.messages
                );
            }
        }
    });
}
