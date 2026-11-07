use crate::active_player::ActivePlayer;
use crate::clients::client_ether::{EtherOutbound, MAX_IGNORES};
#[cfg(before_647)]
use crate::counters::AutoBan;
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client_prot::IgnoreListAdd;
#[cfg(before_647)]
use rs_util::base37::is_valid_userhash;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;

/// Handles the `IgnoreListAdd` client protocol message.
///
/// Forwards an ignore list addition request to the ether (cross-world) service.
/// The ether service is responsible for persisting the ignore relationship and
/// filtering messages from the ignored player.
///
/// If no ether connection is available, the request is silently dropped.
///
/// Before rev 647 the client sends the name as its base-37 hash; from 647 it
/// sends the name as typed, which is hashed here the way the engine hashes a
/// display name (`to_userhash`), so it matches the key ether stores the
/// account under. An empty name hashes to nothing and is dropped.
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
/// * Sends an `EtherOutbound::IgnoreAdd` message containing the owner's and
///   ignored player's base-37 usernames to the ether service.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `EtherOutbound::IgnoreAdd` via channel send
impl ClientGameHandler for IgnoreListAdd {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let engine = engine();

        #[cfg(before_647)]
        let user37 = {
            // The client's base37 encoding always yields a valid hash: a bad one
            // means a modified client.
            if !is_valid_userhash(self.user37 as u64) {
                active.auto_ban(AutoBan::PrivateName);
                return Ok(());
            }
            self.user37
        };
        #[cfg(since_647)]
        let user37 = match rs_util::base37::to_userhash(&self.name) {
            0 => return Ok(()),
            hash => hash as i64,
        };

        if active.friends.contains(&user37) {
            #[cfg(before_647)]
            active.auto_ban(AutoBan::PrivateName);
            return Ok(());
        }

        if active.ignores.contains(&user37) {
            return Ok(());
        }

        if active.ignores.len() >= MAX_IGNORES {
            return Ok(());
        }

        // ignores doesnt get a self check.

        active.ignores.push(user37);

        let Some(tx) = &engine.ether_tx else {
            return Ok(());
        };

        let _ = tx.send(EtherOutbound::IgnoreAdd {
            owner37: active.uid().username37(),
            ignore37: user37 as u64,
        });

        Ok(())
    }
}
