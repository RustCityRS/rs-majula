use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{action_target, npc_target_ok, spell_component_ok};
use rs_entity::InteractionTarget;
use rs_protocol::network::game::client::opnpct::OpNpcT;
use rs_vm::ScriptError;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpNpcT` (cast spell on NPC) client protocol message.
///
/// Processes a "use spell on NPC" interaction. Validates that the spell component
/// (`com`) is acceptable for NPC targets and visible, that the NPC exists, is not
/// delayed, and is visible to the player. Sets up an approach-style interaction
/// (`ApNpcT`) keyed on the spell component id that will trigger the corresponding
/// script once the player reaches the NPC.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success or if the player/NPC is delayed or the target is invalid.
///
/// # Side Effects
///
/// * Clears pending action and unsets the map flag on early exit conditions.
/// * Sets up an `InteractionTarget::Npc` interaction with approach mode on the player.
/// * Records the spell component as the interaction subject and sets `opcalled`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
impl ClientGameHandler for OpNpcT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            active.unset_map_flag();
            return Ok(());
        }

        let spell_com = self.com;
        if !spell_component_ok(active, spell_com, action_target::NPC) {
            // bad client or lag: component is not acceptable for this packet, or not visible
            active.unset_map_flag();
            return Ok(());
        }

        if !npc_target_ok(active, self.nid) {
            // bad client or lag: npc does not exist, is delayed, or is not visible on client
            active.unset_map_flag();
            return Ok(());
        }

        active.clear_pending_action()?;
        active.player.set_interaction(
            InteractionTarget::Npc { nid: self.nid },
            ServerTriggerType::ApNpcT as u8,
            true,
        );
        active.player.interaction.target_subject_com = Some(spell_com);
        active.player.opcalled = true;

        Ok(())
    }
}
