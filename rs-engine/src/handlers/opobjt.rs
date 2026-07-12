use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::opobjt::OpObjT;
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};
use rs_vm::trigger::ServerTriggerType;

/// `ComActionTarget::OBJ` bit: the component may be cast on a ground object.
const ACTION_TARGET_OBJ: u16 = 0x1;

/// Handles the `OpObjT` (cast spell on ground object) client protocol message.
///
/// Processes a "use spell on ground object" interaction. Validates that the spell
/// component (`com`) is acceptable for object targets and visible, that the target
/// coordinates are within the player's build area, and that the ground object
/// exists in the zone. Sets up an approach-style interaction (`ApObjT`) keyed on
/// the spell component id that will trigger the corresponding script once the
/// player reaches the object.
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
/// * Sets up an `InteractionTarget::Obj` interaction with approach mode on the player.
/// * Records the spell component as the interaction subject and sets `opcalled`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
impl ClientGameHandler for OpObjT {
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

        if spell_interface.action_target & ACTION_TARGET_OBJ == 0 {
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
        let receiver = active.uid().username37();
        let Some(zone) = engine.zones.zone(self.x, y, self.z) else {
            // bad client or lag: obj does not exist
            active.unset_map_flag();
            return Ok(());
        };
        let Some(idx) = zone.get_obj(self.x, self.z, self.obj, Some(receiver)) else {
            // bad client or lag: obj does not exist
            active.unset_map_flag();
            return Ok(());
        };

        let target = InteractionTarget::Obj {
            coord: CoordGrid::new(self.x, y, self.z),
            id: self.obj,
            count: zone.objs[idx].count(),
        };

        active.clear_pending_action()?;
        active
            .player
            .set_interaction(target, ServerTriggerType::ApObjT as u8, true);
        active.player.interaction.target_subject_com = Some(spell_com);
        active.player.opcalled = true;

        Ok(())
    }
}
