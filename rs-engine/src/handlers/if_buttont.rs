use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::action_target;
#[cfg(all(since_530, before_647))]
use crate::handlers::op_common::obj_at;
use crate::handlers::opheldt;
#[cfg(since_647)]
use crate::handlers::{op_common::transmitted_inv, opheldu};
use rs_protocol::network::game::client::if_buttont::IfButtonT;
use rs_vm::ScriptError;

/// The 530 client sends IF_BUTTONT only for a spell cast on an if3
/// component, and leaves the obj in the target's slot out.
#[cfg(all(since_530, before_647))]
impl ClientGameHandler for IfButtonT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let obj = obj_at(active, self.com2, self.slot2);
        opheldt::handle(
            self.com2,
            self.slot2,
            obj,
            self.com,
            self.slot,
            action_target::COMPONENT,
            active,
        )
    }
}

#[cfg(since_647)]
impl ClientGameHandler for IfButtonT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if transmitted_inv(active, self.com).is_some() {
            opheldu::handle(
                self.com2, self.slot2, self.obj2, self.com, self.slot, self.obj, active,
            )
        } else if self.obj.is_null() {
            opheldt::handle(
                self.com2,
                self.slot2,
                self.obj2,
                self.com,
                self.slot,
                action_target::HELD,
                active,
            )
        } else {
            Err(ScriptError::Client(format!(
                "No inv transmit for interface with id: {}",
                self.com
            )))
        }
    }
}
