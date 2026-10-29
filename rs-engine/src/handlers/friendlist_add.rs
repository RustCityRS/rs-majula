use crate::active_player::ActivePlayer;
use crate::clients::client_ether::EtherOutbound;
use crate::counters::AutoBan;
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::friendlist_add::FriendListAdd;
use rs_util::base37::is_valid_userhash;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;

/// Handles the `FriendListAdd` client protocol message.
///
/// Forwards a friend list addition request to the ether (cross-world) service.
/// The ether service is responsible for persisting the friend relationship and
/// broadcasting online status updates.
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
/// * Sends an `EtherOutbound::FriendAdd` message containing the owner's and
///   friend's base-37 usernames to the ether service.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `EtherOutbound::FriendAdd` via channel send
impl ClientGameHandler for FriendListAdd {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let engine = engine();

        // The client's base37 encoding always yields a valid hash: a bad one
        // means a modified client.
        if !is_valid_userhash(self.user37 as u64) {
            active.auto_ban(AutoBan::PrivateName);
            return Ok(());
        }

        if active.ignores.contains(&self.user37) {
            active.auto_ban(AutoBan::PrivateName);
            return Ok(());
        }

        if active.friends.contains(&self.user37) {
            return Ok(());
        }

        active.friends.push(self.user37);

        let Some(tx) = &engine.ether_tx else {
            return Ok(());
        };

        let _ = tx.send(EtherOutbound::FriendAdd {
            owner37: active.uid().username37(),
            friend37: self.user37 as u64,
        });

        Ok(())
    }
}
