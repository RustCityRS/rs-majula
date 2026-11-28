//! Dragging one backpack slot onto another moves the obj, as the client
//! reports the drag.
//!
//! IF_BUTTOND (`SceneGraphNode_Sub6_Sub1.method3715`) carries both ends. The
//! drop target is a component of interface 149's second bank of 28 backing
//! sprites, which login unlocks for drag, so the target slot arrives as
//! 28 + slot and `[inv_buttond,inventory:inv]` takes the offset back off.

#![cfg(since_530)]

use rs_engine::Engine;
use rs_test::names::BACKPACK;
use rs_test::runner;
use rs_ver::TypeId;

#[test]
fn a_drag_moves_and_swaps_backpack_slots() {
    runner::on_loaded_engine(|engine| {
        let logs = engine.cache.objs.get_by_debugname("logs").unwrap().id;
        let sword = engine
            .cache
            .objs
            .get_by_debugname("bronze_sword")
            .unwrap()
            .id;

        // Into an empty slot: the obj moves and its slot empties.
        let slots = drag(engine, &[(0, logs.g2())], 0, 5, true);
        assert_eq!(
            (slots[0], slots[5]),
            (None, Some(logs)),
            "logs moved to slot 5"
        );

        // Onto an occupied slot: the two change places.
        for backing in [true, false] {
            let slots = drag(engine, &[(0, logs.g2()), (3, sword.g2())], 0, 3, backing);
            assert_eq!(
                (slots[0], slots[3]),
                (Some(sword), Some(logs)),
                "logs and sword swapped"
            );
        }
    });
}

/// A returning player holding `held`, after one drag from `from` onto the
/// backing sprite of `to`. Returns the backpack, slot by slot.
fn drag(
    engine: &mut Engine,
    held: &[(u16, u16)],
    from: u16,
    to: u16,
    backing: bool,
) -> Vec<Option<TypeId>> {
    engine.harness_reset();
    let mut client = engine.harness_login_returning("dragger").expect("login");
    engine.cycle();
    let inv = engine.cache.invs.get_by_debugname("inv").unwrap().id;
    let backpack = engine
        .harness_player_mut(client.pid)
        .unwrap()
        .invs
        .get_mut(&inv)
        .unwrap();
    for &(slot, obj) in held {
        backpack.set(slot, TypeId::new(obj), 1);
    }
    let source_obj = held.iter().find(|(s, _)| *s == from).unwrap().1;
    // inventory_drag swaps the component objects before IF_BUTTOND reads them.
    let displayed_obj = held
        .iter()
        .find(|(s, _)| *s == to)
        .map_or(-1, |(_, obj)| *obj as i32);

    // 647's backpack has a backing sprite under each slot (28 + slot); 530's if1 backpack has
    // none.
    #[cfg(before_647)]
    let slot2 = to;
    #[cfg(since_647)]
    let slot2 = if backing { 28 + to } else { to };
    client.if_button_d(
        BACKPACK,
        from,
        TypeId::from_int(displayed_obj),
        BACKPACK,
        slot2,
        if backing {
            TypeId::NULL
        } else {
            TypeId::from_int(source_obj as i32)
        },
    );
    engine.cycle();

    assert_eq!(
        engine.harness_player(client.pid).unwrap().last_item,
        Some(TypeId::new(source_obj))
    );

    let backpack = engine
        .harness_player(client.pid)
        .unwrap()
        .invs
        .get(&inv)
        .unwrap();
    (0..28)
        .map(|slot| backpack.get(slot).map(|o| o.obj))
        .collect()
}
