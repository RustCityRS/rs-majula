use crate::active_player::ActivePlayer;
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client::inv_buttond::InvButtonD;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `InvButtonD` (inventory button drag) client protocol message.
///
/// Processes a drag operation within an inventory interface, used when the player
/// drags an item from one slot to another. Validates the interface visibility,
/// checks that the interface is draggable, verifies both source and target slots,
/// and runs the `ServerTriggerType::InvButtonD` script.
///
/// If the player is delayed, the handler sends a partial inventory update for both
/// slots to resynchronize the client without executing the drag.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success, if the player is delayed, or if no trigger is found.
/// * `Err(ScriptError)` if interface/inventory/slot validation fails or a script
///   execution error occurs.
///
/// # Side Effects
///
/// * Sets `last_slot` and `last_target_slot` on the player.
/// * Runs the `ServerTriggerType::InvButtonD` script.
/// * If delayed, sends a partial inventory update to resynchronize the client.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `engine_mut().run_script_by_trigger`
impl ClientGameHandler for InvButtonD {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let engine = engine();
        let interfaces = engine.interfaces();
        let invs = engine.invs();

        let Some(interface) = interfaces.get_by_id(self.com) else {
            return Err(ScriptError::Client(format!(
                "No interface with id: {}",
                self.com
            )));
        };

        if !active.player.is_interface_visible(interface.root_layer) {
            return Err(ScriptError::Client(format!(
                "Interface is not visible: {}",
                interface.root_layer
            )));
        }

        if !interface.draggable {
            return Err(ScriptError::Client(format!(
                "Interface is not draggable: {}",
                interface.root_layer
            )));
        }

        let inv_id = active
            .player
            .inv_transmits
            .iter()
            .find(|(_, coms)| coms.contains(&self.com))
            .map(|(id, _)| *id);

        let Some(inv_id) = inv_id else {
            return Err(ScriptError::Client(format!(
                "No inv transmit for interface with id: {}",
                self.com
            )));
        };

        let inv = invs.get_by_id(inv_id);
        let shared = inv.is_some_and(|t| t.scope == InvScope::Shared);
        let delayed = active.player.state.delayed;

        let Some(inventory) = (if shared {
            engine_mut().get_shared_inv_mut(inv_id)
        } else {
            active.player.invs.get_mut(&inv_id)
        }) else {
            return Err(ScriptError::Client(format!(
                "Inv {} not found for com: {}",
                inv_id, self.com
            )));
        };

        for slot in [self.slot, self.slot2] {
            if !inventory.valid_slot(slot) {
                return Err(ScriptError::Client(format!("Invalid slot: {}", slot)));
            }
        }

        if inventory.get(self.slot).is_none() {
            return Err(ScriptError::Client(format!("Invalid slot: {}", self.slot)));
        }

        if delayed {
            let partial = [
                (
                    self.slot,
                    inventory.get(self.slot).map(|i| (i.obj, i.num as i32)),
                ),
                (
                    self.slot2,
                    inventory.get(self.slot2).map(|i| (i.obj, i.num as i32)),
                ),
            ];
            active.update_inv_partial(self.com, &partial);
            return Ok(());
        }

        active.player.last_slot = Some(self.slot);
        active.player.last_target_slot = Some(self.slot2);

        let protect = interfaces
            .get_by_id(interface.root_layer as u16)
            .is_some_and(|root| !root.overlay);

        let result = engine_mut().run_script_by_trigger(
            (ServerTriggerType::InvButtonD, Some(self.com), None),
            Some(ScriptSubject::Player(active.player.uid)),
            None,
            Some(protect),
            None,
            None,
        );

        match result {
            Err(ScriptError::TriggerNotFound(_)) => {
                #[cfg(debug_assertions)]
                active.message_game(&format!(
                    "No trigger for [inv_buttond,{}]",
                    interface
                        .com_name
                        .as_deref()
                        .unwrap_or(&self.com.to_string())
                ));
            }
            other => other?,
        }

        Ok(())
    }
}
