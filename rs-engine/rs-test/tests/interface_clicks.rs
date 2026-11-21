#![cfg(since_530)]

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
#[cfg(since_647)]
use rs_entity::MoveSpeed;
#[cfg(since_647)]
use rs_test::names::BACKPACK;
#[cfg(since_647)]
use rs_test::names::INVENTORY;
#[cfg(since_647)]
use rs_test::names::com;
use rs_test::runner;
use rs_test::server;
#[cfg(since_647)]
use rs_ver::TypeId;

#[cfg(since_647)]
fn run_orb() -> i32 {
    com("topstat_run:run_background")
}

#[cfg(since_647)]
const NO_OBJ: TypeId = TypeId::NULL;
#[cfg(since_647)]
fn high_alchemy() -> i32 {
    com("magic:sb28")
}

#[cfg(since_647)]
fn fresh(engine: &mut Engine) -> (HarnessClient, TypeId, TypeId, TypeId) {
    engine.harness_reset();
    let client = engine.harness_login_returning("click_checks").unwrap();
    engine.cycle();
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let logs = engine.cache.objs.get_by_debugname("logs").unwrap().id;
    let sword = engine
        .cache
        .objs
        .get_by_debugname("bronze_sword")
        .unwrap()
        .id;
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

/// The URLs a tick has the client open.
fn opened(engine: &mut Engine, client: &mut HarnessClient) -> Vec<String> {
    engine.cycle();
    server::urls(&client.packets())
}

#[cfg(since_647)]
#[test]
fn clicks_require_server_permissions_and_current_inventory_contents() {
    runner::on_loaded_engine(|engine| {
        for case in 0..8 {
            let (mut client, inv, logs, sword) = fresh(engine);
            let (com, slot, obj) = match case {
                0 => (BACKPACK, 0, sword),
                1 => (BACKPACK, 0, NO_OBJ),
                2 => (BACKPACK, 28, logs),
                3 => (BACKPACK | 0xFFFF, 0, logs),
                4 => (com("bank_v2_main:bank_v2_inv"), 0, logs),
                _ => (BACKPACK, 0, logs),
            };
            let player = engine.harness_player_mut(client.pid).unwrap();
            match case {
                5 => {
                    player.subs.events.insert((BACKPACK, 0), 0);
                }
                6 => {
                    player.clear_inv_transmits(BACKPACK);
                }
                7 => {
                    player.state.delayed = true;
                }
                _ => {}
            }
            client.if_button(8, com, slot, obj);
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
        client.if_button(10, BACKPACK, 0, logs);
        engine.harness_inputs();
        assert_eq!(
            engine.harness_player(client.pid).unwrap().last_item,
            Some(logs)
        );
        client.if_button(1, run_orb(), u16::MAX, TypeId::NULL);
        engine.harness_inputs();
        assert_eq!(
            engine.harness_player(client.pid).unwrap().move_speed,
            MoveSpeed::Run,
            "cache-defined static button is enabled"
        );
        // A cache button no script takes: 647's price-checker, 742's toolbelt.
        #[cfg(before_742)]
        let unscripted = "pricechecker_container";
        #[cfg(since_742)]
        let unscripted = "com_42";
        client.if_button(1, (387 << 16) | 42, u16::MAX, TypeId::NULL);
        engine.cycle();
        let trigger = format!("No trigger for [if_button1,wornitems:{unscripted}]");
        assert!(
            HarnessClient::game_messages(&client.packets())
                .iter()
                .any(|(_, m)| m.ends_with(&trigger))
        );

        for case in 0..7 {
            let (mut client, inv, logs, sword) = fresh(engine);
            let (slot, obj, com2, slot2, obj2) = match case {
                0 => (5, sword, BACKPACK, 33, NO_OBJ),
                1 => (0, logs, BACKPACK, 56, NO_OBJ),
                2 => (28, NO_OBJ, BACKPACK, 33, NO_OBJ),
                3 => (0, sword, BACKPACK, 1, logs),
                4 => (0, logs, run_orb(), u16::MAX, NO_OBJ),
                5 => (0, NO_OBJ, BACKPACK, 33, NO_OBJ),
                _ => (0, logs, BACKPACK, 33, NO_OBJ),
            };
            if matches!(case, 3 | 6) {
                engine.harness_player_mut(client.pid).unwrap().state.delayed = true;
            }
            if case == 5 {
                engine
                    .harness_player_mut(client.pid)
                    .unwrap()
                    .subs
                    .events
                    .insert((BACKPACK, 0), 0);
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
            client.if_button_d(BACKPACK, slot, obj, com2, slot2, obj2);
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
                let mut expected = vec![(0, Some((logs.int(), 1)))];
                if case == 3 {
                    expected.push((1, Some((sword.int(), 1))));
                }
                assert_eq!(
                    server::inv_changes(&packets),
                    [(inv.int(), false, expected)],
                    "resend only affected inventory slots"
                );
                assert!(server::invs(&packets).is_empty());
            }
        }

        let (mut client, inv, logs, _) = fresh(engine);
        client.if_button_d(BACKPACK, 0, TypeId::NULL, BACKPACK, 33, TypeId::NULL);
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
                *player.subs.events.get_mut(&(BACKPACK, 0)).unwrap() &= !(0x20 << 11);
            } else if case == 2 {
                *player.subs.events.get_mut(&(BACKPACK, 1)).unwrap() &= !(1 << 22);
            }
            let (com, slot, obj, slot2, obj2) = match case {
                0 => (BACKPACK, 0, logs, 0, logs),
                3 => (run_orb(), u16::MAX, NO_OBJ, 1, sword),
                4 => (BACKPACK, 0, sword, 1, sword),
                _ => (BACKPACK, 0, logs, 1, sword),
            };
            client.if_button_t(com, slot, obj, BACKPACK, slot2, obj2);
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
        client.if_button_t(high_alchemy(), u16::MAX, TypeId::NULL, BACKPACK, 0, logs);
        engine.harness_inputs();
        assert_eq!(
            engine.harness_player(client.pid).unwrap().last_item,
            Some(logs),
            "a spell with the component target mask may target an item"
        );

        let (mut client, _, logs, _) = fresh(engine);
        let other = engine.harness_login_returning("click_target").unwrap();
        engine.cycle();
        client.op_player_t(other.pid, high_alchemy(), u16::MAX, TypeId::NULL);
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
        client.op_player_t(other.pid, BACKPACK, 0, logs);
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
            .subs
            .open
            .iter()
            .find(|(_, (sub, _))| *sub == INVENTORY)
            .unwrap()
            .0;
        let player = engine.harness_player_mut(client.pid).unwrap();
        player.subs.closed(parent);
        player.subs.opened(parent, INVENTORY, 1);
        client.if_button(8, BACKPACK, 0, logs);
        engine.harness_inputs();
        assert_eq!(
            engine.harness_player(client.pid).unwrap().last_item,
            None,
            "reopening a sub must not revive its old event permissions"
        );
    });
}

#[test]
fn a_link_opens_its_page_on_the_website() {
    runner::on_loaded_engine(|engine| {
        engine.harness_reset();
        let mut client = engine.harness_login_returning("link_checks").unwrap();
        engine.cycle();
        client.packets();

        assert!(engine.set_site_origin("https://example.org/ https://www.example.org"));
        client.url_request("forum", "forums.ws", 0);
        client.url_request("www", "parents.ws", 0);
        assert_eq!(
            opened(engine, &mut client),
            ["https://example.org/m=forum/forums.ws"],
            "one link a tick"
        );
        client.url_request("www", "parents.ws", 0);
        assert_eq!(
            opened(engine, &mut client),
            ["https://example.org/parents.ws"]
        );

        client.url_request("forum", "forums.ws", 1);
        client.url_request("forum", "../admin", 0);
        assert!(
            opened(engine, &mut client).is_empty(),
            "links no interface offers"
        );

        assert!(!engine.set_site_origin(""));
        client.url_request("forum", "forums.ws", 0);
        assert!(opened(engine, &mut client).is_empty(), "no website");
        assert!(engine.script_errors.take().is_empty());
    });
}
