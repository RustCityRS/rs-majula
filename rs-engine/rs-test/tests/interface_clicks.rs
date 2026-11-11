#![cfg(since_647)]

#[path = "../../../rs-protocol/tests/support/client_packet.rs"]
mod client_packet;

use client_packet::ClientPacket;
use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_test::runner;

const BACKPACK: i32 = 149 << 16;
const RUN_ORB: i32 = (750 << 16) | 1;
const HIGH_ALCHEMY: i32 = (192 << 16) | 59;

fn fresh(engine: &mut Engine) -> (HarnessClient, u16, u16, u16) {
    engine.harness_reset();
    let client = engine.harness_login_returning("click_checks").unwrap();
    engine.cycle();
    let inv = *engine.cache.invs.debugnames.get("inv").unwrap();
    let logs = *engine.cache.objs.debugnames.get("logs").unwrap();
    let sword = *engine.cache.objs.debugnames.get("bronze_sword").unwrap();
    let held = engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap();
    held.set(0, logs, 1);
    held.set(1, sword, 1);
    (client, inv, logs, sword)
}

fn button(com: i32, slot: u16, obj: u16) -> Vec<u8> {
    let mut p = ClientPacket::new();
    p.p4(com).p2_alt3(obj as i32).p2_alt1(slot as i32);
    p.finish()
}

fn drag(com: i32, slot: u16, obj: u16, com2: i32, slot2: u16, obj2: u16) -> Vec<u8> {
    let mut p = ClientPacket::new();
    p.p4(com)
        .p2_alt3(slot as i32)
        .p2_alt3(slot2 as i32)
        .p4_alt2(com2)
        .p2_alt1(obj as i32)
        .p2(obj2 as i32);
    p.finish()
}

fn target(com: i32, slot: u16, obj: u16, slot2: u16, obj2: u16) -> Vec<u8> {
    let mut p = ClientPacket::new();
    p.p4(BACKPACK)
        .p2_alt1(slot2 as i32)
        .p2_alt1(slot as i32)
        .p2_alt2(obj as i32)
        .p4_alt1(com)
        .p2(obj2 as i32);
    p.finish()
}

