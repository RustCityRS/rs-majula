use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{in_build_area, loc_target, zone_loc};
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::oploc1::OpLoc1;
use rs_protocol::network::game::client::oploc2::OpLoc2;
use rs_protocol::network::game::client::oploc3::OpLoc3;
use rs_protocol::network::game::client::oploc4::OpLoc4;
use rs_protocol::network::game::client::oploc5::OpLoc5;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpLoc1` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 1.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpLoc1 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(1, self.x, self.z, self.loc, active)
    }
}

/// Handles the `OpLoc2` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 2.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpLoc2 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(2, self.x, self.z, self.loc, active)
    }
}

/// Handles the `OpLoc3` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 3.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpLoc3 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(3, self.x, self.z, self.loc, active)
    }
}

/// Handles the `OpLoc4` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 4.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpLoc4 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(4, self.x, self.z, self.loc, active)
    }
}

/// Handles the `OpLoc5` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 5.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpLoc5 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(5, self.x, self.z, self.loc, active)
    }
}

/// Shared handler for location (scenery) operations (ops 1-5).
///
/// Processes a right-click menu operation on a world location (e.g., door, tree,
/// furnace). Validates the target coordinates are within the player's build area,
/// looks up the location in the zone, verifies the operation option exists on the
/// location type, and sets up an approach-style interaction (`ApLoc1`-`ApLoc5`)
/// that will trigger the corresponding script once the player reaches the location.
///
/// # Arguments
///
/// * `op` - The operation number (1-5), corresponding to a right-click menu option.
/// * `x` - The X coordinate of the target location.
/// * `z` - The Z coordinate of the target location.
/// * `loc_id` - The location type ID.
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success or if the player is delayed / target is invalid.
///
/// # Side Effects
///
/// * Clears pending action and unsets map flag on early exit conditions.
/// * Sets up an `InteractionTarget::Loc` interaction with approach mode on the player.
/// * Sets `opcalled` to `true` on the player.
///
/// # Call Stack
///
/// **Called by:** `OpLoc1::handle` through `OpLoc5::handle`
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
fn handle(
    op: u8,
    x: u16,
    z: u16,
    loc_id: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    if active.player.state.delayed {
        active.unset_map_flag();
        return Ok(());
    }

    if !in_build_area(active, x, z) {
        active.unset_map_flag();
        let _ = active.clear_pending_action();
        return Ok(());
    }

    let engine = engine();

    let y = active.player.pathing.coord.y();
    let Some(loc) = zone_loc(x, y, z, loc_id) else {
        active.unset_map_flag();
        active.clear_pending_action()?;
        return Ok(());
    };

    let loc_type = engine.locs().get_by_id(loc_id);
    if let Some(lt) = &loc_type {
        if let Some(ops) = &lt.op {
            if ops.get((op - 1) as usize).is_none_or(|o| o.is_none()) {
                active.unset_map_flag();
                active.clear_pending_action()?;
                return Ok(());
            }
        } else {
            active.unset_map_flag();
            active.clear_pending_action()?;
            return Ok(());
        }
    }

    let mode = match op {
        1 => ServerTriggerType::ApLoc1,
        2 => ServerTriggerType::ApLoc2,
        3 => ServerTriggerType::ApLoc3,
        4 => ServerTriggerType::ApLoc4,
        _ => ServerTriggerType::ApLoc5,
    };

    let target = loc_target(CoordGrid::new(x, y, z), loc);

    active.clear_pending_action()?;
    active.player.set_interaction(target, mode as u8, true);
    active.player.opcalled = true;

    Ok(())
}
