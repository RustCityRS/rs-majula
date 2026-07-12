use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::opobj1::OpObj1;
use rs_protocol::network::game::client::opobj2::OpObj2;
use rs_protocol::network::game::client::opobj3::OpObj3;
use rs_protocol::network::game::client::opobj4::OpObj4;
use rs_protocol::network::game::client::opobj5::OpObj5;
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpObj1` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 1.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpObj1 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(1, self.x, self.z, self.obj, active)
    }
}

/// Handles the `OpObj2` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 2.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpObj2 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(2, self.x, self.z, self.obj, active)
    }
}

/// Handles the `OpObj3` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 3.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpObj3 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(3, self.x, self.z, self.obj, active)
    }
}

/// Handles the `OpObj4` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 4.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpObj4 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(4, self.x, self.z, self.obj, active)
    }
}

/// Handles the `OpObj5` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 5.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpObj5 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(5, self.x, self.z, self.obj, active)
    }
}

/// Shared handler for ground object operations (ops 1-5).
///
/// Processes a right-click menu operation on a ground object (item on the floor).
/// Validates the target coordinates are within the player's build area (within
/// 52 tiles of origin), looks up the ground object in the zone (checking receiver
/// ownership for the active player), verifies the operation option exists on the
/// object type, and sets up an approach-style interaction (`ApObj1`-`ApObj5`).
///
/// Operations 1 and 4 require explicit `op` entries on the object type definition;
/// operations 2, 3, and 5 are allowed even without explicit entries (e.g., "take"
/// and "examine" are implicit).
///
/// # Arguments
///
/// * `op` - The operation number (1-5), corresponding to a right-click menu option.
/// * `x` - The X coordinate of the target ground object.
/// * `z` - The Z coordinate of the target ground object.
/// * `obj_id` - The object type ID of the ground item.
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success or if the player is delayed / target is invalid.
///
/// # Side Effects
///
/// * Clears pending action and unsets map flag on early exit conditions.
/// * Sets up an `InteractionTarget::Obj` interaction with approach mode on the player.
/// * Sets `opcalled` to `true` on the player.
///
/// # Call Stack
///
/// **Called by:** `OpObj1::handle` through `OpObj5::handle`
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
fn handle(
    op: u8,
    x: u16,
    z: u16,
    obj_id: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    if active.player.state.delayed {
        active.unset_map_flag();
        return Ok(());
    }

    let origin_x = active.player.build_area.origin.x() as i32;
    let origin_z = active.player.build_area.origin.z() as i32;
    if (x as i32) < origin_x - 52
        || (x as i32) > origin_x + 52
        || (z as i32) < origin_z - 52
        || (z as i32) > origin_z + 52
    {
        active.unset_map_flag();
        active.clear_pending_action()?;
        return Ok(());
    }

    let engine = engine();

    let y = active.player.pathing.coord.y();
    let Some(zone) = engine.zones.zone(x, y, z) else {
        active.player.move_request = false;
        active.clear_pending_action()?;
        return Ok(());
    };
    let Some(idx) = zone.get_obj(x, z, obj_id, Some(active.uid().username37())) else {
        active.player.move_request = false;
        active.clear_pending_action()?;
        return Ok(());
    };
    let obj = &zone.objs[idx];

    let obj_type = engine.objs().get_by_id(obj_id);
    if let Some(ot) = &obj_type {
        if let Some(ops) = &ot.op {
            if (op == 1 && ops.first().is_none_or(|o| o.is_none()))
                || (op == 4 && ops.get(3).is_none_or(|o| o.is_none()))
            {
                active.unset_map_flag();
                active.clear_pending_action()?;
                return Ok(());
            }
        } else if op == 1 || op == 4 {
            active.unset_map_flag();
            active.clear_pending_action()?;
            return Ok(());
        }
    }

    let mode = match op {
        1 => ServerTriggerType::ApObj1,
        2 => ServerTriggerType::ApObj2,
        3 => ServerTriggerType::ApObj3,
        4 => ServerTriggerType::ApObj4,
        _ => ServerTriggerType::ApObj5,
    };

    let target = InteractionTarget::Obj {
        coord: CoordGrid::new(x, y, z),
        id: obj_id,
        count: obj.count(),
    };

    active.clear_pending_action()?;
    active.player.set_interaction(target, mode as u8, true);
    active.player.opcalled = true;

    Ok(())
}
