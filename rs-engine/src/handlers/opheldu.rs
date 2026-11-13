use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::core::{engine, engine_mut};
#[cfg(before_647)]
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    action_target, held_component_ok, require_held, use_component_ok,
};
use crate::watch::{Target, Using};
#[cfg(before_647)]
use rs_protocol::network::game::client::opheldu::OpHeldU;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};
use rs_vm::state::ScriptState;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};
use std::mem::swap;

#[cfg(before_647)]
impl ClientGameHandler for OpHeldU {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(
            self.com.into(),
            self.slot,
            self.obj,
            self.com2.into(),
            self.slot2,
            self.obj2,
            active,
        )
    }
}

pub(super) fn handle(
    com: ComId,
    slot: u16,
    obj: u16,
    com2: ComId,
    slot2: u16,
    obj2: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    active.watch_use(Using::Item(obj2), Target::Item(obj));
    if active.player.state.delayed || com != com2 {
        return Ok(());
    }
    if slot == slot2 {
        return Err(ScriptError::Client(format!("Slot used on itself: {slot}")));
    }
    if !held_component_ok(active, com, slot)
        || !use_component_ok(active, com2, slot2, action_target::HELD)
    {
        return Err(ScriptError::Client(format!(
            "Component is not acceptable for opheldu: {com}"
        )));
    }
    for (slot, obj) in [(slot, obj), (slot2, obj2)] {
        if !require_held(active, com, slot, obj)? {
            active.player.move_request = false;
            active.clear_pending_action()?;
            return Ok(());
        }
    }
    let objs = engine().objs();
    let obj = objs
        .get_by_id(obj)
        .ok_or(ScriptError::ObjNotFound(obj as i32))?;
    let obj2 = objs
        .get_by_id(obj2)
        .ok_or(ScriptError::ObjNotFound(obj2 as i32))?;

    active.player.last_item = Some(obj.id);
    active.player.last_slot = Some(slot);
    active.player.last_use_item = Some(obj2.id);
    active.player.last_use_slot = Some(slot2);
    active.clear_pending_action()?;

    if (obj.members || obj2.members) && !engine().members {
        active.message_game("To use this item please login to a members' server.");
        return Ok(());
    }
    let (script, swapped) = find_script((obj.id, obj.category), (obj2.id, obj2.category), |key| {
        engine_mut().scripts.get_by_lookup(key).cloned()
    });
    if swapped {
        let player = &mut *active.player;
        swap(&mut player.last_item, &mut player.last_use_item);
        swap(&mut player.last_slot, &mut player.last_use_slot);
    }
    if let Some(script) = script {
        let uid = active.player.uid;
        let state = ScriptState::init(script, Some(ScriptSubject::Player(uid)), None, None);
        engine_mut().run_script_by_state(
            state,
            Some(ScriptSubject::Player(uid)),
            Some(true),
            None,
        )?;
    } else {
        #[cfg(debug_assertions)]
        active.message_game(&format!("No trigger for [opheldu,{}]", obj.debugname));
        active.nothing_interesting();
    }
    Ok(())
}

fn obj_key(id: u16) -> i32 {
    ServerTriggerType::OpHeldU as i32 | (0x2 << 8) | ((id as i32) << 10)
}

fn category_key(category: u16) -> i32 {
    ServerTriggerType::OpHeldU as i32 | (0x1 << 8) | ((category as i32) << 10)
}

/// Looks up the `[opheldu]` script for `obj` (the target) and `obj2` (the
/// used item), each with its category, in the order listed on the handler.
///
/// Returns the script and whether `last_item`/`last_use_item` and the slots
/// end up swapped. A miss on `[opheldu,b]` swaps them even when `[opheldu,a]`
/// doesn't exist either, so a matched category script sees its own obj as
/// `last_useitem`: content such as `[opheldu,_murder_flourobj]` relies on
/// that. Looking up `[opheldu,a_category]` swaps them back.
fn find_script<T>(
    (obj, category): (u16, Option<u16>),
    (obj2, category2): (u16, Option<u16>),
    mut get: impl FnMut(i32) -> Option<T>,
) -> (Option<T>, bool) {
    // [opheldu,b]
    if let Some(script) = get(obj_key(obj)) {
        return (Some(script), false);
    }

    // [opheldu,a]
    let mut script = get(obj_key(obj2));
    let mut swapped = true;

    // [opheldu,b_category]
    if script.is_none()
        && let Some(category) = category
    {
        script = get(category_key(category));
    }

    // [opheldu,a_category]
    if script.is_none()
        && let Some(category) = category2
    {
        script = get(category_key(category));
        swapped = false;
    }

    (script, swapped)
}

