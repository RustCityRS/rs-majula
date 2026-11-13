//! Doric's Quest as of May 2011, talked through as the client does. The lines
//! checked are the ones the 2011 recordings show word for word (2011-04-19
//! YouTube ycSbSwi-IPk, 2011-07-29 v7tHg6zuI_s): 2011's four-option menu,
//! Doric's pickaxe, the "Where can I find those?" choice, and the hand-in.
//! The reward is the 2011 wiki's (Doric's Quest, 2011-04-21): 1 quest point,
//! 180 coins, 1,300 Mining xp, and a bronze pickaxe when the quest starts.

#![cfg(since_647)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_test::runner;
use rs_test::talk::{converse, settexts, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

const MINING: usize = 14;

/// A player beside Doric, with the quest at `stage` and `objs` (name, count) carried.
fn visitor(engine: &mut Engine, stage: i32, objs: &[(&str, u32)]) -> (HarnessClient, u16) {
    engine.harness_reset();
    let mut client = engine
        .harness_login_returning("doric_helper")
        .expect("login");
    for _ in 0..10 {
        engine.cycle();
    }
    let doric = *engine
        .cache
        .npcs
        .debugnames
        .get("doric")
        .expect("npc doric");
    let (nid, x, z) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .find(|n| n.npc.uid.id() == doric)
        .map(|n| {
            (
                n.npc.uid.nid(),
                n.npc.pathing.coord.x(),
                n.npc.pathing.coord.z(),
            )
        })
        .expect("Doric is spawned");
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|&(dx, dz)| {
            rsmod::can_travel(0, x, z, dx, dz, 1, 0, CollisionType::Normal)
                && !rsmod::is_flagged(
                    x.wrapping_add_signed(dx as i16),
                    z.wrapping_add_signed(dz as i16),
                    0,
                    0x100 | 0x200000,
                )
        })
        .map(|(dx, dz)| {
            (
                x.wrapping_add_signed(dx as i16),
                z.wrapping_add_signed(dz as i16),
            )
        })
        .expect("a walkable tile beside Doric");
    engine.harness_telejump(client.pid, sx, 0, sz);
    let varp = *engine.cache.varps.debugnames.get("doricquest").unwrap();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let ids: Vec<(u16, u32)> = objs
        .iter()
        .map(|(o, n)| (*engine.cache.objs.debugnames.get(*o).unwrap(), *n))
        .collect();
    let player = engine.harness_player_mut(client.pid).unwrap();
    player.vars.set(varp, VarValue::Int(stage));
    let backpack = player.invs.get_mut(&inv).unwrap();
    let mut slot = 0u16;
    for (id, n) in ids {
        for _ in 0..n {
            backpack.set(slot, id, 1);
            slot += 1;
        }
    }
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    (client, nid)
}

fn stage(engine: &Engine, client: &HarnessClient) -> i32 {
    let varp = *engine.cache.varps.debugnames.get("doricquest").unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(varp)
        .as_int()
}

fn carried(engine: &Engine, client: &HarnessClient, obj: &str) -> i64 {
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let id = *engine.cache.objs.debugnames.get(obj).unwrap();
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap()
        .total(id) as i64
}

#[test]
fn doric_starts_the_quest_and_hands_over_a_bronze_pickaxe() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, 0, &[]);
        talk_to(&mut client, nid);
        let talk = converse(
            engine,
            &mut client,
            &[
                "I wanted to use your anvils.",
                "Yes, I will get you the materials.",
                "Certainly, I'll be right back!",
            ],
            90,
        );
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        for line in [
            "Hello traveller, what brings you to my humble smithy?",
            // 2011's menu: four options, the last "Ask about something else."
            "Ask about something else.",
            "My anvils get enough work with my own use. I make pickaxes, and it takes a lot of hard work. If you could get me some more materials, then I could let you use them.",
            "Clay is what I use more than anything, to make casts. Could you get me 6 clay, 4 copper ore, and 2 iron ore, please? I could pay a little, and let you use my anvils. Take this pickaxe with you just in case you need it.",
            "Where can I find those?",
        ] {
            assert!(talk.said(line), "2011 shows {line:?}; set {:?}", talk.texts);
        }
        assert!(
            !talk.said("What do you make here?"),
            "2010's top-level option: {:?}",
            talk.texts
        );
        assert_eq!(stage(engine, &client), 10, "Doric's Quest started");
        assert_eq!(
            carried(engine, &client, "bronze_pickaxe"),
            1,
            "Doric's pickaxe"
        );
    });
}

#[test]
fn doric_says_where_to_find_the_ores() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, 0, &[]);
        talk_to(&mut client, nid);
        let talk = converse(
            engine,
            &mut client,
            &[
                "I wanted to use your anvils.",
                "Yes, I will get you the materials.",
                "Where can I find those?",
            ],
            90,
        );
        assert!(talk.unanswered.is_none(), "{:?}", talk.unanswered);
        let line = "You'll be able to find all those ores in the rocks just inside the Dwarven Mine. Head east from here and you'll find the entrance in the side of Ice Mountain.";
        assert!(talk.said(line), "{:?}", talk.texts);
    });
}

#[test]
fn doric_waits_for_all_the_materials() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(engine, 10, &[("clay", 6)]);
        talk_to(&mut client, nid);
        let talk = converse(engine, &mut client, &["Certainly, I'll be right back!"], 60);
        assert!(
            talk.said("Have you got my materials yet, traveller?"),
            "{:?}",
            talk.texts
        );
        assert!(
            talk.said("Sorry, I don't have them all yet."),
            "{:?}",
            talk.texts
        );
        assert_eq!(stage(engine, &client), 10);
        assert_eq!(carried(engine, &client, "clay"), 6, "nothing taken");
    });
}

#[test]
fn doric_takes_the_ores_for_2011s_reward() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let (mut client, nid) = visitor(
            engine,
            10,
            &[("clay", 6), ("copper_ore", 4), ("iron_ore", 2)],
        );
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[MINING];
        talk_to(&mut client, nid);
        let mut talk = converse(engine, &mut client, &[], 60);
        for _ in 0..6 {
            engine.cycle();
            let packets = client.packets();
            talk.texts
                .extend(settexts(&packets).into_iter().map(|(_, t)| t));
        }
        for line in [
            "Have you got my materials yet, traveller?",
            "I have everything you need.",
            "Many thanks. Pass them here, please. I can spare you some coins for your trouble, and please use my anvils any time you want.",
            "You hand the clay, copper, and iron to Doric.",
            "You have completed Doric's Quest!",
            "1300 Mining XP",
            "180 coins",
            "Use of Doric's Anvils",
        ] {
            assert!(talk.said(line), "2011 shows {line:?}; set {:?}", talk.texts);
        }
        assert!(
            !talk.said("I have everything you need!"),
            "void-634's '!' is not 2011's"
        );
        assert_eq!(stage(engine, &client), 100, "Doric's Quest complete");
        assert_eq!(carried(engine, &client, "coins"), 180);
        for ore in ["clay", "copper_ore", "iron_ore"] {
            assert_eq!(carried(engine, &client, ore), 0, "{ore} handed over");
        }
        let gained = engine.harness_player(client.pid).unwrap().stats.xp[MINING] - xp;
        assert_eq!(gained, 13000, "1,300 Mining xp (tenths)");
    });
}
