use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use crate::handlers::op_common::transmitted_inv;
use crate::handlers::op_common::{
    action_target, in_build_area, loc_target, spell_component_ok, zone_loc,
};
#[cfg(since_647)]
use crate::handlers::oplocu;
use crate::watch::{Target, Using};
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::oploct::OpLocT;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

impl ClientGameHandler for OpLocT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let com = self.com as ComId;
        #[cfg(since_647)]
        if transmitted_inv(active, com).is_some() {
            return oplocu::handle(self.x, self.z, self.loc, com, self.slot, self.obj, active);
        }
        active.watch_use(
            Using::Spell(com),
            Target::Loc {
                id: self.loc,
                x: self.x,
                z: self.z,
            },
        );
        if active.player.state.delayed {
            active.unset_map_flag();
            return Ok(());
        }

        if !spell_component_ok(active, com, action_target::LOC) {
            active.unset_map_flag();
            return Ok(());
        }

        if !in_build_area(active, self.x, self.z) {
            active.unset_map_flag();
            return Ok(());
        }

        let y = active.player.pathing.coord.y();
        let Some(loc) = zone_loc(self.x, y, self.z, self.loc) else {
            active.unset_map_flag();
            return Ok(());
        };
        if active.shown_loc(self.loc).is_none() {
            active.unset_map_flag();
            return Ok(());
        }

        let target = loc_target(CoordGrid::new(self.x, y, self.z), loc);

        active.clear_pending_action()?;
        active
            .player
            .set_interaction(target, ServerTriggerType::ApLocT as u8, true);
        active.player.interaction.target_subject_com = Some(com);
        active.player.opcalled = true;

        Ok(())
    }
}
