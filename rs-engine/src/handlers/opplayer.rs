use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::engine::engine_mut;
use crate::handlers::ClientGameHandler;
use rs_entity::InteractionTarget;
use rs_protocol::network::game::client::opplayer1::OpPlayer1;
use rs_protocol::network::game::client::opplayer2::OpPlayer2;
use rs_protocol::network::game::client::opplayer3::OpPlayer3;
use rs_protocol::network::game::client::opplayer4::OpPlayer4;
#[cfg(since_254)]
use rs_protocol::network::game::client::opplayer5::OpPlayer5;
use rs_vm::ScriptError;
use rs_vm::trigger::ServerTriggerType;

/// Handles the `OpPlayer1` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 1.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpPlayer1 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(1, self.pid, active)
    }
}

/// Handles the `OpPlayer2` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 2.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpPlayer2 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(2, self.pid, active)
    }
}

/// Handles the `OpPlayer3` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 3.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpPlayer3 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(3, self.pid, active)
    }
}

/// Handles the `OpPlayer4` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 4.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
impl ClientGameHandler for OpPlayer4 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(4, self.pid, active)
    }
}

/// Handles the `OpPlayer5` client protocol message.
///
/// Delegates to the shared [`handle`] function with operation 5.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`handle`]
#[cfg(since_254)]
impl ClientGameHandler for OpPlayer5 {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(5, self.pid, active)
    }
}

/// Shared handler for player-on-player operations (ops 1-4).
///
/// Processes a right-click menu operation on another player (e.g., trade, follow,
/// challenge). Validates the target player exists and is within the active player's
/// build area, then sets up an approach-style interaction (`ApPlayer1`-`ApPlayer4`)
/// that will trigger the corresponding script once the active player reaches the target.
///
/// # Arguments
///
/// * `op` - The operation number (1-4), corresponding to a right-click menu option.
/// * `pid` - The player ID (slot index) of the target player.
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success or if the player is delayed / target is invalid.
///
/// # Side Effects
///
/// * Clears pending action and unsets map flag on early exit conditions.
/// * Sets up an `InteractionTarget::Player` interaction with approach mode on the player.
/// * Sets `opcalled` to `true` on the player.
///
/// # Call Stack
///
/// **Called by:** `OpPlayer1::handle` through `OpPlayer4::handle`
/// **Calls:** `ActivePlayer::clear_pending_action`, `player.set_interaction`
fn handle(op: u8, pid: u16, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    if active.player.state.delayed {
        active.unset_map_flag();
        return Ok(());
    }

    if engine_mut().get_player(pid).is_none() {
        active.unset_map_flag();
        active.clear_pending_action()?;
        return Ok(());
    }

    if !active.player.build_area.players.contains(pid) {
        active.unset_map_flag();
        active.clear_pending_action()?;
        return Ok(());
    }

    let mode = match op {
        1 => ServerTriggerType::ApPlayer1,
        2 => ServerTriggerType::ApPlayer2,
        3 => ServerTriggerType::ApPlayer3,
        _ => ServerTriggerType::ApPlayer4,
    };

    let target = InteractionTarget::Player { pid };

    active.clear_pending_action()?;
    active.player.set_interaction(target, mode as u8, true);
    active.player.opcalled = true;

    Ok(())
}
