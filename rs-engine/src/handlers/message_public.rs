#[cfg(before_647)]
use crate::active_player::ActivePlayer;
#[cfg(before_647)]
use crate::counters::{self, CHAT_PUBLIC, COUNTERS};
use crate::engine::engine;
#[cfg(before_647)]
use crate::handlers::ClientGameHandler;
#[cfg(before_647)]
use rs_protocol::network::game::client::message_public::MessagePublic;
#[cfg(before_647)]
use rs_protocol::network::game::info_prot::PlayerInfoProt;
#[cfg(before_647)]
use rs_util::wordpack::{pack, unpack};
#[cfg(before_647)]
use rs_vm::ScriptError;
#[cfg(before_647)]
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

/// Rev-647 public chat: a huffman-compressed payload
/// (`colour, effect, smart char_count, huffman bytes`), unfiltered, echoed
/// back to nearby players as MESSAGE_PUBLIC server packets by the broadcast
/// step in the output phase rather than through an info mask.
/// MESSAGE_GAME's clan chat channel (`void-634` `ChatType.ClanChat`).
#[cfg(since_647)]
const CLAN_CHAT: i32 = 11;

#[cfg(since_647)]
impl crate::handlers::ClientGameHandler
    for rs_protocol::network::game::client::message_public::MessagePublic
{
    fn handle(
        self,
        active: &mut crate::active_player::ActivePlayer,
    ) -> Result<(), rs_vm::ScriptError> {
        // Typed chat in clan mode goes to the clan chat, which this server
        // does not have yet (it is post-289): nobody is in one, so the player
        // gets the not-in-a-clan reply on the clan channel (`void-634`
        // `Chat.kt`), and the message never reaches public chat.
        if active.player.chat_mode == rs_entity::player::CHAT_MODE_CLAN {
            active.message_game_typed(CLAN_CHAT, "You must be in a clan chat to talk.");
            return Ok(());
        }

        // Upstream's rules for public chat, as before 647: one message a
        // tick, none while muted.
        let clock = engine().clock;
        if active.last_social == Some(clock) {
            return Ok(());
        }
        if active.is_muted() {
            crate::counters::add(
                &crate::counters::COUNTERS.chat_muted[crate::counters::CHAT_PUBLIC],
                1,
            );
            return Ok(());
        }

        let mut buf = rs_io::Packet::from(self.payload);
        if buf.len().saturating_sub(buf.pos) < 2 {
            return Ok(());
        }

        let colour = buf.g1();
        let effect = buf.g1();
        if colour > 11 || effect > 5 {
            return Ok(());
        }

        let char_count = buf.gsmart1or2() as usize;
        if char_count == 0 || char_count > 80 {
            return Ok(());
        }

        let mut huffman_data = Vec::with_capacity(buf.len().saturating_sub(buf.pos));
        while buf.len().saturating_sub(buf.pos) > 0 {
            huffman_data.push(buf.g1());
        }

        let mut decoded = vec![0u8; char_count];
        engine()
            .cache
            .huffman
            .decode(&huffman_data, 0, &mut decoded, char_count);
        // The word filter, as before 647. The client's text is one byte a
        // character.
        use rs_vm::engine::ScriptEngine as _;
        let text: String = decoded.iter().map(|&b| b as char).collect();
        let mut decoded: Vec<u8> = engine()
            .wordenc()
            .filter(&text)
            .chars()
            .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
            .collect();
        if let Some(first) = decoded.get_mut(0) {
            first.make_ascii_uppercase();
        }
        let char_count = decoded.len();

        let mut encoded = vec![0u8; char_count * 2 + 1];
        let enc_len = engine().cache.huffman.encode(&decoded, &mut encoded);
        encoded.truncate(enc_len);

        active.last_social = Some(clock);
        crate::counters::add(
            &crate::counters::COUNTERS.chat[crate::counters::CHAT_PUBLIC],
            1,
        );
        active.log_chat(None, text);

        let effects = (effect as u16) | ((colour as u16) << 8);
        let rights = active.player.staff_mod_level as u8;
        active.player.pending_public_chat = Some((effects, rights, char_count, encoded));

        Ok(())
    }
}
