use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    action_target, in_build_area, loc_target, spell_component_ok, zone_loc,
};
#[cfg(since_647)]
use crate::handlers::oplocu;
#[cfg(before_647)]
use crate::watch::{Target, Using};
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::oploct::OpLocT;
use rs_vm::ScriptError;
use rs_vm::trigger::ServerTriggerType;

impl ClientGameHandler for OpLocT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        #[cfg(since_647)]
        if self.obj != u16::MAX {
            return oplocu::handle(
                self.x,
                self.z,
                self.loc,
                self.com as i32,
                self.slot,
                self.obj,
                active,
            );
        }
        #[cfg(before_647)]
        active.watch_use(
            Using::Spell(self.com),
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

        #[cfg(before_647)]
        let spell_com = self.com;
        #[cfg(since_647)]
        let spell_com = self.com as i32;
        if !spell_component_ok(active, spell_com, action_target::LOC) {
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
        #[cfg(before_647)]
        if active.shown_loc(self.loc).is_none() {
            active.unset_map_flag();
            return Ok(());
        }

        let target = loc_target(CoordGrid::new(self.x, y, self.z), loc);

        active.clear_pending_action()?;
        active
            .player
            .set_interaction(target, ServerTriggerType::ApLocT as u8, true);
        active.player.interaction.target_subject_com = Some(spell_com);
        active.player.opcalled = true;

        Ok(())
    }
}
