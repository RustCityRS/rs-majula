use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::core::engine_mut;
#[cfg(before_647)]
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{
    action_target, held_component_ok, require_held, spell_component_ok,
};
use crate::watch::{Target, Using};
#[cfg(before_647)]
use rs_protocol::network::game::client::opheldt::OpHeldT;
use rs_vm::engine::ScriptPlayer;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

#[cfg(before_647)]
impl ClientGameHandler for OpHeldT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        handle(
            self.com.into(),
            self.slot,
            self.obj,
            self.com2.into(),
            u16::MAX,
            active,
        )
    }
}

pub(super) fn handle(
    com: ComId,
    slot: u16,
    obj: u16,
    spell: ComId,
    spell_slot: u16,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    active.watch_use(Using::Spell(spell), Target::Item(obj));
    if active.player.state.delayed {
        return Ok(());
    }
    if !spell_component_ok(active, spell, spell_slot, action_target::HELD)
        || !held_component_ok(active, com, slot)
    {
        return Err(ScriptError::Client(format!(
            "Component is not acceptable for opheldt: {spell} on {com}"
        )));
    }
    if !require_held(active, com, slot, obj)? {
        return Ok(());
    }
    active.player.last_item = Some(obj);
    active.player.last_slot = Some(slot);
    active.clear_pending_action()?;
    active.player.info.clear_face_entity_player();

    match engine_mut().run_script_by_trigger(
        (ServerTriggerType::OpHeldT, Some(spell), None),
        Some(ScriptSubject::Player(active.player.uid)),
        None,
        Some(true),
        None,
        None,
    ) {
        Err(ScriptError::TriggerNotFound(_)) => {
            #[cfg(debug_assertions)]
            active.message_game(&format!("No trigger for [opheldt,{spell}]"));
            active.nothing_interesting();
        }
        other => other?,
    }
    Ok(())
}
