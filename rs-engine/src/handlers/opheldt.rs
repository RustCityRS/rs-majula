use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client::opheldt::OpHeldT;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;

/// `ComActionTarget::HELD` bit: the component may be cast on a held (inventory) item.
const ACTION_TARGET_HELD: u16 = 0x10;

/// Handles the `OpHeldT` (cast spell on held item) client protocol message.
///
/// Processes a "use spell on held item" interaction where the player casts a
/// magic spell component (`com2`) onto a held inventory item (`com`). Validates
/// that the spell component is acceptable for held targets and visible, that the
/// item interface is usable and visible, and that the item exists at the given
/// slot. Then runs the `OpHeldT` server trigger keyed on the spell component id.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success, if the player is delayed, or if no matching trigger is
///   found (in which case a game message is sent instead).
/// * `Err(ScriptError)` if interface/inventory/object validation fails or a script
///   execution error occurs.
///
/// # Side Effects
///
/// * Sets `last_item` and `last_slot` on the player.
/// * Clears pending action and face entity.
/// * Runs the `OpHeldT` trigger script keyed on the spell component id.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `engine_mut().run_script_by_trigger`
impl ClientGameHandler for OpHeldT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            // normal: cannot interact while delayed
            return Ok(());
        }

        let engine = engine();
        let interfaces = engine.interfaces();
        let invs = engine.invs();

        let spell_com = self.com2;
        let Some(spell_interface) = interfaces.get_by_id(spell_com) else {
            return Err(ScriptError::Client(format!(
                "No interface with id: {}",
                spell_com
            )));
        };

        if spell_interface.action_target & ACTION_TARGET_HELD == 0 {
            return Err(ScriptError::Client(format!(
                "Component is not acceptable for opheldt: {}",
                spell_com
            )));
        }

        if !active
            .player
            .is_interface_visible(spell_interface.root_layer)
        {
            return Err(ScriptError::Client(format!(
                "Interface is not visible: {}",
                spell_interface.root_layer
            )));
        }

        let Some(interface) = interfaces.get_by_id(self.com) else {
            return Err(ScriptError::Client(format!(
                "No interface with id: {}",
                self.com
            )));
        };

        if !interface.usable {
            return Err(ScriptError::Client(format!(
                "Not usable with id: {}",
                self.com
            )));
        }

        if !active.player.is_interface_visible(interface.root_layer) {
            return Err(ScriptError::Client(format!(
                "Interface is not visible: {}",
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

        if !inventory.valid_slot(self.slot) {
            return Err(ScriptError::Client(format!("Invalid slot: {}", self.slot)));
        }

        if !inventory.has_at(self.slot, self.obj) {
            /*return Err(ScriptError::Client(format!(
                "Invalid slot: {} with obj: {}",
                self.slot, self.obj
            )));*/
            return Ok(());
        }

        active.player.last_item = Some(self.obj);
        active.player.last_slot = Some(self.slot);

        active.clear_pending_action()?;
        active.player.info.clear_face_entity_player();

        let result = engine_mut().run_script_by_trigger(
            (ServerTriggerType::OpHeldT, Some(spell_com), None),
            Some(ScriptSubject::Player(active.player.uid)),
            None,
            Some(true),
            None,
            None,
        );

        match result {
            Err(ScriptError::TriggerNotFound(_)) => {
                #[cfg(debug_assertions)]
                active.message_game(&format!(
                    "No trigger for [opheldt,{}]",
                    spell_interface.com_name.as_deref().unwrap_or("?")
                ));
                active.message_game("Nothing interesting happens.");
            }
            other => other?,
        }

        Ok(())
    }
}
