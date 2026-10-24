#[cfg(any(rev = "225", since_254))]
use crate::active_player::ActivePlayer;
#[cfg(since_254)]
use crate::engine::engine;
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
/// A click the client acted on. The anti-cheat checks that the action it
/// starts arrives on the same tick, and judges the click's spot and rhythm.
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
#[cfg(since_254)]
impl ClientGameHandler for EventMouseClick {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let ms = active.watch.packet_ms();
        active
            .watch
            .anticheat
            .mouse_click(ms, engine().clock, &self);
        Ok(())
    }
}

/// Handles the `EventMouseMove` client protocol message.
///
/// The cursor's path since the last report, for the anti-cheat to compare
/// clicks against.
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
#[cfg(since_254)]
impl ClientGameHandler for EventMouseMove {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let ms = active.watch.packet_ms();
        active.watch.anticheat.mouse_move(ms, &self);
        Ok(())
    }
}

/// Handles the `EventAppletFocus` client protocol message.
///
/// The client's window gained or lost focus; the anti-cheat counts the
/// losses.
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
#[cfg(since_254)]
impl ClientGameHandler for EventAppletFocus {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let ms = active.watch.packet_ms();
        active.watch.anticheat.focus(ms, engine().clock, &self);
        Ok(())
    }
}

/// Handles the `EventCameraPosition` client protocol message.
///
/// The client sends where its camera points while it turns; the anti-cheat
/// counts the reports.
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
#[cfg(any(rev = "225", since_254))]
impl ClientGameHandler for EventCameraPosition {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let ms = active.watch.packet_ms();
        active.watch.anticheat.camera(ms, &self);
        Ok(())
    }
}
