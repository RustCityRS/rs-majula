use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client::opheldu::OpHeldU;
use rs_protocol::network::game::info_prot::PlayerInfoProt;
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, cache};
use rs_vm::state::ScriptState;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpHeldU` (use item on item) client protocol message.
///
/// Processes a "use item on item" interaction where the player uses one held
/// item on another held item. Validates both source and target interfaces for
/// visibility and usability, verifies both items exist at their respective slots,
/// then searches for a matching script trigger in priority order:
///
/// 1. `[opheldu,b]` - Script keyed on the target object ID.
/// 2. `[opheldu,a]` - Script keyed on the source object ID (swaps item/slot tracking).
/// 3. `[opheldu,b_category]` - Script keyed on the target object's category.
/// 4. `[opheldu,a_category]` - Script keyed on the source object's category (swaps item/slot tracking).
///
/// If both items are members-only and the server is not a members' server, the player
/// receives a notification message instead of executing the script.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success, if the player is delayed, if the source and target
///   components differ, or if no matching trigger is found.
/// * `Err(ScriptError)` if interface/inventory/object validation fails or a script
///   execution error occurs.
///
/// # Side Effects
///
/// * Sets `last_item`, `last_slot`, `last_use_item`, and `last_use_slot` on the player.
/// * May swap `last_item`/`last_use_item` and `last_slot`/`last_use_slot` if a trigger
///   is found on the alternate object.
/// * Clears pending action and face entity.
/// * Runs the matched `OpHeldU` trigger script.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `engine_mut().run_script_by_state`
impl ClientGameHandler for OpHeldU {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed {
            return Ok(());
        }

        if self.com != self.com2 {
            return Ok(());
        }

        let Some(interface) = cache().interfaces.get_by_id(self.com) else {
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

        if !interface.usable {
            return Err(ScriptError::Client(format!(
                "Not operable with id: {}",
                self.com
            )));
        };

        let Some(interface2) = cache().interfaces.get_by_id(self.com2) else {
            return Err(ScriptError::Client(format!(
                "No interface with id: {}",
                self.com2
            )));
        };

        if !active.player.is_interface_visible(interface2.root_layer) {
            return Err(ScriptError::Client(format!(
                "Interface is not visible: {}",
                interface2.root_layer
            )));
        }

        if !interface2.usable {
            return Err(ScriptError::Client(format!(
                "Not operable with id: {}",
                self.com2
            )));
        };

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

        let inv_id2 = active
            .player
            .inv_transmits
            .iter()
            .find(|(_, coms)| coms.contains(&self.com2))
            .map(|(id, _)| *id);

        let Some(inv_id2) = inv_id2 else {
            return Err(ScriptError::Client(format!(
                "No inv transmit for interface with id: {}",
                self.com
            )));
        };

        let inv = cache().invs.get_by_id(inv_id);
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
            active.player.move_request = true;
            active.clear_pending_action()?;
            return Ok(());
        }

        let Some(obj) = cache().objs.get_by_id(self.obj) else {
            return Err(ScriptError::Client(format!(
                "Invalid slot: {} with obj: {}",
                self.slot, self.obj
            )));
        };

        let inv2 = cache().invs.get_by_id(inv_id2);
        let shared2 = inv2.is_some_and(|t| t.scope == InvScope::Shared);

        let Some(inventory2) = (if shared2 {
            engine_mut().get_shared_inv_mut(inv_id2)
        } else {
            active.player.invs.get_mut(&inv_id2)
        }) else {
            return Err(ScriptError::Client(format!(
                "Inv {} not found for com: {}",
                inv_id2, self.com
            )));
        };

        if !inventory2.valid_slot(self.slot2) {
            return Err(ScriptError::Client(format!("Invalid slot: {}", self.slot2)));
        }

        if !inventory2.has_at(self.slot2, self.obj2) {
            active.player.move_request = true;
            active.clear_pending_action()?;
            return Ok(());
        }

        let Some(obj2) = cache().objs.get_by_id(self.obj2) else {
            return Err(ScriptError::Client(format!(
                "Invalid slot: {} with obj: {}",
                self.slot2, self.obj2
            )));
        };

        active.player.last_item = Some(self.obj);
        active.player.last_slot = Some(self.slot);
        active.player.last_use_item = Some(self.obj2);
        active.player.last_use_slot = Some(self.slot2);

        active.clear_pending_action()?;
        active.player.info.face_entity = None;
        active.player.info.masks |= PlayerInfoProt::FaceEntity as u16;

        if (obj.members || obj2.members) && !engine().members {
            active.message_game("To use this item please login to a members' server.");
            return Ok(());
        }

        let base = ServerTriggerType::OpHeldU as i32;

        // [opheldu,b]
        let key = base | (0x2 << 8) | ((obj.id as i32) << 10);
        let mut script = engine_mut().scripts.get_by_lookup(key).cloned();

        // [opheldu,a]
        if script.is_none() {
            let key = base | (0x2 << 8) | ((obj2.id as i32) << 10);
            script = engine_mut().scripts.get_by_lookup(key).cloned();
            if script.is_some() {
                std::mem::swap(
                    &mut active.player.last_item,
                    &mut active.player.last_use_item,
                );
                std::mem::swap(
                    &mut active.player.last_slot,
                    &mut active.player.last_use_slot,
                );
            }
        }

        // [opheld,b_category]
        if script.is_none()
            && let Some(cat) = obj.category
        {
            let key = base | (0x1 << 8) | ((cat as i32) << 10);
            script = engine_mut().scripts.get_by_lookup(key).cloned();
        }

        // [opheld,a_category]
        if script.is_none()
            && let Some(cat) = obj2.category
        {
            let key = base | (0x1 << 8) | ((cat as i32) << 10);
            script = engine_mut().scripts.get_by_lookup(key).cloned();
            if script.is_some() {
                std::mem::swap(
                    &mut active.player.last_item,
                    &mut active.player.last_use_item,
                );
                std::mem::swap(
                    &mut active.player.last_slot,
                    &mut active.player.last_use_slot,
                );
            }
        }

        if let Some(script) = script {
            let uid = active.player.uid;
            let state = ScriptState::init(script, Some(ScriptSubject::Player(uid)), None, None);
            engine_mut().run_script_by_state(
                state,
                Some(ScriptSubject::Player(uid)),
                Some(true),
                None,
            )?;
        } else {
            #[cfg(debug_assertions)]
            active.message_game(&format!(
                "No trigger for [opheldu,{}]",
                obj.debugname().unwrap_or(&obj.id.to_string())
            ));
            active.message_game("Nothing interesting happens.");
        }

        Ok(())
    }
}
