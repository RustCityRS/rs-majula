#[cfg(since_254)]
use crate::active_player::ActivePlayer;
#[cfg(since_254)]
use crate::handlers::ClientGameHandler;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_applet_focus::EventAppletFocus;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_click::EventMouseClick;
#[cfg(since_254)]
use rs_protocol::network::game::client::event_mouse_move::EventMouseMove;
#[cfg(since_254)]
use rs_protocol::network::game::client::map_build_complete::MapBuildComplete;
#[cfg(since_254)]
use rs_vm::ScriptError;

/// Handles the `EventMouseClick` client protocol message.
///
/// No-op. The server accepts but ignores client mouse-click telemetry.
#[cfg(since_254)]
impl ClientGameHandler for EventMouseClick {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `EventMouseMove` client protocol message.
///
/// No-op. The server accepts but ignores client mouse-move telemetry.
#[cfg(since_254)]
impl ClientGameHandler for EventMouseMove {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `EventAppletFocus` client protocol message.
///
/// No-op. The server accepts but ignores client applet focus/blur telemetry.
#[cfg(since_254)]
impl ClientGameHandler for EventAppletFocus {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Handles the `MapBuildComplete` client protocol message.
///
/// No-op. The server accepts but ignores the client's map-build-complete notice.
#[cfg(since_254)]
impl ClientGameHandler for MapBuildComplete {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle()
    }
}

/// Shared no-op handler for ignored client telemetry events.
#[cfg(since_254)]
fn handle() -> Result<(), ScriptError> {
    Ok(())
}
