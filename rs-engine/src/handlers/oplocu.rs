use crate::active_player::{ActivePlayer, EnginePlayer};
#[cfg(before_647)]
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    HeldCheck, check_held, in_build_area, loc_target, members_blocked, use_component_ok, zone_loc,
};
use crate::watch::{Target, Using};
use rs_grid::CoordGrid;
#[cfg(before_647)]
use rs_protocol::network::game::client::oplocu::OpLocU;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

#[cfg(before_647)]
impl ClientGameHandler for OpLocU {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(
            self.x,
            self.z,
            self.loc,
            self.com.into(),
            self.slot,
            self.obj,
            active,
        )
    }
}

pub(super) fn handle(
    x: u16,
    z: u16,
    loc: u16,
    com: ComId,
    slot: u16,
    obj: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    active.watch_use(Using::Item(obj), Target::Loc { id: loc, x, z });
    if active.player.state.delayed {
        active.unset_map_flag();
        return Ok(());
    }

    if !in_build_area(active, x, z) {
        active.unset_map_flag();
        return Ok(());
    }

    let y = active.player.pathing.coord.y();
    let Some(loc) = zone_loc(x, y, z, loc) else {
        active.unset_map_flag();
        return Ok(());
    };
    if active.shown_loc(loc.id()).is_none() {
        active.unset_map_flag();
        return Ok(());
    }

    let target = loc_target(CoordGrid::new(x, y, z), loc);

    if !use_component_ok(active, com) {
        active.unset_map_flag();
        return Ok(());
    }

    if check_held(active, com, slot, obj) != HeldCheck::Ok {
        active.unset_map_flag();
        return Ok(());
    }

    active.clear_pending_action()?;

    if members_blocked(active, obj) {
        return Ok(());
    }

    active.player.last_use_item = Some(obj);
    active.player.last_use_slot = Some(slot);
    active.player.last_use_com = Some(com);

    active
        .player
        .set_interaction(target, ServerTriggerType::ApLocU as u8, true);
    active.player.opcalled = true;

    Ok(())
}
