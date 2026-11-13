use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::in_build_area;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client_prot::FaceSquare;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;

impl ClientGameHandler for FaceSquare {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.state.delayed || !in_build_area(active, self.x, self.z) {
            return Ok(());
        }
        active.clear_pending_action()?;
        active.facesquare(CoordGrid::new(
            self.x,
            active.player.pathing.coord.y(),
            self.z,
        ));
        Ok(())
    }
}
