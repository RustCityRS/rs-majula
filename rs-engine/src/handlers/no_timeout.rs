use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::no_timeout::NoTimeout;
use rs_vm::ScriptError;

impl ClientGameHandler for NoTimeout {
    fn handle(self, _active: &mut ActivePlayer) -> Result<(), ScriptError> {
        Ok(())
    }
}
