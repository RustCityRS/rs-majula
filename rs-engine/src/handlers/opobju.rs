use crate::active_player::{ActivePlayer, EnginePlayer};
#[cfg(before_647)]
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    HeldCheck, action_target, check_held, in_build_area, members_blocked, use_component_ok,
    zone_obj,
};
use crate::watch::{Target, Using};
use rs_entity::InteractionTarget;
use rs_grid::CoordGrid;
#[cfg(before_647)]
use rs_protocol::network::game::client::opobju::OpObjU;
use rs_vm::engine::ScriptPlayer;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

#[cfg(before_647)]
impl ClientGameHandler for OpObjU {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(
            self.x,
            self.z,
            self.obj,
            self.com.into(),
            self.slot,
            self.use_obj,
            active,
        )
    }
}

pub(super) fn handle(
    x: u16,
    z: u16,
    obj: u16,
    com: ComId,
    slot: u16,
    use_obj: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    active.watch_use(Using::Item(use_obj), Target::Obj { id: obj, x, z });
    if active.player.state.delayed {
        active.unset_map_flag();
        return Ok(());
    }

    if !in_build_area(active, x, z) {
        active.unset_map_flag();
        return Ok(());
    }

    let y = active.player.pathing.coord.y();
    let receiver = active.uid().username37();
    let Some(obj) = zone_obj(x, y, z, obj, receiver) else {
        active.unset_map_flag();
        return Ok(());
    };

    let target = InteractionTarget::Obj {
        coord: CoordGrid::new(x, y, z),
        id: obj.id(),
        serial: obj.serial(),
    };

    if !use_component_ok(active, com, slot, action_target::OBJ) {
        active.unset_map_flag();
        return Ok(());
    }

    if check_held(active, com, slot, use_obj) != HeldCheck::Ok {
        active.unset_map_flag();
        return Ok(());
    }

    active.clear_pending_action()?;

    if members_blocked(active, use_obj) {
        return Ok(());
    }

    active.player.last_use_item = Some(use_obj);
    active.player.last_use_slot = Some(slot);
    active.player.last_use_com = Some(com);

    active
        .player
        .set_interaction(target, ServerTriggerType::ApObjU as u8, true);
    active.player.opcalled = true;

    Ok(())
}
