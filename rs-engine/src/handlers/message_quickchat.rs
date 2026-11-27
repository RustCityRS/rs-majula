use crate::active_player::ActivePlayer;
use crate::core::engine;
#[cfg(before_647)]
use crate::counters::CHAT_PRIVATE;
use crate::counters::{self, CHAT_PUBLIC, COUNTERS};
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use crate::handlers::message_public::chat_counter;
#[cfg(all(since_530, before_647))]
use crate::handlers::message_public::not_in_clan_chat;
#[cfg(since_647)]
use crate::social::{self, CHAT_QUICK, ChatTarget, name_key};
use rs_db::chat_channel;
use rs_io::Packet;
use rs_pack::ParamValue;
use rs_pack::cache::quickchatphrase::QuickChatDynamicCommand;
use rs_pack::types::DummyItem;
use rs_protocol::network::game::client_prot::{MessageQuickchatPrivate, MessageQuickchatPublic};
use rs_protocol::network::game::info_prot::PlayerInfoProt;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::server::message_quickchat_private_echo::MessageQuickchatPrivateEcho;
use rs_util::bits::getbit_range;
use rs_ver::TypeId;
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};

/// `MESSAGE_QUICKCHAT_PUBLIC`'s first byte for public chat
/// (`activechatphrase_sendpublic`).
const PUBLIC_CHAT: u8 = 0;

/// The first byte for the friends chat (`activechatphrase_sendclan`; 530 calls
/// it the clan chat). 647 adds 2 and 3, the clan channel the player is in and
/// the one it visits as a guest.
const CLAN_CHAT: u8 = 1;
#[cfg(since_647)]
const CLAN_CHANNEL: u8 = 2;
#[cfg(since_647)]
const GUEST_CLAN_CHANNEL: u8 = 3;

/// The colour byte of a `MESSAGE_PUBLIC` that carries a phrase: the top bit
/// of the colour and effect the client reads as one `g2`.
const PHRASE_COLOUR: u8 = 0x80;

/// The highest key a list dialog offers: it lists its enum's strings from key
/// 0 up to the first empty one, at most 249 (`quickchat_phrase_listdialog`).
const MAX_LIST_KEY: u16 = 248;

/// The objs the quick chat obj search leaves out (`quickchat_objdialog_dosearch`).
const OBJ_SEARCH_EXCLUDED: &str = "enum_1547";

/// Handles the `MessageQuickchatPublic` client protocol message.
///
/// A phrase for public chat goes out with the player's chat as a
/// `MESSAGE_PUBLIC` with the phrase bit set, one per tick shared with typed
/// chat. A muted player can still use quick chat. 530 has no clan chat to
/// send one to. From 647 one goes to the friends chat or a clan channel the
/// player is in; the client only offers a channel's when it shows one.
impl ClientGameHandler for MessageQuickchatPublic {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let clock = engine().clock;
        if active.last_social == Some(clock) {
            return Ok(());
        }
        let (values, text) = phrase(active, self.id.g2(), &self.bytes)?;
        match self.chat {
            PUBLIC_CHAT => {}
            #[cfg(all(since_530, before_647))]
            CLAN_CHAT => {
                not_in_clan_chat(active);
                return Ok(());
            }
            #[cfg(since_647)]
            chat @ (CLAN_CHAT | CLAN_CHANNEL | GUEST_CLAN_CHANNEL) => {
                if let Some(target) = social::channel_target(active, chat) {
                    return channel_phrase(active, target, self.id.g2(), values, text);
                }
                if chat == CLAN_CHAT {
                    return Ok(());
                }
                return Err(ScriptError::Client(format!(
                    "Quick chat for a channel the player is not in: {chat}"
                )));
            }
            chat => {
                return Err(ScriptError::Client(format!(
                    "Quick chat for a channel the player is not in: {chat}"
                )));
            }
        }

        active.player.info.chat_bytes =
            Some(phrase_payload(self.id.g2(), &values).into_boxed_slice());
        active.player.info.chat_colour = Some(PHRASE_COLOUR);
        active.player.info.chat_effects = Some(0);
        active.player.info.chat_crown = Some(active.staffmodlevel().min(2));
        active.player.info.masks |= PlayerInfoProt::Chat as u16;

