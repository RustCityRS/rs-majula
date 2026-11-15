use crate::active_player::ActivePlayer;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::WorldlistFetch;
use rs_protocol::network::game::server::worldlist_fetch_reply::WorldlistFetchReply;
use rs_vm::ScriptError;

impl ClientGameHandler for WorldlistFetch {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        active.write(WorldlistFetchReply {
            world: active.world,
        });
        Ok(())
    }
}
