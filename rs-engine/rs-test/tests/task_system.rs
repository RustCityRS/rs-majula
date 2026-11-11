//! 2011's Task System, phase 1 (`tasks/scripts`): at login the tab shows the Lumbridge/Draynor set
//! with its first open tasks in the slots (varbits 8575, 8587..), and finishing a quest that is a
//! task completes it with 2011's messages ("You have completed the Task 'Cook's Assistant'!"; the
//! 2011-05-23 recording A-JAsglDtjE's chat, where a player with 16 done also gets "You have now
//! completed 17 Tasks in total."). A first Task prints no total: 4KO2h2xPtdM (2011-04-30) @610 has
//! "You have completed the Task 'How To Train and Level'!" alone, @680 "... 2 Tasks in total.".

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{converse, talk_to};
use std::sync::Mutex;

static TURN: Mutex<()> = Mutex::new(());

/// Lumbridge/Draynor's first task (enum_3482 1 -> 346, "The Blood Pact").
const LUMBRIDGE_FIRST: i32 = 346;
const COOKS_ASSISTANT_TASK: i32 = 348;
/// The set's length: enum_3482's start, then each struct's param_1269 to the 4094 default (cache
/// 786's structs, walked outside the engine).
const LUMBRIDGE_TASKS: i32 = 133;

fn varp(engine: &Engine, client: &HarnessClient, id: u16) -> i32 {
    engine
        .harness_player(client.pid)
        .unwrap()
        .vars
        .get(id)
        .as_int()
}

fn bits(v: i32, start: u32, end: u32) -> i32 {
    ((v as u32 >> start) & ((1u32 << (end - start + 1)) - 1)) as i32
}

/// task_area (varbit 8575: varp 1960 bits 0-6).
fn area(engine: &Engine, client: &HarnessClient) -> i32 {
    bits(varp(engine, client, 1960), 0, 6)
}

/// task_slot_1..6 (varbits 8587-8592: varps 1963, 1964 and 1965, bits 0-11 and 12-24 of each).
fn slots(engine: &Engine, client: &HarnessClient) -> Vec<i32> {
    [1963, 1964, 1965]
        .into_iter()
        .flat_map(|id| {
            let v = varp(engine, client, id);
            [bits(v, 0, 11), bits(v, 12, 24)]
        })
        .collect()
}

/// varcs task_progress_total (1423) and task_progress_current (1424): the set's tasks and how
/// many of them are done, as the tab shows them.
fn progress(engine: &Engine, client: &HarnessClient) -> (i32, i32) {
    let varcs = &engine.harness_player(client.pid).unwrap().varcs;
    (varcs.get(1423).as_int(), varcs.get(1424).as_int())
}

/// task_progress_overall (varbit 8601: varp 1958 bits 0-11).
fn overall(engine: &Engine, client: &HarnessClient) -> i32 {
    bits(varp(engine, client, 1958), 0, 11)
}

/// Logs in at Lumbridge's spawn (the harness's own spawn is Tutorial Island's).
fn login(engine: &mut Engine, name: &str) -> HarnessClient {
    engine.harness_reset();
    let mut client = engine
        .harness_login_returning_at(name, 3222, 3218)
        .expect("login");
    for _ in 0..10 {
        engine.cycle();
    }
    client.packets();
    client
}

#[test]
fn the_tab_shows_lumbridges_set_at_login() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let client = login(engine, "tasks_login");
        assert_eq!(area(engine, &client), 1, "Lumbridge/Draynor");
        assert_eq!(progress(engine, &client), (LUMBRIDGE_TASKS, 0));
        let slots = slots(engine, &client);
        assert_eq!(slots[0], LUMBRIDGE_FIRST, "the set's first open task");
        let mut open = slots.clone();
        open.sort();
        open.dedup();
        assert!(
            open.len() == 6 && !open.contains(&4091),
            "six different open tasks: {slots:?}"
        );
    });
}

