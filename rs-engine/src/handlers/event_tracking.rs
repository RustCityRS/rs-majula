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
/// The client's input (mouse, keys, focus) since its last report, while the
/// server has tracking on. Before 254 it's the only input report; after,
/// only its keys are new. This packet was removed from the protocol in
/// revision 274.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
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
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let ms = active.watch.ms();
        active.watch.anticheat.tracking(ms, &self);
        Ok(())
    }
}
