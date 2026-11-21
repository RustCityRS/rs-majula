use crate::active_player::ActivePlayer;
use crate::core::engine;
use crate::counters::{self, CHAT_PUBLIC, COUNTERS};
#[cfg(since_647)]
use crate::counters::{CHAT_CLAN, CHAT_FRIENDS_CHAT, CHAT_PRIVATE};
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use crate::social::{self, CHAT_TEXT, ChatTarget};
#[cfg(since_530)]
use rs_crypto::huffman::Huffman;
use rs_db::chat_channel;
#[cfg(since_647)]
use rs_entity::player::CHAT_MODE_PUBLIC;
#[cfg(since_530)]
use rs_io::Packet;
#[cfg(since_530)]
use rs_io::stringtools::{cp1252_to_utf8, utf8_to_cp1252};
#[cfg(since_530)]
use rs_pack::cache::wordenc::WordEncProvider;
use rs_protocol::network::game::client::message_public::MessagePublic;
use rs_protocol::network::game::info_prot::PlayerInfoProt;
#[cfg(before_530)]
use rs_util::wordpack::{pack, unpack};
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};

/// Highest chat effect this revision's client can render.
///
/// `0` = none, `1` = wave, `2` = wave2. From 289 the client also renders
/// `3` = shake, `4` = scroll and `5` = slide.
#[cfg(before_289)]
const MAX_CHAT_EFFECT: u8 = 2;
#[cfg(all(since_289, before_530))]
const MAX_CHAT_EFFECT: u8 = 5;

#[cfg(before_530)]
const MAX_CHAT_COLOUR: u8 = 11;

#[cfg(before_530)]
const MAX_CHAT_LEN: usize = 100;

/// Handles the `MessagePublic` client protocol message.
///
/// Processes a public chat message spoken by the player in-game, one per tick
/// (shared with private chat). Validates the colour (0-11), effect (0-[`MAX_CHAT_EFFECT`]), and byte length (max 100),
/// and drops it if the player is muted. Then unpacks the
/// compressed text, filters it through the word encoder (censorship), repacks
/// it, and stores the result in the player's info block so it can be broadcast
/// to nearby players during the next player info update.
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
///   and `info.chat_crown` fields.
/// * Adds `PlayerInfoProt::Chat` to the player's info update mask.
/// * Logs the unfiltered message to the database (`ActivePlayer::log_chat`).
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `cache().wordenc.filter`, `pack`/`unpack`
#[cfg(before_530)]
impl ClientGameHandler for MessagePublic {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let clock = engine().clock;
        if active.last_social == Some(clock)
            || self.colour > MAX_CHAT_COLOUR
            || self.effect > MAX_CHAT_EFFECT
            || self.bytes.len() > MAX_CHAT_LEN
        {
            return Ok(());
        }

        if active.is_muted() {
            counters::add(&COUNTERS.chat_muted[CHAT_PUBLIC], 1);
            return Ok(());
        }

        let ms = active.watch.packet_ms();
        active.watch.anticheat.chatted(ms);
        let text = unpack(&self.bytes);
        let message = engine().wordenc().filter(&text);
        active.player.info.chat_bytes = Some(pack(&message).into_boxed_slice());
        active.player.info.chat_colour = Some(self.colour);
        active.player.info.chat_effects = Some(self.effect);
        active.player.info.chat_crown = Some(active.staffmodlevel().min(2));
        active.player.info.masks |= PlayerInfoProt::Chat as u16;

        active.last_social = Some(clock);
        counters::add(&COUNTERS.chat[CHAT_PUBLIC], 1);
        active.log_chat(chat_channel::PUBLIC, None, text);

        Ok(())
    }
}

/// The most characters 647's chat box takes (`script802`).
#[cfg(since_530)]
pub const MAX_CHAT_CHARS: usize = 80;

/// 530's line for chat meant for the clan chat when the player is in none,
/// as every player is: there is no clan chat to be in.
#[cfg(all(since_530, before_647))]
pub(crate) fn not_in_clan_chat(active: &mut ActivePlayer) {
    active.message_game("You must be in a clan chat to talk.");
}

