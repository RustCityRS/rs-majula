//! Sheep Shearer at 2011 (`areas/area_lumbridge/scripts/fred_the_farmer.rs2`,
//! `quests/quest_sheep`): Fred the Farmer's job of twenty balls of black wool
//! (2011 wiki, Sheep Shearer), from taking it to his pay, and every way through
//! his Talk-to at each stage.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_inv::{Inventory, StackMode};
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{Conversation, converse, explore, talk_to};
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::sync::Mutex;

const CRAFTING: usize = 12;
const BACKPACK: i32 = 149 << 16;
const SHEEP: u16 = 179;
/// Lumbridge Castle's spinning wheel, first floor, and the tile used from.
const WHEEL: (u16, u16, u16) = (36970, 3209, 3212);
const AT_WHEEL: (u16, u8, u16) = (3209, 1, 3213);

/// Every engine in this binary shares rs-pathfinder's process-static
/// collision map: the cases take turns.
static TURN: Mutex<()> = Mutex::new(());

fn obj(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .objs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no obj {name}"))
}

fn npc(engine: &Engine, name: &str) -> u16 {
    *engine
        .cache
        .npcs
        .debugnames
        .get(name)
        .unwrap_or_else(|| panic!("no npc {name}"))
}

fn inv(engine: &Engine, name: &str) -> u16 {
    *engine.cache.invs.debugnames.get(name).unwrap()
}

fn varp(engine: &Engine, name: &str) -> u16 {
    *engine.cache.varps.debugnames.get(name).unwrap()
}

/// The npc of type `kind` nearest (x, z): its nid, and a tile beside it that
/// one step from the npc reaches, so no wall stands between (npcs wander).
fn beside(engine: &Engine, kind: u16, (x, z): (u16, u16)) -> (u16, u16, u16) {
    let (nid, nx, nz) = engine
        .npc_list
        .npcs
        .iter()
        .flatten()
        .filter(|n| n.npc.uid.id() == kind)
        .map(|n| {
            let c = n.npc.pathing.coord;
            (n.npc.uid.nid(), c.x(), c.z())
        })
        .min_by_key(|&(_, nx, nz)| (nx as i32 - x as i32).abs() + (nz as i32 - z as i32).abs())
        .unwrap_or_else(|| panic!("npc {kind} is spawned"));
    let (sx, sz) = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|&(dx, dz)| {
            rsmod::can_travel(0, nx, nz, dx, dz, 1, 0, CollisionType::Normal)
                && !rsmod::is_flagged(
                    nx.wrapping_add_signed(dx as i16),
                    nz.wrapping_add_signed(dz as i16),
                    0,
                    0x100 | 0x200000,
                )
        })
        .map(|(dx, dz)| {
            (
                nx.wrapping_add_signed(dx as i16),
                nz.wrapping_add_signed(dz as i16),
            )
        })
        .unwrap_or_else(|| panic!("a tile beside npc {kind} at {nx},{nz}"));
    (nid, sx, sz)
}

fn ticks(engine: &mut Engine, client: &mut HarnessClient, n: usize) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..n {
        engine.cycle();
        said.extend(
            HarnessClient::game_messages(&client.packets())
                .into_iter()
                .map(|(_, t)| t),
        );
    }
    said
}

fn op_npc1(client: &mut HarnessClient, nid: u16) {
    let mut p = ClientPacket::new();
    p.p1_alt3(0).p2(nid as i32);
    client.send(ClientProt::OpNpc1, &p.finish());
}

/// Uses the obj in `slot` on the loc `loc` at `(x, z)` (OPLOCT).
fn use_on(client: &mut HarnessClient, o: u16, slot: i32, (loc, x, z): (u16, u16, u16)) {
    let mut p = ClientPacket::new();
    p.p2_alt2(o as i32)
        .p2_alt2(slot)
        .p2(z as i32)
        .p4_alt2(BACKPACK)
        .p2_alt2(x as i32)
        .p1_alt1(0)
        .p2_alt1(loc as i32);
    client.send(ClientProt::OpLocT, &p.finish());
}

fn total(engine: &Engine, client: &HarnessClient, inv_id: u16, o: u16) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv_id)
        .map_or(0, |i| i.total(o) as i32)
}

/// Walks to Fred and talks, taking `choices`.
fn talk_to_fred(engine: &mut Engine, client: &mut HarnessClient, choices: &[&str]) -> Conversation {
    let fred = npc(engine, "fred_the_farmer");
    let (nid, x, z) = beside(engine, fred, (3190, 3273));
    engine.harness_telejump(client.pid, x, 0, z);
    for _ in 0..3 {
        engine.cycle();
    }
    client.packets();
    talk_to(client, nid);
    converse(engine, client, choices, 300)
}

