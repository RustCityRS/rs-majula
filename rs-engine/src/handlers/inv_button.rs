use crate::active_player::ActivePlayer;
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_pack::types::InvScope;
use rs_protocol::network::game::client::inv_button1::InvButton1;
use rs_protocol::network::game::client::inv_button2::InvButton2;
use rs_protocol::network::game::client::inv_button3::InvButton3;
use rs_protocol::network::game::client::inv_button4::InvButton4;
use rs_protocol::network::game::client::inv_button5::InvButton5;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `InvButton1` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 1.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for InvButton1 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(1, self.obj, self.slot, self.com, active)
    }
}

/// Handles the `InvButton2` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 2.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for InvButton2 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(2, self.obj, self.slot, self.com, active)
    }
}

/// Handles the `InvButton3` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 3.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for InvButton3 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(3, self.obj, self.slot, self.com, active)
    }
}

/// Handles the `InvButton4` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 4.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for InvButton4 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(4, self.obj, self.slot, self.com, active)
    }
}

/// Handles the `InvButton5` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 5.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for InvButton5 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(5, self.obj, self.slot, self.com, active)
    }
}

/// Shared handler for inventory button click operations (ops 1-5).
///
/// Processes a click on an inventory interface button. Validates the interface
/// visibility, checks that the inventory operation option exists, verifies the
/// object is present at the specified slot, and then runs the corresponding
/// `InvButton1`-`InvButton5` server trigger script.
///
/// # Arguments
///
/// * `op` - The button operation number (1-5).
/// * `obj` - The object ID in the clicked inventory slot.
/// * `slot` - The inventory slot index that was clicked.
/// * `com` - The interface component ID that was clicked.
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success, if the player is delayed, or if no trigger is found
///   (in which case a game message is sent instead).
/// * `Err(ScriptError)` if interface/inventory validation fails or a script
///   execution error occurs.
///
/// # Side Effects
///
/// * Sets `last_item` and `last_slot` on the player.
/// * Runs the corresponding `ServerTriggerType::InvButton{N}` script.
///
/// # Call Stack
///
/// **Called by:** `InvButton1::handle` through `InvButton5::handle`
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

    let engine = engine();
    let interfaces = engine.interfaces();
    let invs = engine.invs();

    let Some(interface) = interfaces.get_by_id(com) else {
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

    let Some(iop) = &interface.iop else {
        return Err(ScriptError::Client(format!(
            "No iop for interface with id: {}",
            com
        )));
    };

    if iop.get(op - 1).is_none_or(|o| o.is_none()) {
        return Err(ScriptError::Client(format!(
            "No iop option {} for interface with id: {}",
            op, com
        )));
    }

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

    let inv = invs.get_by_id(inv_id);
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

    active.player.last_item = Some(obj);
    active.player.last_slot = Some(slot);

    let protect = interfaces
        .get_by_id(interface.root_layer as u16)
        .is_some_and(|root| !root.overlay);

    let trigger = match op {
        1 => ServerTriggerType::InvButton1,
        2 => ServerTriggerType::InvButton2,
        3 => ServerTriggerType::InvButton3,
        4 => ServerTriggerType::InvButton4,
        5 => ServerTriggerType::InvButton5,
        _ => {
            return Err(ScriptError::Client(format!(
                "Trigger not found for op: {}",
                op
            )));
        }
    };

    let result = engine_mut().run_script_by_trigger(
        (trigger, Some(com), None),
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
                "No trigger for [inv_button{},{}]",
                op,
                interface.com_name.as_deref().unwrap_or(&com.to_string())
            ));
        }
        other => other?,
    }

    Ok(())
}
