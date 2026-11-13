use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::core::engine;
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{action_target, in_build_area, spell_component_ok};
use crate::watch::{Target, Using};
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
use rs_protocol::network::game::client::apcoordt::ApCoordT;
use rs_vm::ScriptError;
use rs_vm::trigger::ServerTriggerType;

impl ClientGameHandler for ApCoordT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.watch_use(
            Using::Spell(self.com),
            Target::Coord {
                x: self.x,
                z: self.z,
            },
        );
        // A source holding an obj is an item, and no item is aimed at a tile:
        // the backpack can't target one.
        if active.player.state.delayed
            || self.obj != u16::MAX
            || !spell_component_ok(active, self.com, self.slot, action_target::COORD)
            || !in_build_area(active, self.x, self.z)
        {
            active.unset_map_flag();
            return Ok(());
        }

        let from = active.player.pathing.coord;
        let coord = CoordGrid::new(self.x, from.y(), self.z);
        if !rsmod::is_zone_allocated(coord.x(), coord.z(), coord.y())
            || !engine().region_allows(from, coord, None)
        {
            active.unset_map_flag();
            return Ok(());
        }

        active.clear_pending_action()?;
        active.player.set_interaction(
            InteractionTarget::Coord { coord },
            ServerTriggerType::ApCoordT as u8,
            true,
        );
        active.player.interaction.target_subject_com = Some(self.com);
        active.player.last_coord = Some(coord);
        active.player.opcalled = true;

        Ok(())
    }
}
