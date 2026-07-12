use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_entity::InteractionTarget;
use rs_protocol::network::game::client::opnpc1::OpNpc1;
use rs_protocol::network::game::client::opnpc2::OpNpc2;
use rs_protocol::network::game::client::opnpc3::OpNpc3;
use rs_protocol::network::game::client::opnpc4::OpNpc4;
use rs_protocol::network::game::client::opnpc5::OpNpc5;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpNpc1` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 1.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpNpc1 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(1, self.nid, active)
    }
}

/// Handles the `OpNpc2` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 2.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpNpc2 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(2, self.nid, active)
    }
}

/// Handles the `OpNpc3` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 3.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpNpc3 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(3, self.nid, active)
    }
}

/// Handles the `OpNpc4` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 4.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpNpc4 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(4, self.nid, active)
    }
}

/// Handles the `OpNpc5` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 5.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpNpc5 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(5, self.nid, active)
    }
}

/// Shared handler for NPC operations (ops 1-5).
///
/// Processes a right-click menu operation on an NPC. Validates the NPC exists,
/// is not delayed, is within the player's build area, and has the requested
/// operation option. Sets up an approach-style interaction (`ApNpc1`-`ApNpc5`)
/// that will trigger the corresponding script once the player reaches the NPC.
///
/// # Arguments
///
/// * `op` - The operation number (1-5), corresponding to a right-click menu option.
/// * `nid` - The NPC instance ID (slot index in the engine's NPC list).
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success or if the player/NPC is delayed or the target is invalid.
///
/// # Side Effects
///
/// * Clears pending action and unsets map flag on early exit conditions.
/// * Sets up an `InteractionTarget::Npc` interaction with approach mode on the player.
/// * Sets `opcalled` to `true` on the player.
///
/// # Call Stack
///
/// **Called by:** `OpNpc1::handle` through `OpNpc5::handle`
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
fn handle(op: u8, nid: u16, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    if active.player.state.delayed {
        active.unset_map_flag();
        return Ok(());
    }

    let engine = engine();

    let npc_info = engine
        .get_npc(nid)
        .map(|n| (n.npc.state.delayed, n.npc.uid.id()));

    let Some((npc_delayed, npc_type_id)) = npc_info else {
        active.unset_map_flag();
        active.clear_pending_action()?;
        return Ok(());
    };

    if npc_delayed {
        active.unset_map_flag();
        active.clear_pending_action()?;
        return Ok(());
    }

    if !active.player.build_area.npcs.contains(nid) {
        active.unset_map_flag();
        active.clear_pending_action()?;
        return Ok(());
    }

    let npc_type = engine.npcs().get_by_id(npc_type_id);
    if let Some(nt) = &npc_type {
        if let Some(ops) = &nt.op {
            if ops.get((op - 1) as usize).is_none_or(|o| o.is_none()) {
                active.unset_map_flag();
                active.clear_pending_action()?;
                return Ok(());
            }
        } else {
            active.unset_map_flag();
            active.clear_pending_action()?;
            return Ok(());
        }
    }

    let mode = match op {
        1 => ServerTriggerType::ApNpc1,
        2 => ServerTriggerType::ApNpc2,
        3 => ServerTriggerType::ApNpc3,
        4 => ServerTriggerType::ApNpc4,
        _ => ServerTriggerType::ApNpc5,
    };

    let target = InteractionTarget::Npc { nid };

    active.clear_pending_action()?;
    active.player.set_interaction(target, mode as u8, true);
    active.player.opcalled = true;

    Ok(())
}
