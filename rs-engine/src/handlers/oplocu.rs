use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client::oplocu::OpLocU;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpLocU` (use item on location) client protocol message.
///
/// Processes a "use held item on location" interaction. Validates that the target
/// coordinates are within the player's build area, that the location exists, that
/// the use component (`com`) is usable and visible, and that the used item exists
/// at the given slot. Sets up an approach-style interaction (`ApLocU`) that will
/// trigger the corresponding script once the player reaches the location.
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
/// * Sets up an `InteractionTarget::Loc` interaction with approach mode and sets `opcalled`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
impl ClientGameHandler for OpLocU {
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

        if !inventory.has_at(self.slot, self.obj) {
            // bad client or lag: item does not exist in inventory
            active.unset_map_flag();
            return Ok(());
        }

        active.clear_pending_action()?;

        if engine.objs().get_by_id(self.obj).is_some_and(|o| o.members) && !engine.members {
            active.message_game("To use this item please login to a members' server.");
            active.unset_map_flag();
            return Ok(());
        }

        active.player.last_use_item = Some(self.obj);
        active.player.last_use_slot = Some(self.slot);

        active
            .player
            .set_interaction(target, ServerTriggerType::ApLocU as u8, true);
        active.player.opcalled = true;

        Ok(())
    }
}
