use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{action_target, in_build_area, spell_component_ok, zone_obj};
#[cfg(since_647)]
use crate::handlers::opobju;
#[cfg(before_647)]
use crate::watch::{Target, Using};
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::opobjt::OpObjT;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;
use rs_vm::trigger::ServerTriggerType;

impl ClientGameHandler for OpObjT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        #[cfg(since_647)]
        if self.obj2 != u16::MAX {
            return opobju::handle(
                self.x,
                self.z,
                self.obj,
                self.com as i32,
                self.slot,
                self.obj2,
                active,
            );
        }
        #[cfg(before_647)]
        active.watch_use(
            Using::Spell(self.com),
            Target::Obj {
                id: self.obj,
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
        if !spell_component_ok(active, spell_com, action_target::OBJ) {
            active.unset_map_flag();
            return Ok(());
        }

        if !in_build_area(active, self.x, self.z) {
            active.unset_map_flag();
            return Ok(());
        }

        let y = active.player.pathing.coord.y();
        let receiver = active.uid().username37();
        let Some(obj) = zone_obj(self.x, y, self.z, self.obj, receiver) else {
            active.unset_map_flag();
            return Ok(());
        };

        let target = InteractionTarget::Obj {
            coord: CoordGrid::new(self.x, y, self.z),
            id: self.obj,
            serial: obj.serial(),
        };

        active.clear_pending_action()?;
        active
            .player
            .set_interaction(target, ServerTriggerType::ApObjT as u8, true);
        active.player.interaction.target_subject_com = Some(spell_com);
        active.player.opcalled = true;

        Ok(())
    }
}
