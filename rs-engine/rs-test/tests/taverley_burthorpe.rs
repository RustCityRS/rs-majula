//! 289's Taverley and Burthorpe at 647 (`areas/area_taverly`,
//! `areas/area_burthorpe`): every way through their npcs' Talk-to says
//! something and fails no script, and the crystal key's halves join as 289's
//! and 2011's do (2011 wiki, Crystal key: "You join the two halves of the key
//! together.").

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_test::runner;
use rs_test::talk::{explore, talk_to};
use rs_vm::ops::test::{calls_for, set_recording};
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
fn taverley_and_burthorpe_npcs_answer_every_branch() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for npc in [
            "death_white_knight",
            "eadburg",
            "gaius",
            "jatix",
            "servant",
            "wistan",
        ] {
            let _ = engine.script_errors.take();
            let branches = explore(
                engine,
                |engine| {
                    engine.harness_reset();
                    let mut client: HarnessClient = engine
                        .harness_login_returning("taverley_visitor")
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
                // A box, or a game message.
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

/// The tooth and loop halves make a crystal key, used either way round.
#[test]
fn the_crystal_key_halves_join() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let obj = |engine: &Engine, name: &str| *engine.cache.objs.debugnames.get(name).unwrap();
        let (tooth, lp, key) = (
            obj(engine, "tooth_half_of_a_key"),
            obj(engine, "loop_half_of_a_key"),
            obj(engine, "crystal_key"),
        );
        let backpack = *engine.cache.invs.debugnames.get("inv").unwrap();
        engine.harness_reset();
        let mut client = engine.harness_login_returning("key_maker").expect("login");
        engine.cycle();
        let pack = engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&backpack)
            .unwrap();
        pack.set(0, tooth, 1);
        pack.set(1, lp, 1);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        set_recording(true);
        let backpack_if: i32 = 149 << 16;
        let mut p = ClientPacket::new();
        p.p4(backpack_if)
            .p2_alt1(1)
            .p2_alt1(0)
            .p2_alt2(tooth as i32)
            .p4_alt1(backpack_if)
            .p2(lp as i32);
        client.send(ClientProt::IfButtonT, &p.finish());
        for _ in 0..4 {
            engine.cycle();
        }
        let pack = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&backpack)
            .unwrap();
        assert_eq!(
            (pack.total(key), pack.total(tooth), pack.total(lp)),
            (1, 0, 0)
        );
        assert!(
            calls_for("mes")
                .iter()
                .any(|m| m == "You join the two halves of the key together.")
        );
    });
}
