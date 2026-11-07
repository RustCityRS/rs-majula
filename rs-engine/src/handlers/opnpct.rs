use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::{action_target, npc_target_ok, spell_component_ok};
#[cfg(since_647)]
use crate::handlers::opnpcu;
#[cfg(before_647)]
use crate::watch::{Target, Using};
use rs_entity::InteractionTarget;
use rs_protocol::network::game::client::opnpct::OpNpcT;
use rs_vm::ScriptError;
use rs_vm::trigger::ServerTriggerType;

impl ClientGameHandler for OpNpcT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        #[cfg(since_647)]
        if self.obj != u16::MAX {
            return opnpcu::handle(self.nid, self.com as i32, self.slot, self.obj, active);
        }
        #[cfg(before_647)]
        active.watch_use(Using::Spell(self.com), Target::Npc(self.nid));
        if active.player.state.delayed {
            active.unset_map_flag();
            return Ok(());
        }

        #[cfg(before_647)]
        let spell_com = self.com;
        #[cfg(since_647)]
        let spell_com = self.com as i32;
        if !spell_component_ok(active, spell_com, action_target::NPC) {
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
        active.player.interaction.target_subject_com = Some(spell_com);
        active.player.opcalled = true;

        Ok(())
    }
}
