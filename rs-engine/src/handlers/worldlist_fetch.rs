use crate::active_player::ActivePlayer;
use crate::core::engine;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::WorldlistFetch;
use rs_vm::ScriptError;

impl ClientGameHandler for WorldlistFetch {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        for reply in engine().world_list.reply(self.checksum) {
            active.write(reply);
        }
        Ok(())
    }
}
