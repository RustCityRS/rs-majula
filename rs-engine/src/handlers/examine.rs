//! Rev-647 examine: the sixth op on an npc, loc or ground obj.
//!
//! The client names these OPNPC6, OPLOC6 and OPOBJ6 and sends only the type
//! id -- for an npc, the type after multi-npc resolution
//! (`Statics1.do_action`). 225/289 examined on the client, from the type's
//! `desc` in the cache; 647's cache carries no examine text, so the server
//! answers with the type's `desc` as a game message, as `void-634` does
//! (`Examines.kt`). A type with no `desc` gets no answer, as in void.

use crate::active_player::ActivePlayer;
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::{ExamineLoc, ExamineNpc, ExamineObj};
use rs_vm::ScriptError;

fn answer(active: &mut ActivePlayer, desc: Option<&str>) -> Result<(), ScriptError> {
    if let Some(desc) = desc {
        active.message_game(desc);
    }
    Ok(())
}

impl ClientGameHandler for ExamineNpc {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let desc = engine()
            .cache
            .npcs
            .get_by_id(self.id)
            .and_then(|t| t.desc.as_deref());
        answer(active, desc)
    }
}

impl ClientGameHandler for ExamineLoc {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let desc = engine()
            .cache
            .locs
            .get_by_id(self.id)
            .and_then(|t| t.desc.as_deref());
        answer(active, desc)
    }
}

impl ClientGameHandler for ExamineObj {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let desc = engine()
            .cache
            .objs
            .get_by_id(self.id)
            .and_then(|t| t.desc.as_deref());
        answer(active, desc)
    }
}
