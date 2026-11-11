use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use crate::handlers::op_common::transmitted_inv;
use crate::handlers::op_common::{action_target, npc_target_ok, spell_component_ok};
#[cfg(since_647)]
use crate::handlers::opnpcu;
use crate::watch::{Target, Using};
use rs_entity::InteractionTarget;
use rs_protocol::network::game::client::opnpct::OpNpcT;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};

impl ClientGameHandler for OpNpcT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let com = self.com as ComId;
        #[cfg(since_647)]
        if transmitted_inv(active, com).is_some() {
            return opnpcu::handle(self.nid, com, self.slot, self.obj, active);
        }
        active.watch_use(Using::Spell(com), Target::Npc(self.nid));
        if active.player.state.delayed {
            active.unset_map_flag();
            return Ok(());
        }

        if !spell_component_ok(active, com, action_target::NPC) {
            active.unset_map_flag();
            return Ok(());
        }

        if !npc_target_ok(active, self.nid) {
            active.unset_map_flag();
            return Ok(());
        }

        active.clear_pending_action()?;
        active.player.set_interaction(
            InteractionTarget::Npc { nid: self.nid },
            ServerTriggerType::ApNpcT as u8,
            true,
        );
        active.player.interaction.target_subject_com = Some(com);
        active.player.opcalled = true;

        Ok(())
    }
}
