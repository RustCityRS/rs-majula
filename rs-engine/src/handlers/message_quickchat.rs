use crate::active_player::ActivePlayer;
use crate::core::engine;
use crate::counters::{self, CHAT_PRIVATE, CHAT_PUBLIC, COUNTERS};
use crate::handlers::ClientGameHandler;
use crate::handlers::message_public::not_in_clan_chat;
use rs_io::Packet;
use rs_pack::ParamValue;
use rs_pack::cache::quickchatphrase::QuickChatDynamicCommand;
use rs_pack::types::DummyItem;
use rs_protocol::network::game::client_prot::{MessageQuickchatPrivate, MessageQuickchatPublic};
use rs_protocol::network::game::info_prot::PlayerInfoProt;
use rs_protocol::network::game::server::message_quickchat_private_echo::MessageQuickchatPrivateEcho;
use rs_util::base37::to_userhash;
use rs_util::bits::getbit_range;
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};

/// `MESSAGE_QUICKCHAT_PUBLIC`'s first byte for public chat
/// (`activechatphrase_sendpublic`).
const PUBLIC_CHAT: u8 = 0;

/// The first byte for the clan chat (`activechatphrase_sendclan`). 2 and 3
/// are for the clan channel the player is in and the one it listens to.
const CLAN_CHAT: u8 = 1;

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
/// chat. A muted player can still use quick chat. One for the clan chat gets
/// 2011's line for a player in none, and the client only offers a clan
/// channel's when the server has sent it one, which it never does.
impl ClientGameHandler for MessageQuickchatPublic {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let clock = engine().clock;
        if active.last_social == Some(clock) {
            return Ok(());
        }
        let (values, text) = phrase(active, self.id, &self.bytes)?;
        match self.chat {
            PUBLIC_CHAT => {}
            CLAN_CHAT => {
                not_in_clan_chat(active);
                return Ok(());
            }
            chat => {
                return Err(ScriptError::Client(format!(
                    "Quick chat for a channel the player is not in: {chat}"
                )));
            }
        }

        let mut bytes = Packet::new(2 + values.len());
        bytes.p2(self.id);
        bytes.pdata(&values, 0, values.len());
        active.player.info.chat_bytes = Some(bytes.data.into_boxed_slice());
        active.player.info.chat_colour = Some(PHRASE_COLOUR);
        active.player.info.chat_effects = Some(0);
        active.player.info.chat_ignored = Some(active.staffmodlevel().min(2));
        active.player.info.masks |= PlayerInfoProt::Chat as u16;

        active.last_social = Some(clock);
        counters::add(&COUNTERS.chat[CHAT_PUBLIC], 1);
        active.log_chat(None, text);
        Ok(())
    }
}

/// Handles the `MessageQuickchatPrivate` client protocol message.
///
/// A phrase for a friend, one per tick shared with all chat, even from a muted
/// player. One for a name not on the friends list is dropped. The sender gets
/// it back as `MESSAGE_QUICKCHAT_PRIVATE_ECHO`; 647's ether relay is not built
/// yet, so it goes no further.
impl ClientGameHandler for MessageQuickchatPrivate {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let clock = engine().clock;
        if active.last_social == Some(clock) {
            return Ok(());
        }
        let (values, text) = phrase(active, self.id, &self.bytes)?;
        let target37 = to_userhash(&self.name);
        if !active.friends.contains(&(target37 as i64)) {
            return Ok(());
        }

        active.write(MessageQuickchatPrivateEcho {
            recipient: self.name,
            phrase_id: self.id,
            quickchat_data: values,
        });
        active.last_social = Some(clock);
        counters::add(&COUNTERS.chat[CHAT_PRIVATE], 1);
        active.log_chat(Some(target37), text);
        Ok(())
    }
}

/// The values `id`'s phrase goes out with, as its receivers read them, and the
/// text they show, for the chat log. The sender's client picks what a list,
/// obj or count dialog fills in, so it must be one the dialog offers. The
/// server fills in the rest, writing over the 0 the client sends for a stat;
/// a clan or clan chat's count is 0, with none to be in.
fn phrase(active: &ActivePlayer, id: u16, sent: &[u8]) -> Result<(Vec<u8>, String), ScriptError> {
    let engine = engine();
    let Some(phrase) = engine.cache.chatphrases.get(id) else {
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
            "tostring_varbit" => {
                let varbit = engine
                    .varbits()
                    .get_by_id(params[0])
                    .ok_or(ScriptError::VarbitNotFound(params[0] as i32))?;
                let base = active.get_var(varbit.basevar).as_int();
                getbit_range(base, varbit.start_bit as i32, varbit.end_bit as i32)
            }
            "enum_string_statbase" => active.stat_base(params[1] as usize) as i32,
            "tostring_shared" => engine.get_var(params[0]).as_int(),
            "activecombatlevel" => active.player.combat_level as i32,
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
    engine.objs().get_by_id(id).is_some_and(|obj| {
        (!stockmarket || obj.stockmarket)
            && obj.certtemplate.is_none()
            && obj.lenttemplate.is_none()
            && obj.boughttemplate.is_none()
            && obj.dummyitem == DummyItem::None
            && !excluded
            && !obj_name(id).eq_ignore_ascii_case("null")
    })
}

/// The name the client shows for obj `id`.
fn obj_name(id: u16) -> &'static str {
    engine()
        .objs()
        .get_by_id(id)
        .and_then(|obj| obj.name.as_deref())
        .unwrap_or("null")
}

/// The string the client shows for `key` in enum `id`.
fn enum_string(id: u16, key: i32) -> &'static str {
    let Some(enum_type) = engine().enums().get_by_id(id) else {
        return "null";
    };
    match enum_type.values.get(&key) {
        Some(ParamValue::String(value)) => value,
        _ => enum_type.default_str.as_deref().unwrap_or("null"),
    }
}
