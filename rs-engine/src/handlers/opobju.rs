use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client::opobju::OpObjU;
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};
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

        let engine = engine();

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

        let Some(use_interface) = engine.interfaces().get_by_id(self.com) else {
            // bad client: component is not acceptable for this packet
            active.unset_map_flag();
            return Ok(());
        };

        if !use_interface.usable {
            // bad client: component is not acceptable for this packet
            active.unset_map_flag();
            return Ok(());
        }

        if !active.player.is_interface_visible(use_interface.root_layer) {
            // bad client or lag: component is not visible
            active.unset_map_flag();
            return Ok(());
        }

        let inv_id = active
            .player
            .inv_transmits
            .iter()
            .find(|(_, coms)| coms.contains(&self.com))
            .map(|(id, _)| *id);

        let Some(inv_id) = inv_id else {
            // bad client or lag: inventory is not transmitted to client
            active.unset_map_flag();
            return Ok(());
        };

        let inv = engine.invs().get_by_id(inv_id);
        let shared = inv.is_some_and(|t| t.scope == InvScope::Shared);

        let Some(inventory) = (if shared {
            engine_mut().get_shared_inv_mut(inv_id)
        } else {
            active.player.invs.get_mut(&inv_id)
        }) else {
            // bad client or lag: inventory is not transmitted to client
            active.unset_map_flag();
            return Ok(());
        };

        if !inventory.valid_slot(self.slot) {
            // bad client: real inventory is smaller
            active.unset_map_flag();
            return Ok(());
        }

        if !inventory.has_at(self.slot, self.use_obj) {
            // bad client or lag: item does not exist in inventory
            active.unset_map_flag();
            return Ok(());
        }

        active.clear_pending_action()?;

        if engine
            .objs()
            .get_by_id(self.use_obj)
            .is_some_and(|o| o.members)
            && !engine.members
        {
            active.message_game("To use this item please login to a members' server.");
            active.unset_map_flag();
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