#[test]
fn finishing_cooks_assistant_completes_its_task() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tasks_cook");
        assert_eq!(progress(engine, &client), (LUMBRIDGE_TASKS, 0));
        let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
        let cookquest = *engine.cache.varps.debugnames.get("cookquest").unwrap();
        let items: Vec<u16> = ["cook_milk", "cook_flour", "cook_egg"]
            .iter()
            .map(|n| *engine.cache.objs.debugnames.get(*n).unwrap())
            .collect();
        {
            let player = engine.harness_player_mut(client.pid).unwrap();
            player.vars.set(cookquest, VarValue::Int(1));
            let pack = player.invs.get_mut(&inv).unwrap();
            for (slot, o) in items.iter().enumerate() {
                pack.set(slot as u16, *o, 1);
            }
        }
        let cook = *engine.cache.npcs.debugnames.get("cook").unwrap();
        let (nid, x, y, z) = engine
            .npc_list
            .npcs
            .iter()
            .flatten()
            .find(|n| n.npc.uid.id() == cook)
            .map(|n| {
                let c = n.npc.pathing.coord;
                (n.npc.uid.nid(), c.x(), c.y(), c.z())
            })
            .expect("the Cook is spawned");
        let (sx, sz) = [(1i16, 0i16), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .map(|(dx, dz)| (x.wrapping_add_signed(dx), z.wrapping_add_signed(dz)))
            .find(|&(x, z)| !rsmod::is_flagged(x, z, y, 0x100 | 0x200000))
            .expect("a tile beside the Cook");
        engine.harness_telejump(client.pid, sx, y, sz);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        talk_to(&mut client, nid);
        let mut talk = converse(engine, &mut client, &[], 400);
        for _ in 0..4 {
            engine.cycle();
            talk.messages.extend(
                HarnessClient::game_messages(&client.packets())
                    .into_iter()
                    .map(|(_, t)| t),
            );
        }
        assert_eq!(
            varp(engine, &client, cookquest),
            2,
            "Cook's Assistant complete"
        );
        assert!(
            !talk.messages.iter().any(|m| m.contains("in total")),
            "no total for a first Task; {:?}",
            talk.messages
        );
        let line = "You have completed the Task 'Cook's Assistant'!";
        assert!(
            talk.messages.iter().any(|m| m == line),
            "2011's message {line:?}; {:?}",
            talk.messages
        );
        assert_eq!(overall(engine, &client), 1, "one task in total");
        assert_eq!(
            progress(engine, &client),
            (LUMBRIDGE_TASKS, 1),
            "one of the set done"
        );
        let slots = slots(engine, &client);
        assert!(
            !slots.contains(&COOKS_ASSISTANT_TASK),
            "the done task leaves the tab: {slots:?}"
        );
    });
}

/// Karamja's "Five a Day" (task 182): one banana from each of five trees in the plantation east
/// of the volcano (banana_tree, loc 2073, in cache 786's map square 45_49).
#[test]
fn five_plantation_bananas_complete_five_a_day() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let mut client = login(engine, "tasks_bananas");
        let (sx, sz) = (2903u16..2913)
            .flat_map(|x| (3155u16..3168).map(move |z| (x, z)))
            .find(|&(x, z)| !rsmod::is_flagged(x, z, 0, 0x100 | 0x200000))
            .expect("a walkable tile in the plantation");
        engine.harness_telejump(client.pid, sx, 0, sz);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        let mut messages = Vec::new();
        let mut per_tree = Vec::new();
        for (x, z) in [
            (2906, 3161),
            (2906, 3164),
            (2909, 3158),
            (2909, 3161),
            (2909, 3164),
        ] {
            let mut p = ClientPacket::new();
            p.p2_alt1(z).p2_alt1(2073).p1(0).p2_alt2(x);
            client.send(ClientProt::OpLoc1, &p.finish());
            let mut these = Vec::new();
            for _ in 0..25 {
                engine.cycle();
                these.extend(
                    HarnessClient::game_messages(&client.packets())
                        .into_iter()
                        .map(|(_, t)| t),
                );
            }
            let at = engine.harness_player(client.pid).unwrap().pathing.coord;
            per_tree.push(((x, z), (at.x(), at.z()), these.clone()));
            messages.extend(these);
        }
        let picked = messages
            .iter()
            .filter(|m| *m == "You pick a banana.")
            .count();
        assert_eq!(picked, 5, "five bananas: {per_tree:?}");
        assert_eq!(bits(varp(engine, &client, 1000), 0, 2), 5, "varbit 3566");
        assert!(
            messages
                .iter()
                .any(|m| m == "You have completed the Task 'Five a Day'!"),
            "the task's message: {messages:?}"
        );
        assert_eq!(overall(engine, &client), 1, "one task in total");
    });
}
