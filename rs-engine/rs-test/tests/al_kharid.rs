//! 289's Al Kharid at 647 (`areas/area_alkharid`): every way through each of
//! its npcs' Talk-to says something and fails no script, 2011's tanner (Ellis)
//! and the gem and silk traders (2011 wiki: both in Al Kharid) included.

#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
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
fn al_kharid_s_npcs_answer_every_branch() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        for npc in [
            "dommik",
            "gem_trader",
            "karim",
            "louie_legs",
            "ranael",
            "shantay",
            "shantay_guard",
            "silk_trader",
            "ellis",
            "zeke",
        ] {
            let _ = engine.script_errors.take();
            let branches = explore(
                engine,
                |engine| {
                    engine.harness_reset();
                    let mut client: HarnessClient = engine
                        .harness_login_returning("al_kharid_visitor")
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
                assert!(
                    !branch.conversation.texts.is_empty(),
                    "{npc}, path {:?}: nothing said ({:?})",
                    branch.path,
                    branch.conversation.messages
                );
            }
        }
    });
}

/// 2011's tanning interface (324): eight hide rows with Tan 1, 5, 10, X and
/// All (op1-5, cache 786). At Ellis, Tan 1 on soft leather (com1) takes a
/// cowhide and 1 gp; Tan All on hard leather (com2) the rest at 3 gp each.
#[test]
fn ellis_tans_on_2011_s_tanning_interface() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    runner::on_loaded_engine(|engine| {
        let obj = |engine: &Engine, name: &str| *engine.cache.objs.debugnames.get(name).unwrap();
        let (cowhide, leather, hard, coins) = (
            obj(engine, "cowhide"),
            obj(engine, "leather"),
            obj(engine, "hard_leather"),
            obj(engine, "coins"),
        );
        let backpack = *engine.cache.invs.debugnames.get("inv").unwrap();
        engine.harness_reset();
        let mut client = engine
            .harness_login_returning("tanner_customer")
            .expect("login");
        engine.cycle();
        let (nid, x, y, z) = beside(engine, "ellis");
        engine.harness_telejump(client.pid, x, y, z);
        let pack = engine
            .harness_player_mut(client.pid)
            .unwrap()
            .invs
            .get_mut(&backpack)
            .unwrap();
        pack.set(0, cowhide, 1);
        pack.set(1, cowhide, 1);
        pack.set(2, cowhide, 1);
        pack.set(3, coins, 10);
        for _ in 0..3 {
            engine.cycle();
        }
        client.packets();
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt2(nid as i32);
        client.send(ClientProt::OpNpc3, &p.finish());
        for _ in 0..6 {
            engine.cycle();
        }
        assert!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .is_interface_open(324),
            "Trade opens the tanning interface"
        );
        let press = |client: &mut HarnessClient, prot: ClientProt, com: i32| {
            let mut p = ClientPacket::new();
            p.p4((324 << 16) | com).p2_alt3(0xFFFF).p2_alt1(0xFFFF);
            client.send(prot, &p.finish());
        };
        let pid = client.pid;
        let total = |engine: &Engine, o: u16| {
            engine
                .harness_player(pid)
                .unwrap()
                .invs
                .get(&backpack)
                .unwrap()
                .total(o)
        };
        press(&mut client, ClientProt::IfButton1, 1);
        for _ in 0..3 {
            engine.cycle();
        }
        assert_eq!(
            (
                total(engine, leather),
                total(engine, cowhide),
                total(engine, coins)
            ),
            (1, 2, 9)
        );
        // The first tan closed the interface (289's tan_leather); Trade again.
        let mut p = ClientPacket::new();
        p.p1_alt3(0).p2_alt2(nid as i32);
        client.send(ClientProt::OpNpc3, &p.finish());
        for _ in 0..6 {
            engine.cycle();
        }
        press(&mut client, ClientProt::IfButton5, 2);
        for _ in 0..3 {
            engine.cycle();
        }
        assert_eq!(
            (
                total(engine, hard),
                total(engine, cowhide),
                total(engine, coins)
            ),
            (2, 0, 3)
        );
    });
}
