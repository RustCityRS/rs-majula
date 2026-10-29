use crate::active_player::ActivePlayer;
use crate::clients::client_ether::EtherOutbound;
use crate::counters::AutoBan;
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::friendlist_del::FriendListDel;
use rs_util::base37::is_valid_userhash;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;

/// Handles the `FriendListDel` client protocol message.
///
/// Forwards a friend list removal request to the ether (cross-world) service.
/// The ether service is responsible for persisting the removal and updating
/// online status broadcasts.
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
/// * Sends an `EtherOutbound::FriendDel` message containing the owner's and
///   friend's base-37 usernames to the ether service.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `EtherOutbound::FriendDel` via channel send
impl ClientGameHandler for FriendListDel {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let engine = engine();

        // The client only sends names from the friend list: a bad one means a
        // modified client.
        if !is_valid_userhash(self.user37 as u64) {
            active.auto_ban(AutoBan::PrivateName);
            return Ok(());
        }

        if !active.friends.contains(&self.user37) {
            return Ok(());
        }

        // The client drops the name from its own list as it sends this.
        active.friends.retain(|&f| f != self.user37);
        active.watch.forget_friend(self.user37 as u64);

        let Some(tx) = &engine.ether_tx else {
            return Ok(());
        };

        let _ = tx.send(EtherOutbound::FriendDel {
            owner37: active.uid().username37(),
            friend37: self.user37 as u64,
        });

        Ok(())
    }
}