/// Shears Fred's black sheep until one gives wool, or says why it will not;
/// returns the messages of the last try.
fn shear_a_black_sheep(
    engine: &mut Engine,
    client: &mut HarnessClient,
    wool: (u16, u16),
) -> Vec<String> {
    let (backpack, black_wool) = wool;
    let black = npc(engine, "sheep_8876");
    let before = total(engine, client, backpack, black_wool);
    let mut said = Vec::new();
    // Sheep get away one time in four (289's): try again until one is shorn.
    for _ in 0..12 {
        let (nid, x, z) = beside(engine, black, (3200, 3268));
        engine.harness_telejump(client.pid, x, 0, z);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        op_npc1(client, nid);
        said = ticks(engine, client, 15);
        if total(engine, client, backpack, black_wool) > before
            || said
                .iter()
                .any(|m| m.starts_with("The farmer wouldn't like"))
        {
            break;
        }
    }
    said
}

#[test]
fn fred_s_job_from_taking_it_to_his_pay() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("shearer").expect("login");
        engine.cycle();
        let backpack = inv(engine, "inv");
        let bank = inv(engine, "bank");
        let sheep = varp(engine, "sheep");
        assert_eq!(sheep, SHEEP, "289's sheep is the cache's varp 179");
        let (shears, black_wool, ball, coins) = (
            obj(engine, "shears"),
            obj(engine, "black_wool"),
            obj(engine, "black_ball_of_wool"),
            obj(engine, "coins"),
        );
        let _ = engine.script_errors.take();

        // Before the job, the black sheep are not for shearing.
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&backpack).unwrap().set(0, shears, 1);
        let said = shear_a_black_sheep(engine, &mut client, (backpack, black_wool));
        assert!(
            said.contains(
                &"The farmer wouldn't like you shearing these special sheep. Try the white ones."
                    .to_string()
            ),
            "before the job: {said:?}"
        );
        assert_eq!(total(engine, &client, backpack, black_wool), 0);

        // Fred offers work, and the player takes it.
        let talk = talk_to_fred(
            engine,
            &mut client,
            &[
                "I'm looking for work.",
                "It takes all sorts",
                "I'll take the job.",
                "Of course!",
                "I'm something of an expert",
            ],
        );
        assert!(
            talk.said("bring me twenty balls of black wool"),
            "{:?}",
            talk.texts
        );
        assert!(
            talk.said("I'm not paying you by the hour!"),
            "{:?}",
            talk.texts
        );
        assert_eq!(talk.unanswered, None);
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(player.vars.get(sheep).as_int(), 1, "the job is on");

        // A black sheep gives black wool now.
        let said = shear_a_black_sheep(engine, &mut client, (backpack, black_wool));
        assert_eq!(total(engine, &client, backpack, black_wool), 1, "{said:?}");
        assert!(
            said.contains(&"You get some black wool.".to_string()),
            "{said:?}"
        );

        // Lumbridge Castle's wheel spins it into a ball (the wool sits after
        // the shears).
        engine.harness_telejump(client.pid, AT_WHEEL.0, AT_WHEEL.1, AT_WHEEL.2);
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[CRAFTING];
        let slot = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&backpack)
            .unwrap()
            .slots
            .iter()
            .position(|i| i.as_ref().is_some_and(|i| i.obj == black_wool))
            .expect("the black wool") as i32;
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        use_on(&mut client, black_wool, slot, WHEEL);
        let said = ticks(engine, &mut client, 8);
        assert_eq!(total(engine, &client, backpack, ball), 1, "spun: {said:?}");
        assert_eq!(total(engine, &client, backpack, black_wool), 0);
        assert_eq!(
            engine.harness_player(client.pid).unwrap().stats.xp[CRAFTING] - xp,
            25,
            "2.5 xp"
        );

        // Five balls: Fred takes them and wants fifteen more.
        let player = engine.harness_player_mut(client.pid).unwrap();
        let pack = player.invs.get_mut(&backpack).unwrap();
        for slot in 0..5 {
            pack.set(slot, ball, 1);
        }
        let talk = talk_to_fred(
            engine,
            &mut client,
            &["I have some balls of black wool for you."],
        );
        assert!(
            talk.said("I need 15 more before I can pay you."),
            "{:?}",
            talk.texts
        );
        assert_eq!(total(engine, &client, backpack, ball), 0);
        assert_eq!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .vars
                .get(sheep)
                .as_int(),
            6
        );

        // The other fifteen, with black wool left over in the pack and the
        // bank: he pays, and takes the rest.
        let player = engine.harness_player_mut(client.pid).unwrap();
        let pack = player.invs.get_mut(&backpack).unwrap();
        for slot in 0..15 {
            pack.set(slot, ball, 1);
        }
        pack.set(15, black_wool, 1);
        // The bank as opening it would make it (Engine::get_or_create_inv).
        let size = engine.cache.invs.get_by_id(bank).unwrap().size as usize;
        let player = engine.harness_player_mut(client.pid).unwrap();
        let held = player
            .invs
            .entry(bank)
            .or_insert_with(|| Inventory::with_stack_mode(size, StackMode::Always));
        held.set(0, coins, 5);
        held.set(1, black_wool, 3);
        held.set(2, shears, 1);
        let xp = engine.harness_player(client.pid).unwrap().stats.xp[CRAFTING];
        let talk = talk_to_fred(
            engine,
            &mut client,
            &["I have some balls of black wool for you."],
        );
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "paying: {errors:?}");
        assert!(talk.said("That's the last of them."), "{:?}", talk.texts);
        // The item box breaks its line with <br>.
        assert!(
            talk.said("Fred gives you some money and teaches you some<br>Crafting techniques."),
            "{:?}",
            talk.texts
        );
        let player = engine.harness_player(client.pid).unwrap();
        assert_eq!(
            player.vars.get(sheep).as_int(),
            21,
            "done (cache quest 173)"
        );
        assert_eq!(player.stats.xp[CRAFTING] - xp, 1500, "150 Crafting xp");
        assert_eq!(total(engine, &client, backpack, coins), 2000);
        for o in [ball, black_wool] {
            assert_eq!(total(engine, &client, backpack, o), 0, "pack keeps {o}");
            assert_eq!(total(engine, &client, bank, o), 0, "bank keeps {o}");
        }
        let held = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&bank)
            .unwrap();
        assert_eq!(
            held.get(1).map(|i| i.obj),
            Some(shears),
            "the bank closed up"
        );

        // Afterwards, the black sheep are not for shearing again.
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.invs.get_mut(&backpack).unwrap().set(16, shears, 1);
        let said = shear_a_black_sheep(engine, &mut client, (backpack, black_wool));
        assert!(
            said.iter()
                .any(|m| m.starts_with("The farmer wouldn't like")),
            "after the job: {said:?}"
        );
        let errors = engine.script_errors.take();
        assert!(errors.is_empty(), "{errors:?}");
    });
}

