use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_entity::InteractionTarget;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client::opnpcu::OpNpcU;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpNpcU` (use item on NPC) client protocol message.
///
/// Processes a "use held item on NPC" interaction. Validates that the use
/// component (`com`) is usable and visible, that the used item exists at the
/// given slot, and that the NPC exists, is not delayed, and is visible to the
/// player. Sets up an approach-style interaction (`ApNpcU`) that will trigger
/// the corresponding script once the player reaches the NPC.
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
/// * Sets `last_use_item` and `last_use_slot` on the player.
/// * Sets up an `InteractionTarget::Npc` interaction with approach mode and sets `opcalled`.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
impl ClientGameHandler for OpNpcU {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            active.unset_map_flag();
            return Ok(());
        }

        let engine = engine();

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

        let npc_delayed = engine.get_npc(self.nid).map(|n| n.npc.state.delayed);
        let Some(npc_delayed) = npc_delayed else {
            // bad client or lag: npc does not exist
            active.unset_map_flag();
            return Ok(());
        };
        if npc_delayed {
            // normal: cannot interact with delayed npcs
            active.unset_map_flag();
            return Ok(());
        }

        if !active.player.build_area.npcs.contains(self.nid) {
            // bad client or lag: npc is not visible on client
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

        active.player.set_interaction(
            InteractionTarget::Npc { nid: self.nid },
            ServerTriggerType::ApNpcU as u8,
            true,
        );
        active.player.opcalled = true;

        Ok(())
    }
}
