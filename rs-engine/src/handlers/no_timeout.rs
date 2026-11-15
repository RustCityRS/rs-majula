use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::no_timeout::NoTimeout;
#[cfg(since_647)]
use rs_protocol::network::game::server::cam_smoothreset::CamSmoothreset;
use rs_vm::ScriptError;

/// Handles the `NoTimeout` keepalive. In the game the client's other traffic
/// keeps it connected and nothing is sent back; a lobby client, sent nothing
/// else, is answered.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::read` (via `ClientGameHandler` dispatch)
impl ClientGameHandler for NoTimeout {
    #[cfg_attr(before_647, allow(unused_variables))]
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        #[cfg(since_647)]
        if active.lobby {
            active.write(CamSmoothreset);
        }
        Ok(())
    }
}
