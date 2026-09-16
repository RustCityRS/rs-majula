use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    HeldCheck, check_held, members_blocked, player_target_ok, use_component_ok,
};
use rs_entity::InteractionTarget;
use rs_protocol::network::game::client::opplayeru::OpPlayerU;
use rs_vm::ScriptError;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpPlayerU` (use item on player) client protocol message.
///
/// Processes a "use held item on player" interaction. Validates that the use
/// component (`com`) is usable and visible, that the used item exists at the given
/// slot, and that the target player exists and is visible to the active player.
/// Sets up an approach-style interaction (`ApPlayerU`) keyed on the used object id
/// that will trigger the corresponding script once the active player reaches the
/// target.
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
/// * Sets up an `InteractionTarget::Player` interaction with approach mode, records
///   the used object as the interaction subject, and sets `opcalled`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
impl ClientGameHandler for OpPlayerU {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            active.unset_map_flag();
            return Ok(());
        }

        if !use_component_ok(active, self.com) {
            // bad client or lag: component is not acceptable for this packet, or not visible
            active.unset_map_flag();
            return Ok(());
        }

        if check_held(active, self.com, self.slot, self.obj) != HeldCheck::Ok {
            // bad client or lag: item does not exist at that slot of a transmitted inventory
            active.unset_map_flag();
            return Ok(());
        }

        if !player_target_ok(active, self.pid) {
            // bad client or lag: player does not exist or is not visible on client
            active.unset_map_flag();
            return Ok(());
        }

        active.clear_pending_action()?;

        if members_blocked(active, self.obj) {
            return Ok(());
        }

        active.player.last_use_item = Some(self.obj);
        active.player.last_use_slot = Some(self.slot);

        active.player.set_interaction(
            InteractionTarget::Player { pid: self.pid },
            ServerTriggerType::ApPlayerU as u8,
            true,
        );
        active.player.interaction.target_subject_com = Some(self.obj);
        active.player.opcalled = true;

        Ok(())
    }
}