        active.last_social = Some(clock);
        counters::add(&COUNTERS.chat[CHAT_PUBLIC], 1);
        active.log_chat(chat_channel::PUBLIC, None, text);
        Ok(())
    }
}

/// A phrase's id and then its values, as chat and ether carry it.
fn phrase_payload(id: u16, values: &[u8]) -> Vec<u8> {
    let mut bytes = Packet::new(2 + values.len());
    bytes.p2(id);
    bytes.pdata(values, 0, values.len());
    bytes.data
}

/// A phrase for a friend, the friends chat, or a clan channel the player
/// is in or visits.
#[cfg(since_647)]
fn channel_phrase(
    active: &mut ActivePlayer,
    target: ChatTarget,
    id: u16,
    values: Vec<u8>,
    text: String,
) -> Result<(), ScriptError> {
    active.last_social = Some(engine().clock);
    counters::add(&COUNTERS.chat[chat_counter(&target)], 1);
    let message = social::say(
        active,
        target,
        CHAT_QUICK,
        phrase_payload(id, &values),
        text,
    );
    engine().send_ether(message);
    Ok(())
}

/// Handles the `MessageQuickchatPrivate` client protocol message.
///
/// A phrase for a friend, one per tick shared with all chat, even from a muted
/// player. On 530 one for a name not on the friends list is dropped, and the
/// sender gets it back as `MESSAGE_QUICKCHAT_PRIVATE_ECHO` with no relay.
#[cfg(all(since_530, before_647))]
impl ClientGameHandler for MessageQuickchatPrivate {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let clock = engine().clock;
        if active.last_social == Some(clock) {
            return Ok(());
        }
        let (values, text) = phrase(active, self.id.g2(), &self.bytes)?;
        let target37 = self.user37 as u64;
        if !active.friends.contains(&(target37 as i64)) {
            return Ok(());
        }

        active.write(MessageQuickchatPrivateEcho {
            user37: self.user37,
            phrase: self.id,
            bytes: &values,
        });
        active.last_social = Some(clock);
        counters::add(&COUNTERS.chat[CHAT_PRIVATE], 1);
        active.log_chat(chat_channel::PRIVATE, Some(target37), text);
        Ok(())
    }
}

/// From 647 ether delivers the phrase to the friend, and the sender gets it
/// back as `MESSAGE_QUICKCHAT_PRIVATE_ECHO` once it is delivered.
#[cfg(since_647)]
impl ClientGameHandler for MessageQuickchatPrivate {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let clock = engine().clock;
        if active.last_social == Some(clock) {
            return Ok(());
        }
        let (values, text) = phrase(active, self.id.g2(), &self.bytes)?;
        let Some(name) = name_key(&self.name) else {
            return Ok(());
        };
        channel_phrase(active, ChatTarget::Friend(name), self.id.g2(), values, text)
    }
}

