use crate::active_player::ActivePlayer;
#[cfg(before_647)]
use crate::clients::client_ether::EtherOutbound;
use crate::core::engine;
use crate::counters::{self, CHAT_PRIVATE, COUNTERS};
use crate::handlers::ClientGameHandler;
#[cfg(all(since_530, before_647))]
use crate::handlers::message_public::MAX_CHAT_CHARS;
#[cfg(since_530)]
use crate::handlers::message_public::{decode_chat, filtered_chat};
#[cfg(before_647)]
use crate::handlers::social::forged_name;
#[cfg(since_647)]
use crate::social::{self, CHAT_TEXT, ChatTarget, MAX_PRIVATE_CHARS, name_key};
#[cfg(before_647)]
use rs_db::chat_channel;
use rs_protocol::network::game::client::message_private::MessagePrivate;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::server::message_private_echo::MessagePrivateEcho;
#[cfg(before_530)]
use rs_util::wordpack::{pack, unpack};
use rs_vm::ScriptError;
#[cfg(before_530)]
use rs_vm::engine::ScriptEngine;
#[cfg(before_647)]
use rs_vm::engine::ScriptPlayer;

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
#[cfg(before_530)]
impl ClientGameHandler for MessagePrivate {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let engine = engine();
        if active.last_social == Some(engine.clock) || self.bytes.len() > 100 {
            return Ok(());
        }

        if active.is_muted() {
            counters::add(&COUNTERS.chat_muted[CHAT_PRIVATE], 1);
            return Ok(());
        }

        if forged_name(active, self.user37) {
            return Ok(());
        }

        if !active.friends.contains(&self.user37) {
            return Ok(());
        }

        let Some(tx) = &engine.ether_tx else {
            return Ok(());
        };

        let text = unpack(&self.bytes);
        let message = engine.wordenc().filter(&text);

        let _ = tx.send(EtherOutbound::PrivateMessage {
            sender37: active.uid().username37(),
            target37: self.user37 as u64,
            level: active.staffmodlevel(),
            bytes: pack(&message),
        });

        active.last_social = Some(engine.clock);
        counters::add(&COUNTERS.chat[CHAT_PRIVATE], 1);
        active.log_chat(chat_channel::PRIVATE, Some(self.user37 as u64), text);
        Ok(())
    }
}

/// Handles the `MessagePrivate` client protocol message.
///
/// Rev 530 sends the text huffman-compressed, at most 80 characters, to a
/// friend's name hash, one message per tick shared with public chat. A muted
/// player's message is dropped, and one for a name not on the friends list;
/// a name that is no name comes only from a modified client, which is banned
/// (`ActivePlayer::auto_ban`). The sender gets the filtered text back as
/// `MESSAGE_PRIVATE_ECHO`, and the ether relays it to the friend's world.
#[cfg(all(since_530, before_647))]
impl ClientGameHandler for MessagePrivate {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let engine = engine();
        let chars = self.chars as usize;
        if active.last_social == Some(engine.clock) || !(1..=MAX_CHAT_CHARS).contains(&chars) {
            return Ok(());
        }
        if active.is_muted() {
            counters::add(&COUNTERS.chat_muted[CHAT_PRIVATE], 1);
            return Ok(());
        }
        if forged_name(active, self.user37) {
            return Ok(());
        }
        if !active.friends.contains(&self.user37) {
            return Ok(());
        }

        let text = decode_chat(self.bytes, chars)?;
        let bytes = filtered_chat(&text);
        active.write(MessagePrivateEcho {
            user37: self.user37,
            bytes: &bytes,
        });
        engine.send_ether(EtherOutbound::PrivateMessage {
            sender37: active.uid().username37(),
            target37: self.user37 as u64,
            level: active.staffmodlevel(),
            bytes,
        });
        active.last_social = Some(engine.clock);
        counters::add(&COUNTERS.chat[CHAT_PRIVATE], 1);
        active.log_chat(chat_channel::PRIVATE, Some(self.user37 as u64), text);
        Ok(())
    }
}

/// Handles the `MessagePrivate` client protocol message.
///
/// Rev 647 names the friend and sends the text huffman-compressed, at most 80
/// characters (255 from 742, which takes private messages in the chat box),
/// one message per tick shared with public chat. A muted player's message is
/// dropped. Ether delivers it to the friend; the sender gets the filtered text
/// back as `MESSAGE_PRIVATE_ECHO` once it is delivered, else why it wasn't.
#[cfg(since_647)]
impl ClientGameHandler for MessagePrivate {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let clock = engine().clock;
        let chars = self.chars as usize;
        if active.last_social == Some(clock) || !(1..=MAX_PRIVATE_CHARS).contains(&chars) {
            return Ok(());
        }
        if active.is_muted() {
            counters::add(&COUNTERS.chat_muted[CHAT_PRIVATE], 1);
            return Ok(());
        }
        let Some(name) = name_key(&self.name) else {
            return Ok(());
        };

        let text = decode_chat(self.bytes, chars)?;
        let payload = filtered_chat(&text);
        active.last_social = Some(clock);
        counters::add(&COUNTERS.chat[CHAT_PRIVATE], 1);
        let message = social::say(active, ChatTarget::Friend(name), CHAT_TEXT, payload, text);
        engine().send_ether(message);
        Ok(())
    }
}
