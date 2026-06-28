#[cfg(any(rev = "225", since_254))]
use crate::active_player::ActivePlayer;
#[cfg(any(rev = "225", since_254))]
use crate::handlers::ClientGameHandler;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_applet_focus::EventAppletFocus;
#[cfg(any(rev = "225", since_254))]
use rs_protocol::network::game::client::event_camera_position::EventCameraPosition;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_click::EventMouseClick;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_move::EventMouseMove;
#[cfg(any(rev = "225", since_254))]
use rs_vm::ScriptError;

/// Handles the `EventMouseClick` client protocol message.
///
/// No-op. The server accepts but ignores client mouse-click telemetry.
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
#[cfg(since_254)]
impl ClientGameHandler for EventMouseClick {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `EventMouseMove` client protocol message.
///
/// No-op. The server accepts but ignores client mouse-move telemetry.
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
#[cfg(since_254)]
impl ClientGameHandler for EventMouseMove {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `EventAppletFocus` client protocol message.
///
/// No-op. The server accepts but ignores client applet focus/blur telemetry.
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
#[cfg(since_254)]
impl ClientGameHandler for EventAppletFocus {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `EventCameraPosition` client protocol message.
///
/// No-op. The client periodically sends camera position updates, but the server
/// does not currently use this information. The packet is accepted and discarded.
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
#[cfg(any(rev = "225", since_254))]
impl ClientGameHandler for EventCameraPosition {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        Ok(())
    }
}

/// Shared no-op handler for ignored client telemetry events.
#[cfg(since_254)]
fn handle() -> Result<(), ScriptError> {
    Ok(())
}
