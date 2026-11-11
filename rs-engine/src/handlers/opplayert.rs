use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use crate::handlers::op_common::transmitted_inv;
use crate::handlers::op_common::{action_target, player_target_ok, spell_component_ok};
#[cfg(since_647)]
use crate::handlers::opplayeru;
use crate::watch::{Target, Using};
use rs_entity::InteractionTarget;
use rs_protocol::network::game::client::opplayert::OpPlayerT;
use rs_vm::ScriptError;
use rs_vm::trigger::ServerTriggerType;

impl ClientGameHandler for OpPlayerT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        #[cfg(before_647)]
        let com = self.com.into();
        #[cfg(since_647)]
        let com = self.com;
        #[cfg(since_647)]
        if transmitted_inv(active, com).is_some() {
            return opplayeru::handle(self.pid, com, self.slot, self.obj, active);
        }
        active.watch_use(Using::Spell(com), Target::Player(self.pid));
        if active.player.state.delayed {
            active.unset_map_flag();
            return Ok(());
        }

        #[cfg(before_647)]
        let slot = u16::MAX;
        #[cfg(since_647)]
        let slot = self.slot;
        #[cfg(since_647)]
        if self.obj != u16::MAX {
            active.unset_map_flag();
            return Ok(());
        }
        if !spell_component_ok(active, com, slot, action_target::PLAYER) {
            active.unset_map_flag();
            return Ok(());
        }

        if !player_target_ok(active, self.pid) {
            active.unset_map_flag();
            return Ok(());
        }

        active.clear_pending_action()?;
        active.player.set_interaction(
            InteractionTarget::Player { pid: self.pid },
            ServerTriggerType::ApPlayerT as u8,
            true,
        );
        active.player.interaction.target_subject_com = Some(com);
        active.player.opcalled = true;

        Ok(())
    }
}
