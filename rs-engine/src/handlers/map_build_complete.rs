#[cfg(since_254)]
use crate::active_player::ActivePlayer;
#[cfg(since_254)]
use crate::handlers::ClientGameHandler;
#[cfg(since_254)]
use rs_protocol::network::game::client::map_build_complete::MapBuildComplete;
#[cfg(since_254)]
use rs_vm::ScriptError;

/// Handles the `MapBuildComplete` client protocol message.
///
/// No-op. The server accepts but ignores the client's map-build-complete notice.
#[cfg(since_254)]
impl ClientGameHandler for MapBuildComplete {
    fn handle(self, _: &mut ActivePlayer) -> Result<(), ScriptError> {
        Ok(())
    }
}
