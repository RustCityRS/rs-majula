use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    action_target, in_build_area, loc_target, spell_component_ok, zone_loc,
};
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::oploct::OpLocT;
use rs_vm::ScriptError;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpLocT` (cast spell on location) client protocol message.
///
/// Processes a "use spell on location" interaction. Validates that the spell
/// component (`com`) is acceptable for location targets and visible, that the
/// target coordinates are within the player's build area, and that the location
/// exists in the zone. Sets up an approach-style interaction (`ApLocT`) keyed on
/// the spell component id that will trigger the corresponding script once the
/// player reaches the location.
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
/// * Sets up an `InteractionTarget::Loc` interaction with approach mode on the player.
/// * Records the spell component as the interaction subject and sets `opcalled`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
impl ClientGameHandler for OpLocT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            active.unset_map_flag();
            return Ok(());
        }

        let spell_com = self.com;
        if !spell_component_ok(active, spell_com, action_target::LOC) {
            // bad client or lag: component is not acceptable for this packet, or not visible
            active.unset_map_flag();
            return Ok(());
        }

        if !in_build_area(active, self.x, self.z) {
            // bad client: tile is not visible on client
            active.unset_map_flag();
            return Ok(());
        }

        let y = active.player.pathing.coord.y();
        let Some(loc) = zone_loc(self.x, y, self.z, self.loc) else {
            // bad client or lag: loc does not exist
            active.unset_map_flag();
            return Ok(());
        };

        let target = loc_target(self.loc, CoordGrid::new(self.x, y, self.z), loc);

        active.clear_pending_action()?;
        active
            .player
            .set_interaction(target, ServerTriggerType::ApLocT as u8, true);
        active.player.interaction.target_subject_com = Some(spell_com);
        active.player.opcalled = true;

        Ok(())
    }
}
