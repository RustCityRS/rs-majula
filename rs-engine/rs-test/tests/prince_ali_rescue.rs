//! Prince Ali Rescue at 647 (`quests/quest_prince`, `areas/area_draynor`,
//! `areas/area_alkharid`): every way through each of its npcs' Talk-to at the
//! stages that change what they say, and Aggie's 2011 skin paste, which takes
//! no flour (2011 wiki, Skin paste).

#![cfg(since_647)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_test::runner;
use rs_test::talk::{converse, explore, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn varp(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .varps
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no varp {name}"))
}

/// A spawn of npc `name`: its nid, and a tile beside it that one step from
/// the npc reaches, so no wall stands between (npcs wander).
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

/// Logs in beside `npc` with %princequest at `stage` and %prince_keystatus at
/// `key`, and talks to it.
fn visit(engine: &mut Engine, npc: &str, stage: i32, key: i32) -> HarnessClient {
    let quest = varp(engine, "princequest");
    let keystatus = varp(engine, "prince_keystatus");
    engine.harness_reset();
    let mut client: HarnessClient = engine
        .harness_login_returning("ali_visitor")
        .expect("login");
    for _ in 0..10 {
        engine.cycle();
    }
    let (nid, x, y, z) = beside(engine, npc);
    engine.harness_telejump(client.pid, x, y, z);
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.vars.set(quest, VarValue::Int(stage));
    player.vars.set(keystatus, VarValue::Int(key));
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    talk_to(&mut client, nid);
    client
}

/// Explores every branch of `npc`'s Talk-to at `stage`, and checks each said
/// something and no script failed.
fn every_branch(engine: &mut Engine, npc: &str, stage: i32, key: i32) {
    let _ = engine.script_errors.take();
    let branches = explore(engine, |engine| visit(engine, npc, stage, key), 48);
    let errors: Vec<String> = engine
        .script_errors
        .take()
        .into_iter()
        .map(|e| format!("{e:?}"))
        .collect();
    assert!(
        errors.is_empty(),
        "{npc} at %princequest={stage}: {errors:?}"
    );
    assert!(!branches.is_empty());
    for branch in &branches {
        assert!(
            !branch.conversation.texts.is_empty(),
            "{npc} at %princequest={stage}, path {:?}: nothing said ({:?})",
            branch.path,
            branch.conversation.messages
        );
    }
}

/// Each npc before the quest, at the stages its lines turn on, and after it.
#[test]
fn every_prince_ali_rescue_npc_answers_every_branch() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for (npc, stages) in [
            ("hassan", &[0, 10, 20, 30, 110][..]),
            // Osman is 2011's multi-npc 5282, shown as osman (924).
            ("npc_5282", &[0, 10, 20, 30, 110]),
            ("leela", &[0, 20, 30, 40, 110]),
            ("ned", &[0, 20, 110]),
            ("aggie", &[0, 20]),
            ("joe_916", &[0, 30, 40]),
            // Once tied up (50 on), Keli only shouts over her head and sets her
            // guard on the player (2011 wiki, Prince Ali Rescue): no dialogue.
            ("lady_keli", &[0, 20, 30]),
            ("prince_ali", &[0, 50]),
            ("jailguard", &[0]),
            ("border_guard", &[0, 110]),
        ] {
            for &stage in stages {
                every_branch(engine, npc, stage, 0);
            }
        }
        // Osman and Leela with the key made, and claimed.
        for key in [1, 2] {
            every_branch(engine, "npc_5282", 20, key);
            every_branch(engine, "leela", 20, key);
        }
    });
}

/// 2011's skin paste is ash, water and redberries: Aggie makes it with no
/// pot of flour (2011 wiki, Skin paste and Aggie).
#[test]
fn aggie_mixes_skin_paste_without_flour() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let backpack = *engine.cache.invs.debugnames.get("inv").unwrap();
        let (ashes, berries, water, paste) = (
            obj(engine, "ashes"),
            obj(engine, "redberries"),
            obj(engine, "bucket_of_water"),
            obj(engine, "paste"),
        );
        let _ = engine.script_errors.take();
        let mut client = visit(engine, "aggie", 20, 0);
        let player = engine.harness_player_mut(client.pid).unwrap();
        let pack = player.invs.get_mut(&backpack).unwrap();
        pack.set(0, ashes, 1);
        pack.set(1, berries, 1);
        pack.set(2, water, 1);
        let talk = converse(
            engine,
            &mut client,
            &[
                "Could you think of a way to make skin paste?",
                "Yes please. Mix me some skin paste.",
            ],
            300,
        );
        assert_eq!(talk.unanswered, None, "{:?}", talk.texts);
        assert!(
            talk.said("You hand the ash, water and redberries to Aggie."),
            "{:?}",
            talk.texts
        );
        let pack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&backpack)
            .unwrap();
        assert_eq!(pack.total(paste), 1);
        for o in [ashes, berries, water] {
            assert_eq!(pack.total(o), 0, "Aggie keeps {o}");
        }
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}