#[test]
fn clicks_require_server_permissions_and_current_inventory_contents() {
    runner::on_loaded_engine(|engine| {
        for case in 0..8 {
            let (mut client, inv, logs, sword) = fresh(engine);
            let (com, slot, obj) = match case {
                0 => (BACKPACK, 0, sword),
                1 => (BACKPACK, 0, u16::MAX),
                2 => (BACKPACK, 28, logs),
                3 => (BACKPACK | 0xFFFF, 0, logs),
                4 => ((762 << 16) | 93, 0, logs),
                _ => (BACKPACK, 0, logs),
            };
            let player = engine.harness_player_mut(client.pid).unwrap();
            match case {
                5 => {
                    player.if_events.insert((BACKPACK, 0), 0);
                }
                6 => {
                    player.clear_inv_transmits(BACKPACK);
                }
                7 => {
                    player.state.delayed = true;
                }
                _ => {}
            }
            client.send(ClientProt::IfButton8, &button(com, slot, obj));
            engine.harness_inputs();
            let player = engine.harness_player(client.pid).unwrap();
            assert!(
                player.invs.get(&inv).unwrap().has_at(0, logs),
                "rejected button case {case} changed the inventory"
            );
            assert_eq!(
                player.last_item, None,
                "rejected button case {case} changed click state"
            );
        }

        let (mut client, _, logs, _) = fresh(engine);
        client.send(ClientProt::IfButton10, &button(BACKPACK, 0, logs));
        engine.harness_inputs();
        assert_eq!(
            engine.harness_player(client.pid).unwrap().last_item,
            Some(logs)
        );
        client.send(ClientProt::IfButton1, &button(RUN_ORB, u16::MAX, u16::MAX));
        engine.harness_inputs();
        assert!(
            engine.harness_player(client.pid).unwrap().run,
            "cache-defined static button is enabled"
        );

        for case in 0..7 {
            let (mut client, inv, logs, sword) = fresh(engine);
            let (slot, obj, com2, slot2, obj2) = match case {
                0 => (0, sword, BACKPACK, 33, u16::MAX),
                1 => (0, logs, BACKPACK, 56, u16::MAX),
                2 => (28, u16::MAX, BACKPACK, 33, u16::MAX),
                3 => (0, logs, BACKPACK, 1, logs),
                4 => (0, logs, RUN_ORB, u16::MAX, u16::MAX),
                5 => (0, u16::MAX, BACKPACK, 33, u16::MAX),
                _ => (0, logs, BACKPACK, 33, u16::MAX),
            };
            if case == 6 {
                engine.harness_player_mut(client.pid).unwrap().state.delayed = true;
            }
            if matches!(case, 3 | 6) {
                engine
                    .harness_player_mut(client.pid)
                    .unwrap()
                    .invs
                    .get_mut(&inv)
                    .unwrap()
                    .clear_dirty();
                client.packets();
            }
            client.send(
                ClientProt::IfButtonD,
                &drag(BACKPACK, slot, obj, com2, slot2, obj2),
            );
            engine.harness_inputs();
            let player = engine.harness_player(client.pid).unwrap();
            assert!(
                player.invs.get(&inv).unwrap().has_at(0, logs),
                "rejected drag case {case} moved the source"
            );
            assert!(
                player.invs.get(&inv).unwrap().has_at(1, sword),
                "rejected drag case {case} changed the target"
            );
            assert_eq!(
                player.last_target_slot, None,
                "rejected drag case {case} changed click state"
            );
            if matches!(case, 3 | 6) {
                engine.cycle();
                let packets = client.packets();
                let partials: Vec<_> = packets
                    .iter()
                    .filter(|(prot, _)| *prot == ServerProt::UpdateInvPartial as u16)
                    .map(|(_, bytes)| bytes.as_slice())
                    .collect();
                let mut expected = ClientPacket::new();
                expected
                    .p2(inv as i32)
                    .p1(0)
                    .p1(0)
                    .p2(logs as i32 + 1)
                    .p1(1);
                if case == 3 {
                    expected.p1(1).p2(sword as i32 + 1).p1(1);
                }
                assert_eq!(
                    partials,
                    [expected.finish()],
                    "resend only affected inventory slots"
                );
                assert!(
                    packets
                        .iter()
                        .all(|(prot, _)| *prot != ServerProt::UpdateInvFull as u16)
                );
            }
        }

        let (mut client, inv, logs, _) = fresh(engine);
        client.send(
            ClientProt::IfButtonD,
            &drag(BACKPACK, 0, logs, BACKPACK, 33, u16::MAX),
        );
        engine.harness_inputs();
        let held = engine
            .harness_player(client.pid)
            .unwrap()
            .invs
            .get(&inv)
            .unwrap();
        assert!(
            held.get(0).is_none() && held.has_at(5, logs),
            "offset backing slots remain valid drop targets"
        );

        for case in 0..5 {
            let (mut client, _, logs, sword) = fresh(engine);
            let player = engine.harness_player_mut(client.pid).unwrap();
            if case == 1 {
                *player.if_events.get_mut(&(BACKPACK, 0)).unwrap() &= !(0x20 << 11);
            } else if case == 2 {
                *player.if_events.get_mut(&(BACKPACK, 1)).unwrap() &= !(1 << 22);
            }
            let (com, slot, obj, slot2, obj2) = match case {
                0 => (BACKPACK, 0, logs, 0, logs),
                3 => (RUN_ORB, u16::MAX, u16::MAX, 1, sword),
                4 => (BACKPACK, 0, sword, 1, sword),
                _ => (BACKPACK, 0, logs, 1, sword),
            };
            client.send(ClientProt::IfButtonT, &target(com, slot, obj, slot2, obj2));
            engine.harness_inputs();
            let player = engine.harness_player(client.pid).unwrap();
            assert_eq!(
                player.last_item, None,
                "rejected use case {case} changed click state"
            );
            assert_eq!(
                player.last_use_item, None,
                "rejected use case {case} selected an item"
            );
        }

        let (mut client, _, logs, _) = fresh(engine);
        client.send(
            ClientProt::IfButtonT,
            &target(HIGH_ALCHEMY, u16::MAX, u16::MAX, 0, logs),
        );
        engine.harness_inputs();
        assert_eq!(
            engine.harness_player(client.pid).unwrap().last_item,
            Some(logs),
            "a spell with the component target mask may target an item"
        );

        let (mut client, _, logs, _) = fresh(engine);
        let other = engine.harness_login_returning("click_target").unwrap();
        engine.cycle();
        let mut p = ClientPacket::new();
        p.p2_alt2(other.pid as i32)
            .p2_alt1(-1)
            .p4(HIGH_ALCHEMY)
            .p1_alt3(0)
            .p2_alt1(-1);
        client.send(ClientProt::OpPlayerT, &p.finish());
        engine.harness_inputs();
        assert_eq!(
            engine
                .harness_player(client.pid)
                .unwrap()
                .interaction
                .target_op,
            None,
            "alchemy cannot target a player"
        );
        let mut p = ClientPacket::new();
        p.p2_alt2(other.pid as i32)
            .p2_alt1(0)
            .p4(BACKPACK)
            .p1_alt3(0)
            .p2_alt1(logs as i32);
        client.send(ClientProt::OpPlayerT, &p.finish());
        engine.harness_inputs();
        assert_eq!(
            engine.harness_player(client.pid).unwrap().last_use_item,
            Some(logs),
            "the inventory's player target permission is respected"
        );

        let (mut client, _, logs, _) = fresh(engine);
        let parent = *engine
            .harness_player(client.pid)
            .unwrap()
            .open_subs
            .iter()
            .find(|(_, (sub, _))| *sub == 149)
            .unwrap()
            .0;
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.sub_closed(parent);
        player.sub_opened(parent, 149, 1);
        client.send(ClientProt::IfButton8, &button(BACKPACK, 0, logs));
        engine.harness_inputs();
        assert_eq!(
            engine.harness_player(client.pid).unwrap().last_item,
            None,
            "reopening a sub must not revive its old event permissions"
        );
    });
}
