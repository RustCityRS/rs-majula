use crate::active_player::ActivePlayer;
use crate::clients::client_ether::EtherOutbound;
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::message_private::MessagePrivate;
use rs_util::base37::is_valid_userhash;
use rs_util::wordpack::{pack, unpack};
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};

/// Handles the `MessagePrivate` client protocol message.
///
/// Processes a private (whisper) chat message from the player to a specific target.
/// The message bytes are validated for length (max 100 bytes), unpacked from the
/// game's compressed text format, filtered through the word encoder (censorship),
/// repacked, and forwarded to the ether (cross-world) service for delivery.
/// One message per tick (shared with public chat); a muted player's message
/// is dropped. A target that isn't a valid name only comes from a modified
/// client, so the sender is banned and logged out (`ActivePlayer::auto_ban`).
///
/// If no ether connection is available, the message is silently dropped.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` always (oversized messages are silently dropped).
///
/// # Side Effects
///
/// * Sends an `EtherOutbound::PrivateMessage` containing the sender's username,
///   target username, staff level, and filtered message bytes to the ether service.
/// * Logs the unfiltered message to the database (`ActivePlayer::log_chat`).
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `cache().wordenc.filter`, `pack`/`unpack`, `EtherOutbound::PrivateMessage`
impl ClientGameHandler for MessagePrivate {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let engine = engine();
        if active.last_social == Some(engine.clock) || self.bytes.len() > 100 {
            return Ok(());
        }

        if active.is_muted() {
            return Ok(());
        }

        // The client only sends names from the friend list: a bad one means a
        // modified client.
        if !is_valid_userhash(self.user37 as u64) {
            active.auto_ban();
            return Ok(());
        }

        let Some(tx) = &engine.ether_tx else {
            return Ok(());
        };

        let text = unpack(&self.bytes);
        let message = engine.wordenc().filter(&text);
        let filtered_bytes = pack(&message);

        let _ = tx.send(EtherOutbound::PrivateMessage {
            sender37: active.uid().username37(),
            target37: self.user37 as u64,
            level: active.player.staff_mod_level as u8,
            bytes: filtered_bytes,
        });

        active.last_social = Some(engine.clock);
        active.log_chat(Some(self.user37 as u64), text);
        Ok(())
    }
}