/// The values `id`'s phrase goes out with, as its receivers read them, and the
/// text they show, for the chat log. The sender's client picks what a list,
/// obj or count dialog fills in, so it must be one the dialog offers. The
/// server fills in the rest, writing over the 0 the client sends for a stat.
/// From 647 the friends chat's member count and mean combat level are those
/// of the channel the player is in; a clan's values are 0, with none to be in.
fn phrase(active: &ActivePlayer, id: u16, sent: &[u8]) -> Result<(Vec<u8>, String), ScriptError> {
    let engine = engine();
    let Some(phrase) = engine.cache.chatphrases.get(TypeId::new(id)) else {
        return Err(ScriptError::Client(format!(
            "Unknown quick chat phrase: {id}"
        )));
    };
    let invalid = || ScriptError::Client(format!("Invalid values for quick chat phrase {id}"));
    let commands: Vec<_> = phrase
        .commands
        .iter()
        .map(|command| {
            (
                QuickChatDynamicCommand::from_id(command.id),
                &command.params,
            )
        })
        .collect();
    if sent.len()
        != commands
            .iter()
            .map(|(command, _)| command.encode_bytes)
            .sum::<usize>()
    {
        return Err(invalid());
    }

    let mut sent = Packet::from(sent);
    let mut values = Packet::new(
        commands
            .iter()
            .map(|(command, _)| command.decode_bytes)
            .sum(),
    );
    let mut parts = phrase.text.as_deref().unwrap_or_default().split('<');
    let mut text = parts.next().unwrap_or_default().to_string();
    for (command, params) in commands {
        let value = match command.name {
            "listdialog" => {
                let key = sent.g2();
                if !listed(params[0], key) {
                    return Err(invalid());
                }
                key as i32
            }
            "objdialog" | "objtradedialog" => {
                let obj = sent.g2();
                if !searchable(obj, command.name == "objtradedialog") {
                    return Err(invalid());
                }
                obj as i32
            }
            "countdialog" => {
                let count = sent.g4s();
                if count < 0 {
                    return Err(invalid());
                }
                count
            }
            "stat_base" => {
                if sent.g1() != 0 {
                    return Err(invalid());
                }
                active.stat_base(params[0] as usize) as i32
            }
            "enum_string" => active.get_var(params[1]).as_int(),
            "tostring_varp" => active.get_var(params[0]).as_int(),
            "tostring_varpbit" => {
                let varpbit = engine
                    .varpbits()
                    .get_by_id(TypeId::new(params[0]))
                    .ok_or(ScriptError::VarpBitNotFound(params[0] as i32))?;
                let base = active.get_var(varpbit.basevar).as_int();
                getbit_range(base, varpbit.start_bit as i32, varpbit.end_bit as i32)
            }
            "enum_string_statbase" => active.stat_base(params[1] as usize) as i32,
            "tostring_shared" => engine.get_var(params[0]).as_int(),
            "activecombatlevel" => active.player.combat_level as i32,
            #[cfg(since_647)]
            "acc_getcount_world" => i32::from(active.social.friendchat_count()),
            #[cfg(since_647)]
            "acc_getmeancombatlevel" => i32::from(active.social.friendchat_mean_combat()),
            _ => 0,
        };
        match command.decode_bytes {
            1 => values.p1(value as u8),
            2 => values.p2(value as u16),
            _ => values.p4(value),
        }
        match command.name {
            "objdialog" | "objtradedialog" => text.push_str(obj_name(value as u16)),
            "listdialog" | "enum_string" | "enum_string_clan" | "enum_string_statbase" => {
                text.push_str(enum_string(params[0], value))
            }
            _ => text.push_str(&value.to_string()),
        }
        text.push_str(parts.next().unwrap_or_default());
    }
    Ok((values.data, text))
}

/// Whether a list dialog for enum `id` offers `key`.
fn listed(id: u16, key: u16) -> bool {
    key <= MAX_LIST_KEY && (0..=key).all(|key| !enum_string(id, key as i32).is_empty())
}

/// Whether the quick chat obj search can list `id`: one `oc_find` finds (only
/// the stockmarket's for a phrase about trading), that has a name and
/// `enum_1547` doesn't leave out.
fn searchable(id: u16, stockmarket: bool) -> bool {
    let engine = engine();
    let excluded = engine
        .enums()
        .get_by_debugname(OBJ_SEARCH_EXCLUDED)
        .is_some_and(|excluded| match excluded.values.get(&(id as i32)) {
            Some(ParamValue::Int(value)) => *value != 0,
            _ => excluded.default_int != 0,
        });
    engine.objs().get_by_id(TypeId::new(id)).is_some_and(|obj| {
        #[cfg(since_647)]
        if obj.boughttemplate.is_some() {
            return false;
        }
        (!stockmarket || obj.stockmarket)
            && obj.certtemplate.is_none()
            && obj.lenttemplate.is_none()
            && obj.dummyitem == DummyItem::None
            && !excluded
            && !obj_name(id).eq_ignore_ascii_case("null")
    })
}

/// The name the client shows for obj `id`.
fn obj_name(id: u16) -> &'static str {
    engine()
        .objs()
        .get_by_id(TypeId::new(id))
        .and_then(|obj| obj.name.as_deref())
        .unwrap_or("null")
}

/// The string the client shows for `key` in enum `id`.
fn enum_string(id: u16, key: i32) -> &'static str {
    let Some(enum_type) = engine().enums().get_by_id(TypeId::new(id)) else {
        return "null";
    };
    match enum_type.values.get(&key) {
        Some(ParamValue::String(value)) => value,
        _ => enum_type.default_str.as_deref().unwrap_or("null"),
    }
}
