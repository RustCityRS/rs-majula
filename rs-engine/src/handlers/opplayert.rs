use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_entity::InteractionTarget;
use rs_protocol::network::game::client::opplayert::OpPlayerT;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::trigger::ServerTriggerType;

/// `ComActionTarget::PLAYER` bit: the component may be cast on a player.
const ACTION_TARGET_PLAYER: u16 = 0x8;

/// Handles the `OpPlayerT` (cast spell on player) client protocol message.
///
/// Processes a "use spell on player" interaction. Validates that the spell
/// component (`com`) is acceptable for player targets and visible, that the target
/// player exists and is visible to the active player. Sets up an approach-style
/// interaction (`ApPlayerT`) keyed on the spell component id that will trigger the
/// corresponding script once the active player reaches the target.
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
/// * Sets up an `InteractionTarget::Player` interaction with approach mode on the player.
/// * Records the spell component as the interaction subject and sets `opcalled`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
impl ClientGameHandler for OpPlayerT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            active.unset_map_flag();
            return Ok(());
        }

        let engine = engine();

        let spell_com = self.com;
        let Some(spell_interface) = engine.interfaces().get_by_id(spell_com) else {
            // bad client: component is not acceptable for this packet
            active.unset_map_flag();
            return Ok(());
        };

        if spell_interface.action_target & ACTION_TARGET_PLAYER == 0 {
            // bad client: component is not acceptable for this packet
            active.unset_map_flag();
            return Ok(());
        }

        if !active
            .player
            .is_interface_visible(spell_interface.root_layer)
        {
            // bad client or lag: component is not visible
            active.unset_map_flag();
            return Ok(());
        }

        if engine_mut().get_player(self.pid).is_none() {
            // bad client or lag: player does not exist
            active.unset_map_flag();
            return Ok(());
        }

        if !active.player.build_area.players.contains(self.pid) {
            // bad client or lag: player is not visible on client
            active.unset_map_flag();
            return Ok(());
        }

        active.clear_pending_action()?;
        active.player.set_interaction(
            InteractionTarget::Player { pid: self.pid },
            ServerTriggerType::ApPlayerT as u8,
            true,
        );
        active.player.interaction.target_subject_com = Some(spell_com);
        active.player.opcalled = true;

        Ok(())
    }
}
