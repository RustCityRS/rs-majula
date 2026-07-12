use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::oploct::OpLocT;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::trigger::ServerTriggerType;

/// `ComActionTarget::LOC` bit: the component may be cast on a location.
const ACTION_TARGET_LOC: u16 = 0x4;

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

        let engine = engine();

        let spell_com = self.com;
        let Some(spell_interface) = engine.interfaces().get_by_id(spell_com) else {
            // bad client: component is not acceptable for this packet
            active.unset_map_flag();
            return Ok(());
        };

        if spell_interface.action_target & ACTION_TARGET_LOC == 0 {
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

        let origin_x = active.player.build_area.origin.x() as i32;
        let origin_z = active.player.build_area.origin.z() as i32;
        if (self.x as i32) < origin_x - 52
            || (self.x as i32) > origin_x + 52
            || (self.z as i32) < origin_z - 52
            || (self.z as i32) > origin_z + 52
        {
            // bad client: tile is not visible on client
            active.unset_map_flag();
            return Ok(());
        }

        let y = active.player.pathing.coord.y();
        let Some(zone) = engine.zones.zone(self.x, y, self.z) else {
            // bad client or lag: loc does not exist
            active.unset_map_flag();
            return Ok(());
        };
        let Some(idx) = zone.get_loc(self.x, self.z, self.loc) else {
            // bad client or lag: loc does not exist
            active.unset_map_flag();
            return Ok(());
        };
        let loc = &zone.locs[idx];

        let loc_type = engine.locs().get_by_id(self.loc);
        let width = loc_type.map(|lt| lt.width).unwrap_or(1);
        let length = loc_type.map(|lt| lt.length).unwrap_or(1);
        let coord = CoordGrid::new(self.x, y, self.z);
        let target = InteractionTarget::Loc {
            coord,
            id: self.loc,
            width,
            length,
            shape: loc.shape(),
            angle: loc.angle(),
            layer: loc.layer(),
        };

        active.clear_pending_action()?;
        active
            .player
            .set_interaction(target, ServerTriggerType::ApLocT as u8, true);
        active.player.interaction.target_subject_com = Some(spell_com);
        active.player.opcalled = true;

        Ok(())
    }
}
