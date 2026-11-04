use crate::active_player::{ActivePlayer, EnginePlayer};
#[cfg(before_647)]
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    HeldCheck, check_held, members_blocked, player_target_ok, use_component_ok,
};
#[cfg(before_647)]
use crate::watch::{Target, Using};
use rs_entity::InteractionTarget;
#[cfg(before_647)]
use rs_protocol::network::game::client::opplayeru::OpPlayerU;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

#[cfg(before_647)]
impl ClientGameHandler for OpPlayerU {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(self.pid, self.com, self.slot, self.obj, active)
    }
}

pub(super) fn handle(
    pid: u16,
    com: ComId,
    slot: u16,
    obj: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    #[cfg(before_647)]
    active.watch_use(Using::Item(obj), Target::Player(pid));
    if active.player.state.delayed {
        active.unset_map_flag();
        return Ok(());
    }

    if !use_component_ok(active, com) {
        active.unset_map_flag();
        return Ok(());
    }

    if check_held(active, com, slot, obj) != HeldCheck::Ok {
        active.unset_map_flag();
        return Ok(());
    }

    if !player_target_ok(active, pid) {
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

    active.player.set_interaction(
        InteractionTarget::Player { pid },
        ServerTriggerType::ApPlayerU as u8,
        true,
    );
    #[cfg(since_647)]
    let obj = i32::from(obj);
    active.player.interaction.target_subject_com = Some(obj);
    active.player.opcalled = true;

    Ok(())
}
