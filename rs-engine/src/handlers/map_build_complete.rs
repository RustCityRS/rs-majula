#[cfg(since_254)]
use crate::active_player::ActivePlayer;
#[cfg(since_254)]
use crate::handlers::ClientGameHandler;
#[cfg(since_254)]
use rs_protocol::network::game::client::map_build_complete::MapBuildComplete;
#[cfg(since_254)]
use rs_vm::ScriptError;

/// Records that the client has finished building its scene.
#[cfg(since_254)]
impl ClientGameHandler for MapBuildComplete {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.player.build_area.loading = false;
        Ok(())
    }
}
