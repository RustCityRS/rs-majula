use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    HeldCheck, check_held, in_build_area, members_blocked, use_component_ok, zone_obj,
};
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::opobju::OpObjU;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpObjU` (use item on ground object) client protocol message.
///
/// Processes a "use held item on ground object" interaction. Validates that the
/// target coordinates are within the player's build area, that the ground object
/// exists, that the use component (`com`) is usable and visible, and that the used
/// item exists at the given slot. Sets up an approach-style interaction (`ApObjU`)
/// that will trigger the corresponding script once the player reaches the object.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success or if the player is delayed / target is invalid.
///
/// # Side Effects
///
/// * Clears pending action and unsets the map flag on early exit conditions.
/// * Sets `last_use_item` and `last_use_slot` on the player.
/// * Sets up an `InteractionTarget::Obj` interaction with approach mode and sets `opcalled`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
impl ClientGameHandler for OpObjU {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            active.unset_map_flag();
            return Ok(());
        }

        if !in_build_area(active, self.x, self.z) {
            // bad client: tile is not visible on client
            active.unset_map_flag();
            return Ok(());
        }

        let y = active.player.pathing.coord.y();
        let receiver = active.uid().username37();
        let Some(obj) = zone_obj(self.x, y, self.z, self.obj, receiver) else {
            // bad client or lag: obj does not exist
            active.unset_map_flag();
            return Ok(());
        };

        let target = InteractionTarget::Obj {
            coord: CoordGrid::new(self.x, y, self.z),
            id: self.obj,
            count: obj.count(),
        };

        if !use_component_ok(active, self.com) {
            // bad client or lag: component is not acceptable for this packet, or not visible
            active.unset_map_flag();
            return Ok(());
        }

        if check_held(active, self.com, self.slot, self.use_obj) != HeldCheck::Ok {
            // bad client or lag: item does not exist at that slot of a transmitted inventory
            active.unset_map_flag();
            return Ok(());
        }

        active.clear_pending_action()?;

        if members_blocked(active, self.use_obj) {
            return Ok(());
        }

        active.player.last_use_item = Some(self.use_obj);
        active.player.last_use_slot = Some(self.slot);

        active
            .player
            .set_interaction(target, ServerTriggerType::ApObjU as u8, true);
        active.player.opcalled = true;

        Ok(())
    }
}
