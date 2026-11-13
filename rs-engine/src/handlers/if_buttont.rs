use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use crate::handlers::op_common::transmitted_inv;
use crate::handlers::{opheldt, opheldu};
use rs_protocol::network::game::client::if_buttont::IfButtonT;
use rs_vm::ScriptError;

impl ClientGameHandler for IfButtonT {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if transmitted_inv(active, self.com).is_some() {
            opheldu::handle(
                self.com2, self.slot2, self.obj2, self.com, self.slot, self.obj, active,
            )
        } else if self.obj == u16::MAX {
            opheldt::handle(
                self.com2, self.slot2, self.obj2, self.com, self.slot, active,
            )
        } else {
            Err(ScriptError::Client(format!(
                "No inv transmit for interface with id: {}",
                self.com
            )))
        }
    }
}