#[cfg(test)]
mod tests {
    use super::{category_key, find_script, obj_key};

    const MURDERNECKLACEDUST: u16 = 1797;
    const MURDERPAPER: u16 = 1811;
    const POT_FLOUR: u16 = 1933;
    const MURDER_FLOUROBJ: u16 = 50;
    const MURDER_PROOFOBJ: u16 = 51;
    const STRENGTH3: u16 = 115;
    const STRENGTH2: u16 = 117;
    const POTION: u16 = 175;

    /// Runs the lookup against a script set, returning the matched key and
    /// the obj the script sees as `last_item`.
    fn run(
        target: (u16, Option<u16>),
        used: (u16, Option<u16>),
        scripts: &[i32],
    ) -> (Option<i32>, u16) {
        let (key, swapped) = find_script(target, used, |key| scripts.contains(&key).then_some(key));
        (key, if swapped { used.0 } else { target.0 })
    }

    #[test]
    fn target_obj_trigger_keeps_the_order() {
        let scripts = [obj_key(MURDERNECKLACEDUST)];
        let found = run((MURDERNECKLACEDUST, None), (MURDERPAPER, None), &scripts);
        assert_eq!(
            found,
            (Some(obj_key(MURDERNECKLACEDUST)), MURDERNECKLACEDUST)
        );
    }

    #[test]
    fn used_obj_trigger_swaps() {
        let scripts = [obj_key(POT_FLOUR)];
        let found = run((MURDERNECKLACEDUST, None), (POT_FLOUR, None), &scripts);
        assert_eq!(found, (Some(obj_key(POT_FLOUR)), POT_FLOUR));
    }

    #[test]
    fn flypaper_dragged_onto_the_flour_covered_item() {
        let scripts = [category_key(MURDER_FLOUROBJ)];
        let found = run(
            (MURDERNECKLACEDUST, Some(MURDER_FLOUROBJ)),
            (MURDERPAPER, None),
            &scripts,
        );
        assert_eq!(found, (Some(category_key(MURDER_FLOUROBJ)), MURDERPAPER));
    }

    #[test]
    fn flour_covered_item_dragged_onto_the_flypaper() {
        let scripts = [category_key(MURDER_FLOUROBJ)];
        let found = run(
            (MURDERPAPER, None),
            (MURDERNECKLACEDUST, Some(MURDER_FLOUROBJ)),
            &scripts,
        );
        assert_eq!(found, (Some(category_key(MURDER_FLOUROBJ)), MURDERPAPER));
    }

    #[test]
    fn target_category_wins_over_used_category() {
        let scripts = [category_key(MURDER_FLOUROBJ), category_key(MURDER_PROOFOBJ)];
        let found = run(
            (MURDERNECKLACEDUST, Some(MURDER_FLOUROBJ)),
            (POT_FLOUR, Some(MURDER_PROOFOBJ)),
            &scripts,
        );
        assert_eq!(found, (Some(category_key(MURDER_FLOUROBJ)), POT_FLOUR));
    }

    #[test]
    fn strength_potion_dragged_onto_another() {
        let scripts = [category_key(POTION)];
        let found = run(
            (STRENGTH3, Some(POTION)),
            (STRENGTH2, Some(POTION)),
            &scripts,
        );
        assert_eq!(found, (Some(category_key(POTION)), STRENGTH2));
    }

    #[test]
    fn no_trigger() {
        let found = run(
            (MURDERNECKLACEDUST, Some(MURDER_FLOUROBJ)),
            (MURDERPAPER, None),
            &[],
        );
        assert_eq!(found.0, None);
    }
}