/// The text of a chat message: `chars` characters, huffman-compressed.
/// `None` if the bytes run out first.
#[cfg(since_530)]
pub fn chat_text(huffman: &Huffman, mut bytes: Vec<u8>, chars: usize) -> Option<String> {
    let len = bytes.len();
    bytes.resize(len.max(chars << 2), 0);
    let mut decoded = vec![0; chars];
    let consumed = huffman.decode(&bytes, 0, &mut decoded, chars);
    (consumed <= len).then(|| cp1252_to_utf8(&decoded))
}

/// `text` as a chat message carries it: through the word filter with its
/// first letter capitalised, then a smart character count and huffman.
#[cfg(since_530)]
pub fn chat_bytes(huffman: &Huffman, wordenc: &WordEncProvider, text: &str) -> Vec<u8> {
    let mut decoded = utf8_to_cp1252(&wordenc.filter(text));
    if let Some(first) = decoded.first_mut() {
        first.make_ascii_uppercase();
    }
    let mut bytes = Packet::new(2 + (decoded.len() << 2));
    bytes.psmart1or2(decoded.len() as i32);
    let len = huffman.encode(&decoded, &mut bytes.data[bytes.pos..]);
    bytes.data.truncate(bytes.pos + len);
    bytes.data
}

/// [`chat_text`] with this world's huffman table.
#[cfg(since_530)]
pub(crate) fn decode_chat(bytes: Vec<u8>, chars: usize) -> Result<String, ScriptError> {
    chat_text(&engine().cache.huffman, bytes, chars)
        .ok_or_else(|| ScriptError::Client("Truncated chat message".to_string()))
}

/// [`chat_bytes`] with this world's huffman table and word filter.
#[cfg(since_530)]
pub(crate) fn filtered_chat(text: &str) -> Vec<u8> {
    chat_bytes(&engine().cache.huffman, engine().wordenc(), text)
}

/// Typed chat to the friends chat, the player's clan chat, or the one they
/// visit as a guest. The client only sends clan chat while it shows the
/// player in that channel at a rank that may talk.
#[cfg(since_647)]
fn channel_chat(
    active: &mut ActivePlayer,
    mode: u8,
    bytes: Vec<u8>,
    chars: usize,
) -> Result<(), ScriptError> {
    let Some(target) = social::channel_target(active, mode) else {
        return Ok(());
    };
    let counter = chat_counter(&target);
    if active.is_muted() {
        counters::add(&COUNTERS.chat_muted[counter], 1);
        return Ok(());
    }
    let text = decode_chat(bytes, chars)?;
    let payload = filtered_chat(&text);
    active.last_social = Some(engine().clock);
    counters::add(&COUNTERS.chat[counter], 1);
    let message = social::say(active, target, CHAT_TEXT, payload, text);
    engine().send_ether(message);
    Ok(())
}

/// The chat counter chat to `target` counts in.
#[cfg(since_647)]
pub(crate) fn chat_counter(target: &ChatTarget) -> usize {
    match target {
        ChatTarget::Friend(_) => CHAT_PRIVATE,
        ChatTarget::FriendsChat(_) => CHAT_FRIENDS_CHAT,
        ChatTarget::Clan(_) => CHAT_CLAN,
    }
}

#[cfg(since_530)]
impl ClientGameHandler for MessagePublic {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let clock = engine().clock;
        let char_count = self.chars as usize;
        if active.last_social == Some(clock)
            || self.colour > 11
            || self.effect > 5
            || !(1..=MAX_CHAT_CHARS).contains(&char_count)
        {
            return Ok(());
        }
        #[cfg(since_647)]
        if active.player.chat_mode != CHAT_MODE_PUBLIC {
            return channel_chat(active, active.player.chat_mode, self.bytes, char_count);
        }
        if active.is_muted() {
            counters::add(&COUNTERS.chat_muted[CHAT_PUBLIC], 1);
            return Ok(());
        }

        let text = decode_chat(self.bytes, char_count)?;
        active.player.info.chat_bytes = Some(filtered_chat(&text).into_boxed_slice());
        active.player.info.chat_colour = Some(self.colour);
        active.player.info.chat_effects = Some(self.effect);
        active.player.info.chat_crown = Some(active.staffmodlevel().min(2));
        active.player.info.masks |= PlayerInfoProt::Chat as u16;

        active.last_social = Some(clock);
        counters::add(&COUNTERS.chat[CHAT_PUBLIC], 1);
        active.log_chat(chat_channel::PUBLIC, None, text);

        Ok(())
    }
}
