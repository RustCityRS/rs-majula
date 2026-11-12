use crate::active_player::ActivePlayer;
use crate::core::engine;
use crate::counters::{self, CHAT_PUBLIC, COUNTERS};
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use rs_entity::player::CHAT_MODE_CLAN;
#[cfg(since_647)]
use rs_io::Packet;
#[cfg(since_647)]
use rs_io::cp1252::{decode_string, encode_string};
use rs_protocol::network::game::client::message_public::MessagePublic;
use rs_protocol::network::game::info_prot::PlayerInfoProt;
#[cfg(before_647)]
use rs_util::wordpack::{pack, unpack};
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};

/// Highest chat effect this revision's client can render.
///
/// `0` = none, `1` = wave, `2` = wave2. From 289 the client also renders
/// `3` = shake, `4` = scroll and `5` = slide.
#[cfg(before_289)]
const MAX_CHAT_EFFECT: u8 = 2;
#[cfg(all(since_289, before_647))]
const MAX_CHAT_EFFECT: u8 = 5;

#[cfg(before_647)]
const MAX_CHAT_COLOUR: u8 = 11;

#[cfg(before_647)]
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
///   and `info.chat_ignored` fields.
/// * Adds `PlayerInfoProt::Chat` to the player's info update mask.
/// * Logs the unfiltered message to the database (`ActivePlayer::log_chat`).
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** `cache().wordenc.filter`, `pack`/`unpack`
#[cfg(before_647)]
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
        active.player.info.chat_ignored = Some(active.staffmodlevel().min(2));
        active.player.info.masks |= PlayerInfoProt::Chat as u16;

        active.last_social = Some(clock);
        counters::add(&COUNTERS.chat[CHAT_PUBLIC], 1);
        active.log_chat(None, text);

        Ok(())
    }
}

#[cfg(since_647)]
const CLAN_CHAT: i32 = 11;

/// The most characters 647's chat box takes (`script802`).
#[cfg(since_647)]
pub(crate) const MAX_CHAT_CHARS: usize = 80;

/// 2011's line for chat meant for the clan chat when the player is in none,
/// as every player is: there is no clan chat to be in.
#[cfg(since_647)]
pub(crate) fn not_in_clan_chat(active: &mut ActivePlayer) {
    active.message_game_typed(CLAN_CHAT, "You must be in a clan chat to talk.");
}

/// The text of a 647 chat message: `chars` characters, huffman-compressed.
#[cfg(since_647)]
pub(crate) fn decode_chat(mut bytes: Vec<u8>, chars: usize) -> Result<String, ScriptError> {
    let len = bytes.len();
    bytes.resize(len.max(chars << 2), 0);
    let mut decoded = vec![0; chars];
    let consumed = engine()
        .cache
        .huffman
        .decode(&bytes, 0, &mut decoded, chars);
    if consumed > len {
        return Err(ScriptError::Client("Truncated chat message".to_string()));
    }
    Ok(decode_string(&decoded))
}

/// `text` as a 647 chat message carries it: through the word filter with its
/// first letter capitalised, then a smart character count and huffman.
#[cfg(since_647)]
pub(crate) fn filtered_chat(text: &str) -> Vec<u8> {
    let mut decoded = encode_string(&engine().wordenc().filter(text));
    if let Some(first) = decoded.first_mut() {
        first.make_ascii_uppercase();
    }
    let mut bytes = Packet::new(2 + (decoded.len() << 2));
    bytes.psmart1or2(decoded.len() as i32);
    let len = engine()
        .cache
        .huffman
        .encode(&decoded, &mut bytes.data[bytes.pos..]);
    bytes.data.truncate(bytes.pos + len);
    bytes.data
}

#[cfg(since_647)]
impl ClientGameHandler for MessagePublic {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if active.player.chat_mode == CHAT_MODE_CLAN {
            not_in_clan_chat(active);
            return Ok(());
        }
        let clock = engine().clock;
        let char_count = self.chars as usize;
        if active.last_social == Some(clock)
            || self.colour > 11
            || self.effect > 5
            || !(1..=MAX_CHAT_CHARS).contains(&char_count)
        {
            return Ok(());
        }
        if active.is_muted() {
            counters::add(&COUNTERS.chat_muted[CHAT_PUBLIC], 1);
            return Ok(());
        }

        let text = decode_chat(self.bytes, char_count)?;
        active.player.info.chat_bytes = Some(filtered_chat(&text).into_boxed_slice());
        active.player.info.chat_colour = Some(self.colour);
        active.player.info.chat_effects = Some(self.effect);
        active.player.info.chat_ignored = Some(active.staffmodlevel().min(2));
        active.player.info.masks |= PlayerInfoProt::Chat as u16;

        active.last_social = Some(clock);
        counters::add(&COUNTERS.chat[CHAT_PUBLIC], 1);
        active.log_chat(None, text);

        Ok(())
    }
}