/// Explores every branch of Fred's Talk-to with %sheep at `stage` and
/// %the_thing_interacted at `thing`, `balls` balls of black wool in the pack,
/// and checks each said something and no script failed.
fn every_branch(engine: &mut Engine, stage: i32, thing: bool, balls: u16) -> usize {
    let sheep = varp(engine, "sheep");
    let the_thing = varp(engine, "the_thing_interacted");
    let backpack = inv(engine, "inv");
    let ball = obj(engine, "black_ball_of_wool");
    let fred = npc(engine, "fred_the_farmer");
    let _ = engine.script_errors.take();
    let branches = explore(
        engine,
        |engine| {
            engine.harness_reset();
            let mut client = engine
                .harness_login_returning("fred_visitor")
                .expect("login");
            for _ in 0..10 {
                engine.cycle();
            }
            let (nid, x, z) = beside(engine, fred, (3190, 3273));
            engine.harness_telejump(client.pid, x, 0, z);
            let player = engine.harness_player_mut(client.pid).unwrap();
            player.vars.set(sheep, VarValue::Int(stage));
            player.vars.set(the_thing, VarValue::Int(thing as i32));
            let pack = player.invs.get_mut(&backpack).unwrap();
            for slot in 0..balls {
                pack.set(slot, ball, 1);
            }
            for _ in 0..3 {
                engine.cycle();
            }
            client.packets();
            talk_to(&mut client, nid);
            client
        },
        48,
    );
    let errors: Vec<String> = engine
        .script_errors
        .take()
        .into_iter()
        .map(|e| format!("{e:?}"))
        .collect();
    assert!(errors.is_empty(), "%sheep={stage}: {errors:?}");
    assert!(!branches.is_empty());
    for branch in &branches {
        assert!(
            !branch.conversation.texts.is_empty(),
            "%sheep={stage}, path {:?}: nothing said ({:?}, menus {:?})",
            branch.path,
            branch.conversation.messages,
            branch.menus
        );
    }
    branches.len()
}

/// Before the job, during it (with balls and without, having seen The Thing
/// or not), and after.
#[test]
fn fred_answers_every_branch_at_every_stage() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut ways = Vec::new();
        for (stage, thing, balls) in [
            (0, false, 0),
            (1, false, 0),
            (1, true, 0),
            (10, false, 3),
            (20, false, 1),
            (21, false, 0),
            (21, true, 0),
        ] {
            ways.push(every_branch(engine, stage, thing, balls));
        }
        // Before the job: kill, lost, and work's 2 x 2 x (3 + 2) ways in.
        assert!(ways[0] >= 10, "{ways:?}");
        // After it, the Thing adds its story to kill and lost.
        assert_eq!(ways[5], 2, "{ways:?}");
        assert_eq!(ways[6], 3, "{ways:?}");
    });
}
