use crate::active_player::{ActivePlayer, EnginePlayer};
#[cfg(before_647)]
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    HeldCheck, action_target, check_held, members_blocked, npc_target_ok, use_component_ok,
};
use crate::watch::{Target, Using};
use rs_entity::InteractionTarget;
#[cfg(before_647)]
use rs_protocol::network::game::client::opnpcu::OpNpcU;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

#[cfg(before_647)]
impl ClientGameHandler for OpNpcU {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(self.nid, self.com.into(), self.slot, self.obj, active)
    }
}

pub(super) fn handle(
    nid: u16,
    com: ComId,
    slot: u16,
    obj: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    active.watch_use(Using::Item(obj), Target::Npc(nid));
    if active.player.state.delayed {
        active.unset_map_flag();
        return Ok(());
    }

    if !use_component_ok(active, com, slot, action_target::NPC) {
        active.unset_map_flag();
        return Ok(());
    }

    if check_held(active, com, slot, obj) != HeldCheck::Ok {
        active.unset_map_flag();
        return Ok(());
    }

    if !npc_target_ok(active, nid) {
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
        InteractionTarget::Npc { nid },
        ServerTriggerType::ApNpcU as u8,
        true,
    );
    active.player.opcalled = true;

    Ok(())
}
