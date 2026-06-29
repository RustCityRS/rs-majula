#[cfg(before_274)]
use crate::active_player::ActivePlayer;
#[cfg(before_274)]
use crate::handlers::ClientGameHandler;
#[cfg(before_274)]
use rs_protocol::network::game::client::event_tracking::EventTracking;
#[cfg(before_274)]
use rs_vm::ScriptError;

/// Handles the `EventTracking` client protocol message.
///
/// No-op. The server accepts but ignores client input-tracking telemetry.
/// This packet was removed from the protocol in revision 274.
///
/// # Arguments
///
/// * `_` - The active player (unused).
///
/// # Returns
///
/// * `Ok(())` always.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
#[cfg(before_274)]
impl ClientGameHandler for EventTracking {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        Ok(())
    }
}
