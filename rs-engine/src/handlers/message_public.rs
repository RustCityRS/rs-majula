use crate::active_player::ActivePlayer;
use crate::engine::engine;
use crate::handlers::ClientGameHandler;
use rs_protocol::network::game::client::message_public::MessagePublic;
use rs_protocol::network::game::info_prot::PlayerInfoProt;
use rs_util::wordpack::{pack, unpack};
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;

/// Handles the `MessagePublic` client protocol message.
///
/// Processes a public chat message spoken by the player in-game. Validates the
/// colour (0-11), effect (0-2), and byte length (max 100), then unpacks the
/// compressed text, filters it through the word encoder (censorship), repacks it,
/// and stores the result in the player's info block so it can be broadcast to
/// nearby players during the next player info update.
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` always (invalid messages are silently dropped).
///
/// # Side Effects
///
/// * Sets the player's `info.chat_bytes`, `info.chat_colour`, `info.chat_effects`,
///   and `info.chat_ignored` fields.
/// * Adds `PlayerInfoProt::Chat` to the player's info update mask.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `cache().wordenc.filter`, `pack`/`unpack`
impl ClientGameHandler for MessagePublic {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if self.colour > 11 || self.effect > 2 || self.bytes.len() > 100 {
            return Ok(());
        }

        let message = engine().wordenc().filter(&unpack(&self.bytes));
        active.player.info.chat_bytes = Some(pack(&message).into_boxed_slice());
        active.player.info.chat_colour = Some(self.colour);
        active.player.info.chat_effects = Some(self.effect);
        active.player.info.chat_ignored = Some(active.player.staff_mod_level as u8);
        active.player.info.masks |= PlayerInfoProt::Chat as u16;

        Ok(())
    }
}
