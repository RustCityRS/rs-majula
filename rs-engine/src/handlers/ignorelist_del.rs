use crate::active_player::ActivePlayer;
use crate::clients::client_ether::EtherOutbound;
use crate::counters::AutoBan;
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::ignorelist_del::IgnoreListDel;
use rs_util::base37::is_valid_userhash;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;

/// Handles the `IgnoreListDel` client protocol message.
///
/// Forwards an ignore list removal request to the ether (cross-world) service.
/// The ether service is responsible for persisting the removal.
///
/// If no ether connection is available, the request is silently dropped.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` always.
///
/// # Side Effects
///
/// * Sends an `EtherOutbound::IgnoreDel` message containing the owner's and
///   ignored player's base-37 usernames to the ether service.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `EtherOutbound::IgnoreDel` via channel send
impl ClientGameHandler for IgnoreListDel {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let engine = engine();

        // The client only sends names from the ignore list: a bad one means a
        // modified client.
        if !is_valid_userhash(self.user37 as u64) {
            active.auto_ban(AutoBan::PrivateName);
            return Ok(());
        }

        if !active.ignores.contains(&self.user37) {
            return Ok(());
        }

        // ignores doesnt get a self check.

        active.ignores.retain(|&i| i != self.user37);

        let Some(tx) = &engine.ether_tx else {
            return Ok(());
        };

        let _ = tx.send(EtherOutbound::IgnoreDel {
            owner37: active.uid().username37(),
            ignore37: self.user37 as u64,
        });

        Ok(())
    }
}
