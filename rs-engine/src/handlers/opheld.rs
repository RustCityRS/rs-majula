use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::{cache, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client::opheld1::OpHeld1;
use rs_protocol::network::game::client::opheld2::OpHeld2;
use rs_protocol::network::game::client::opheld3::OpHeld3;
use rs_protocol::network::game::client::opheld4::OpHeld4;
use rs_protocol::network::game::client::opheld5::OpHeld5;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpHeld1` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 1.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpHeld1 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(1, self.obj, self.slot, self.com, active)
    }
}

/// Handles the `OpHeld2` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 2.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpHeld2 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(2, self.obj, self.slot, self.com, active)
    }
}

/// Handles the `OpHeld3` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 3.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpHeld3 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(3, self.obj, self.slot, self.com, active)
    }
}

/// Handles the `OpHeld4` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 4.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpHeld4 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(4, self.obj, self.slot, self.com, active)
    }
}

/// Handles the `OpHeld5` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 5.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpHeld5 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(5, self.obj, self.slot, self.com, active)
    }
}

/// Shared handler for held item operations (ops 1-5).
///
/// Processes a right-click menu operation on a held (inventory) item. Validates the
/// interface visibility, checks the interface is operable, verifies the object exists
/// at the specified slot with the correct ID, and checks that the item has the
/// requested interaction option (iop). Then runs the corresponding `OpHeld1`-`OpHeld5`
/// server trigger script, looking up by both object ID and category.
///
/// # Arguments
///
/// * `op` - The operation number (1-5), corresponding to a right-click menu option.
/// * `obj` - The object ID in the inventory slot that was operated on.
/// * `slot` - The inventory slot index of the item.
/// * `com` - The interface component ID containing the inventory.
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success, if the player is delayed, or if no trigger is found
///   (in which case a game message is sent instead).
/// * `Err(ScriptError)` if interface/inventory/object validation fails or a script
///   execution error occurs.
///
/// # Side Effects
///
/// * Sets `last_item` and `last_slot` on the player.
/// * Clears pending action if the interface is not a main modal.
/// * Clears face entity and move request on the player.
/// * Runs the corresponding `ServerTriggerType::OpHeld{N}` script.
///
/// # Call Stack
///
/// **Called by:** `OpHeld1::handle` through `OpHeld5::handle`
/// **Calls:** `engine_mut().run_script_by_trigger`
fn handle(
    op: usize,
    obj: u16,
    slot: u16,
    com: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    if active.player.state.delayed {
        return Ok(());
    }

    let Some(interface) = cache().interfaces.get_by_id(com) else {
        return Err(ScriptError::Client(format!(
            "No interface with id: {}",
            com
        )));
    };

    if !active.player.is_interface_visible(interface.root_layer) {
        return Err(ScriptError::Client(format!(
            "Interface is not visible: {}",
            interface.root_layer
        )));
    }

    if !interface.operable {
        return Err(ScriptError::Client(format!(
            "Not operable with id: {}",
            com
        )));
    };

    let inv_id = active
        .player
        .inv_transmits
        .iter()
        .find(|(_, coms)| coms.contains(&com))
        .map(|(id, _)| *id);

    let Some(inv_id) = inv_id else {
        return Err(ScriptError::Client(format!(
            "No inv transmit for interface with id: {}",
            com
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
            inv_id, com
        )));
    };

    if !inventory.valid_slot(slot) {
        return Err(ScriptError::Client(format!("Invalid slot: {}", slot)));
    }

    if !inventory.has_at(slot, obj) {
        /*return Err(ScriptError::Client(format!(
            "Invalid slot: {} with obj: {}",
            slot, obj
        )));*/
        return Ok(());
    }

    let Some(obj) = cache().objs.get_by_id(obj) else {
        return Err(ScriptError::Client(format!(
            "Invalid slot: {} with obj: {}",
            slot, obj
        )));
    };

    let Some(iop) = &obj.iop else {
        return Err(ScriptError::Client(format!(
            "No iop for obj with id: {}",
            obj.id
        )));
    };

    if iop.get(op - 1).is_none_or(|o| o.is_none()) {
        return Err(ScriptError::Client(format!(
            "No iop option {} for obj with id: {}",
            op, obj.id
        )));
    }

    active.player.last_item = Some(obj.id);
    active.player.last_slot = Some(slot);

    if active.player.modal_main.map(|v| v as i32) != Some(interface.root_layer) {
        active.clear_pending_action()?;
    }

    active.player.move_request = false; // uses the dueling ring op to move whilst busy & queue pending: https://youtu.be/GPfN3Isl2rM
    active.player.info.clear_face_entity_player();

    let trigger = match op {
        1 => ServerTriggerType::OpHeld1,
        2 => ServerTriggerType::OpHeld2,
        3 => ServerTriggerType::OpHeld3,
        4 => ServerTriggerType::OpHeld4,
        5 => ServerTriggerType::OpHeld5,
        _ => {
            return Err(ScriptError::Client(format!(
                "Trigger not found for op: {}",
                op
            )));
        }
    };

    let result = engine_mut().run_script_by_trigger(
        (
            trigger,
            Some(obj.id),
            Some(obj.category.map(|x| x as i32).unwrap_or(-1)),
        ),
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
                "No trigger for [opheld{},{}]",
                op,
                obj.debugname().unwrap_or(&obj.id.to_string())
            ));
        }
        other => other?,
    }

    Ok(())
}
