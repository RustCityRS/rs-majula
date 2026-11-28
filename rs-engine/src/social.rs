use crate::clients::client_ether::{EtherInbound, EtherOutbound, MAX_IGNORES};
#[cfg(before_742)]
use crate::handlers::message_public::MAX_CHAT_CHARS;
use rs_db::chat_channel;
pub use rs_entity::player::{
    CHAT_MODE_CLAN, CHAT_MODE_FRIENDS_CHAT, CHAT_MODE_GUEST_CLAN, chat_filters_valid,
    checked_chat_filters,
};
#[cfg(since_742)]
use rs_protocol::network::game::client_prot::AffinedClansettingsAddBannedFromChannel;
use rs_protocol::network::game::client_prot::{
    ClanJoinchatLeavechat, ClanKickUser, ClanchannelKickUser, FriendListAdd, FriendListDel,
    FriendSetRank, IgnoreListAdd, IgnoreListDel,
};
use rs_protocol::network::game::server::ServerProtMessage;
use rs_protocol::network::game::server::chat_filter_settings::ChatFilterSettings;
use rs_protocol::network::game::server::chat_filter_settings_privatechat::ChatFilterSettingsPrivatechat;
use rs_protocol::network::game::server::clanchannel_delta::{ClanChannelDeltaOp, ClanchannelDelta};
use rs_protocol::network::game::server::clanchannel_full::{
    ClanChannelData, ClanChannelUser, ClanchannelFull,
};
use rs_protocol::network::game::server::message_clanchannel::MessageClanchannel;
use rs_protocol::network::game::server::message_friendchat::MessageFriendchat;
use rs_protocol::network::game::server::message_private::MessagePrivate;
use rs_protocol::network::game::server::message_private_echo::MessagePrivateEcho;
use rs_protocol::network::game::server::message_quickchat_clanchannel::MessageQuickchatClanchannel;
use rs_protocol::network::game::server::message_quickchat_friendchat::MessageQuickchatFriendchat;
use rs_protocol::network::game::server::message_quickchat_private::MessageQuickchatPrivate;
use rs_protocol::network::game::server::message_quickchat_private_echo::MessageQuickchatPrivateEcho;
use rs_protocol::network::game::server::update_friendchat_channel_full::{
    FriendchatChannel, FriendchatMember, UpdateFriendchatChannelFull,
};
use rs_protocol::network::game::server::update_friendchat_channel_singleuser::{
    FRIENDCHAT_REMOVED, UpdateFriendchatChannelSingleuser,
};
use rs_protocol::network::game::server::update_friendlist::{FriendEntry, UpdateFriendlist};
#[cfg(since_742)]
use rs_protocol::network::game::server::update_ignorelist::{IgnoreEntry, UpdateIgnoreList};
#[cfg(before_742)]
use rs_protocol::network::game::server::update_ignorelist_full::{
    IgnoreEntry, UpdateIgnorelistFull,
};
#[cfg(before_742)]
use rs_protocol::network::game::server::update_ignorelist_partial::{
    IGNORE_TEMPORARY, UpdateIgnorelistPartial,
};
use rs_ver::TypeId;
use std::collections::VecDeque;
use std::ops::RangeInclusive;
use tracing::warn;

pub const CHAT_TEXT: u8 = 0;
pub const CHAT_QUICK: u8 = 1;

pub const FRIEND_MESSAGE: i32 = 30;
pub const FRIENDCHAT_MESSAGE: i32 = 11;

pub const PM_DELIVERED: u8 = 0;
pub const PM_UNAVAILABLE: u8 = 1;
pub const PM_NOT_FRIEND: u8 = 2;

const LOBBY_WORLD: u16 = 1100;
const PLATFORM_RUNESCAPE: u8 = 1;
const MAX_FRIENDCHAT: usize = 100;
const PENDING_PMS: usize = 16;

/// The ranks a friends chat owner can give a friend: none to General.
const FRIEND_RANKS: RangeInclusive<i8> = 0..=6;

/// The most characters a private message holds: the chat box's, and 255 from
/// 742, which takes private messages in the chat box.
#[cfg(before_742)]
pub const MAX_PRIVATE_CHARS: usize = MAX_CHAT_CHARS;
#[cfg(since_742)]
pub const MAX_PRIVATE_CHARS: usize = 255;

#[derive(Debug)]
pub struct NamedFriend {
    pub renamed: bool,
    pub name: String,
    pub prev: String,
    pub world: u16,
    pub members: bool,
    pub rank: u8,
}

#[derive(Debug)]
pub struct NamedIgnore {
    pub flags: u8,
    pub name: String,
    pub prev: String,
}

#[derive(Clone, Debug)]
pub struct ChannelMember {
    pub name: String,
    pub world: u16,
    pub rank: i8,
    pub combat: u8,
}

#[derive(Debug)]
pub struct FriendChannel {
    pub owner37: u64,
    pub owner: String,
    pub channel37: i64,
    pub min_kick: i8,
    pub members: Vec<ChannelMember>,
}

#[derive(Clone, Debug)]
pub struct ClanUser {
    pub name: String,
    pub rank: i8,
    pub world: u16,
}

#[derive(Debug)]
pub struct ClanChannel {
    pub hash: i64,
    pub update_num: i64,
    pub name: String,
    pub rank_kick: i8,
    pub rank_talk: i8,
    pub users: Vec<ClanUser>,
}

#[derive(Debug)]
pub enum ClanOp {
    Add {
        name: String,
        world: u16,
        rank: i8,
    },
    Update {
        slot: u16,
        rank: i8,
        world: u16,
        name: String,
    },
    Delete {
        slot: u16,
    },
    Base {
        name: Option<String>,
        rank_talk: i8,
        rank_kick: i8,
    },
}

/// A message id the 647 client keys its de-duplication on: 16 bits, then 24.
pub type MessageUid = (u16, u32);

/// What the server has told a client of its channels, so that it never sends
/// one a packet the client can't apply (which logs it out), and the private
/// messages it sent awaiting ether's verdict, for the chat log.
#[derive(Default)]
pub struct Social {
    pub friendchat: Option<FriendChannel>,
    pub clans: [Option<ClanChannel>; 2],
    pending: VecDeque<(u16, String)>,
    next_seq: u16,
}

impl Social {
    /// Keeps `text` for a private message and returns the number ether hands
    /// back with its verdict.
    pub fn pending_pm(&mut self, text: String) -> u16 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        if self.pending.len() == PENDING_PMS {
            self.pending.pop_front();
        }
        self.pending.push_back((seq, text));
        seq
    }

    fn take_pm(&mut self, seq: u16) -> Option<String> {
        let at = self.pending.iter().position(|&(s, _)| s == seq)?;
        self.pending.remove(at).map(|(_, text)| text)
    }

    /// The members of the friends chat the player is in, for its quick chat.
    pub fn friendchat_count(&self) -> u8 {
        self.friendchat
            .as_ref()
            .map_or(0, |c| c.members.len() as u8)
    }

    /// Their mean combat level; members in a lobby have none and don't count.
    pub fn friendchat_mean_combat(&self) -> u8 {
        let Some(channel) = &self.friendchat else {
            return 0;
        };
        let levels: Vec<usize> = channel
            .members
            .iter()
            .filter(|m| m.combat > 0)
            .map(|m| m.combat as usize)
            .collect();
        if levels.is_empty() {
            0
        } else {
            (levels.iter().sum::<usize>() / levels.len()) as u8
        }
    }

    pub fn clan(&self, affined: bool) -> Option<&ClanChannel> {
        self.clans[affined as usize].as_ref()
    }
}

/// A world or lobby connection that social traffic is shown on.
pub trait SocialClient {
    fn write<M: ServerProtMessage>(&mut self, message: M);
    fn social(&mut self) -> &mut Social;
    fn social_message(&mut self, kind: i32, text: &str);
    fn log_chat(&mut self, channel: i16, to37: Option<u64>, text: String);
    /// The base-37 hash of the player's login name.
    fn user37(&self) -> u64;
    /// The crown the player's chat shows: none, player moderator or Jagex
    /// moderator.
    fn tier(&self) -> u8;
    /// The name the player shows as in channels.
    fn shown_name(&self) -> String;

    /// The public and trade filters, then the private one in its own packet.
    fn send_chat_filters(&mut self, public: u8, private: u8, trade: u8) {
        self.write(ChatFilterSettings { public, trade });
        self.write(ChatFilterSettingsPrivatechat { private });
    }
}

/// A social request from the client, as the ether message it becomes. `None`
/// drops one the client would never send.
pub trait SocialRequest {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound>;
}

/// The 647 client names the friend as typed. Ether finds the account by its display
/// name (or a previous one it still holds), checks the list, and answers with
/// the new entry or the reason it can't be added.
impl SocialRequest for FriendListAdd {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound> {
        Some(EtherOutbound::FriendAddName {
            owner37: client.user37(),
            name: name_key(&self.name)?,
        })
    }
}

/// The 647 client names the friend as it shows on the list, which the client has
/// already dropped it from; ether removes the account that name belongs to.
impl SocialRequest for FriendListDel {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound> {
        Some(EtherOutbound::FriendDelName {
            owner37: client.user37(),
            name: name_key(&self.name)?,
        })
    }
}

/// The 647 client names the player as typed, and the report-abuse screen's ignore is
/// temporary: it lasts until the player logs out. Ether finds the account and
/// answers with the new entry or the reason it can't be added.
impl SocialRequest for IgnoreListAdd {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound> {
        Some(EtherOutbound::IgnoreAddName {
            owner37: client.user37(),
            name: name_key(&self.name)?,
            temporary: self.temporary,
        })
    }
}

/// The 647 client names the player as it shows on the list, which the client has
/// already dropped it from.
impl SocialRequest for IgnoreListDel {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound> {
        Some(EtherOutbound::IgnoreDelName {
            owner37: client.user37(),
            name: name_key(&self.name)?,
        })
    }
}

/// Gives a friend a friends chat rank, 0 (none) to 6 (General), from the
/// friends chat setup. The client sends any rank its op gives; a higher one
/// comes from a modified client.
impl SocialRequest for FriendSetRank {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound> {
        if !FRIEND_RANKS.contains(&self.rank) {
            return None;
        }
        Some(EtherOutbound::FriendSetRank {
            owner37: client.user37(),
            rank: self.rank as u8,
            name: name_key(&self.name)?,
        })
    }
}

/// Joins the friends chat of the named owner, or with no name leaves the one
/// the player is in.
impl SocialRequest for ClanJoinchatLeavechat {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound> {
        let user37 = client.user37();
        if self.name.is_empty() {
            return Some(EtherOutbound::FriendchatLeave { user37 });
        }
        Some(EtherOutbound::FriendchatJoin {
            user37,
            name: name_key(&self.name)?,
        })
    }
}

/// Kicks the named player from the friends chat, banning them for an hour.
/// The client only sends it while it shows a channel.
impl SocialRequest for ClanKickUser {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound> {
        client.social().friendchat.as_ref()?;
        Some(EtherOutbound::FriendchatKick {
            user37: client.user37(),
            name: name_key(&self.name)?,
        })
    }
}

/// Kicks a guest from a clan channel, by their slot and name. The client
/// sends it only for a channel it holds; ether checks the guest and the
/// player's rank against the channel as it is.
impl SocialRequest for ClanchannelKickUser {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound> {
        client.social().clan(self.affined)?;
        Some(EtherOutbound::ClanKick {
            user37: client.user37(),
            affined: self.affined,
            slot: self.index,
            name: self.name,
        })
    }
}

/// Bans a guest in the player's own clan's channel from it for good, which
/// needs the clan's settings. Ether checks it as it does a kick.
#[cfg(since_742)]
impl SocialRequest for AffinedClansettingsAddBannedFromChannel {
    fn ether<C: SocialClient>(self, client: &mut C) -> Option<EtherOutbound> {
        client.social().clan(true)?;
        Some(EtherOutbound::ClanBan {
            user37: client.user37(),
            slot: self.index,
            name: self.name,
        })
    }
}

/// Whom a chat message goes to: a friend by name, the friends chat (by its
/// owner, for the chat log) or a clan channel.
pub enum ChatTarget {
    Friend(String),
    FriendsChat(u64),
    Clan(bool),
}

/// Whom chat in `mode` goes to, a `CHAT_MODE_*` past public chat (quick chat
/// numbers its channels the same), if the player is in that channel. A player
/// in no friends chat is told so. Ether checks a clan channel's talk rank.
pub fn channel_target<C: SocialClient>(client: &mut C, mode: u8) -> Option<ChatTarget> {
    match mode {
        CHAT_MODE_FRIENDS_CHAT => {
            let owner37 = client.social().friendchat.as_ref().map(|c| c.owner37);
            if owner37.is_none() {
                client.social_message(
                    FRIENDCHAT_MESSAGE,
                    "You are not currently in a friends chat channel.",
                );
            }
            owner37.map(ChatTarget::FriendsChat)
        }
        CHAT_MODE_CLAN | CHAT_MODE_GUEST_CLAN => {
            let affined = mode == CHAT_MODE_CLAN;
            client
                .social()
                .clan(affined)
                .map(|_| ChatTarget::Clan(affined))
        }
        _ => None,
    }
}

/// The ether message for chat of `kind` to `target`, its `payload` the
/// huffman bytes or the phrase. `text` goes in the chat log, a private
/// message's once ether delivers it.
pub fn say<C: SocialClient>(
    client: &mut C,
    target: ChatTarget,
    kind: u8,
    payload: Vec<u8>,
    text: String,
) -> EtherOutbound {
    let (user37, tier) = (client.user37(), client.tier());
    match target {
        ChatTarget::Friend(name) => EtherOutbound::PmSend {
            sender37: user37,
            seq: client.social().pending_pm(text),
            tier,
            kind,
            name,
            payload,
        },
        ChatTarget::FriendsChat(owner37) => {
            client.log_chat(chat_channel::FRIENDS_CHAT, Some(owner37), text);
            EtherOutbound::FriendchatSay {
                user37,
                tier,
                kind,
                payload,
            }
        }
        ChatTarget::Clan(affined) => {
            client.log_chat(chat_channel::CLAN, None, text);
            EtherOutbound::ClanSay {
                user37,
                affined,
                tier,
                kind,
                payload,
            }
        }
    }
}

/// What friends see of the player: their name, crown and combat level (for
/// friends chat quick chat), 0 in a lobby.
pub fn profile<C: SocialClient>(client: &C, combat: u8) -> EtherOutbound {
    EtherOutbound::Profile {
        user37: client.user37(),
        tier: client.tier(),
        combat,
        display: client.shown_name(),
    }
}

/// The world number friends see lobby `lobby_id` as: 1100 for lobby 1001.
pub fn lobby_world(lobby_id: u16) -> u16 {
    lobby_id + (LOBBY_WORLD - 1001)
}

/// How the client names a world: worlds by number, lobbies from 1100.
pub fn world_name(world: u16) -> String {
    if world >= LOBBY_WORLD {
        format!("Lobby {}", world - (LOBBY_WORLD - 1))
    } else {
        format!("RuneScape {world}")
    }
}

/// A name as the client compares names (`AClass31_Sub2.method2505`): trimmed
/// of separators, 1-12 characters, separators as `_`, accents folded,
/// lowercase. `None` for a name the client would never send.
pub fn name_key(input: &str) -> Option<String> {
    let separator = |c: char| matches!(c, ' ' | '\u{a0}' | '_' | '-');
    let trimmed = input.trim_matches(separator);
    let count = trimmed.chars().count();
    if !(1..=12).contains(&count) {
        return None;
    }
    let mut key = String::with_capacity(count);
    for c in trimmed.chars() {
        if c.is_control() {
            continue;
        }
        let folded = match c {
            c if separator(c) => '_',
            '[' | ']' | '#' => c,
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'À' | 'Á' | 'Â' | 'Ä' | 'Ã' => 'a',
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
            'í' | 'î' | 'ï' | 'Í' | 'Î' | 'Ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' | 'Ò' | 'Ó' | 'Ô' | 'Ö' | 'Õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ÿ' | 'Ÿ' => 'y',
            'ñ' | 'Ñ' => 'n',
            'ß' => 'b',
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            _ => continue,
        };
        key.push(folded);
    }
    (!key.is_empty()).then_some(key)
}

/// A quick chat message's phrase and the values its receivers read.
fn phrase(payload: &[u8]) -> Option<(TypeId, Vec<u8>)> {
    let (id, values) = payload.split_first_chunk::<2>()?;
    Some((TypeId::new(u16::from_be_bytes(*id)), values.to_vec()))
}

/// Shows one of ether's named frames to the client it is for. Returns `false`
/// for any other frame.
pub fn deliver<C: SocialClient>(client: &mut C, message: EtherInbound) -> bool {
    match message {
        EtherInbound::Friends { friends, .. } => {
            client.write(UpdateFriendlist {
                friends: friends.into_iter().map(friend_entry).collect(),
            });
        }
        EtherInbound::Ignores { full, ignores, .. } => send_ignores(client, full, ignores),
        EtherInbound::PmNamed {
            uid,
            tier,
            kind,
            sender,
            payload,
            ..
        } => {
            let (msg_uid_hi, msg_uid_lo) = (uid.0, uid.1 as i32);
            if kind == CHAT_QUICK {
                if let Some((phrase_id, quickchat_data)) = phrase(&payload) {
                    client.write(MessageQuickchatPrivate {
                        sender,
                        display_name: None,
                        msg_uid_hi,
                        msg_uid_lo,
                        tier,
                        phrase_id,
                        quickchat_data,
                    });
                }
            } else {
                client.write(MessagePrivate {
                    sender,
                    display_name: None,
                    msg_uid_hi,
                    msg_uid_lo,
                    tier,
                    message_data: payload,
                });
            }
        }
        EtherInbound::PmResult {
            seq,
            target37,
            code,
            kind,
            target,
            payload,
            ..
        } => {
            let text = client.social().take_pm(seq);
            if code != PM_NOT_FRIEND
                && let Some(text) = text
            {
                client.log_chat(chat_channel::PRIVATE, Some(target37), text);
            }
            match code {
                PM_DELIVERED if kind == CHAT_QUICK => {
                    if let Some((phrase_id, quickchat_data)) = phrase(&payload) {
                        client.write(MessageQuickchatPrivateEcho {
                            recipient: target,
                            phrase_id,
                            quickchat_data,
                        });
                    }
                }
                PM_DELIVERED => client.write(MessagePrivateEcho {
                    recipient: target,
                    message_data: payload,
                }),
                PM_NOT_FRIEND => client.social_message(
                    FRIEND_MESSAGE,
                    "Unable to send message - player not on your friends list.",
                ),
                _ => client.social_message(
                    FRIEND_MESSAGE,
                    "That player is offline, or has privacy mode enabled.",
                ),
            }
        }
        EtherInbound::SocialMessage { kind, text, .. } => {
            client.social_message(i32::from(kind), &text);
        }
        EtherInbound::FriendchatChannel { channel, .. } => {
            let packet = friendchat_full(channel.as_ref());
            client.social().friendchat = channel;
            client.write(packet);
        }
        EtherInbound::FriendchatUser { member, .. } => friendchat_user(client, member),
        EtherInbound::FriendchatMessage {
            channel37,
            uid,
            tier,
            kind,
            sender,
            payload,
            ..
        } => {
            if client.social().friendchat.is_none() {
                return true;
            }
            let (msg_uid_hi, msg_uid_lo) = (uid.0, uid.1 as i32);
            if kind == CHAT_QUICK {
                if let Some((phrase_id, quickchat_data)) = phrase(&payload) {
                    client.write(MessageQuickchatFriendchat {
                        sender,
                        display_name: None,
                        channel37,
                        msg_uid_hi,
                        msg_uid_lo,
                        tier,
                        phrase_id,
                        quickchat_data,
                    });
                }
            } else {
                client.write(MessageFriendchat {
                    sender,
                    display_name: None,
                    channel37,
                    msg_uid_hi,
                    msg_uid_lo,
                    tier,
                    message_data: payload,
                });
            }
        }
        EtherInbound::ClanChannel {
            affined, channel, ..
        } => {
            let packet = clan_full(affined, channel.as_ref());
            client.social().clans[affined as usize] = channel;
            client.write(packet);
        }
        EtherInbound::ClanDelta {
            affined,
            hash,
            update_num,
            ops,
            ..
        } => clan_delta(client, affined, hash, update_num, ops),
        EtherInbound::ClanMessage {
            affined,
            uid,
            tier,
            kind,
            sender,
            payload,
            ..
        } => {
            if client.social().clans[affined as usize].is_none() {
                return true;
            }
            let (msg_uid_hi, msg_uid_lo) = (uid.0, uid.1 as i32);
            if kind == CHAT_QUICK {
                if let Some((phrase_id, quickchat_data)) = phrase(&payload) {
                    client.write(MessageQuickchatClanchannel {
                        affined,
                        sender,
                        msg_uid_hi,
                        msg_uid_lo,
                        tier,
                        phrase_id,
                        quickchat_data,
                    });
                }
            } else {
                client.write(MessageClanchannel {
                    affined,
                    sender,
                    msg_uid_hi,
                    msg_uid_lo,
                    tier,
                    message_data: payload,
                });
            }
        }
        _ => return false,
    }
    true
}

/// Sends the client the channels the server says it is in again, after a
/// reconnect that may have lost one in flight.
pub fn resend_channels<C: SocialClient>(client: &mut C) {
    let social = client.social();
    let packets = (
        social.friendchat.as_ref().map(|c| friendchat_full(Some(c))),
        [false, true].map(|affined| social.clan(affined).map(|c| clan_full(affined, Some(c)))),
    );
    if let Some(friendchat) = packets.0 {
        client.write(friendchat);
    }
    for clan in packets.1.into_iter().flatten() {
        client.write(clan);
    }
}

fn friendchat_full(channel: Option<&FriendChannel>) -> UpdateFriendchatChannelFull {
    UpdateFriendchatChannelFull {
        channel: channel.map(|c| FriendchatChannel {
            owner: c.owner.clone(),
            channel37: c.channel37,
            min_kick: c.min_kick,
            members: c
                .members
                .iter()
                .take(MAX_FRIENDCHAT)
                .map(friendchat_member)
                .collect(),
        }),
    }
}

fn friendchat_member(member: &ChannelMember) -> FriendchatMember {
    FriendchatMember {
        name: member.name.clone(),
        display_name: None,
        world: member.world,
        rank: member.rank,
        world_name: world_name(member.world),
    }
}

fn clan_full(affined: bool, channel: Option<&ClanChannel>) -> ClanchannelFull {
    ClanchannelFull {
        affined,
        channel: channel.map(|c| ClanChannelData {
            clan_hash: c.hash,
            update_num: c.update_num,
            name: c.name.clone(),
            rank_kick: c.rank_kick,
            rank_talk: c.rank_talk,
            users: c
                .users
                .iter()
                .map(|u| ClanChannelUser {
                    display_name: u.name.clone(),
                    rank: u.rank,
                    world: u.world,
                })
                .collect(),
        }),
    }
}

fn friend_entry(f: NamedFriend) -> FriendEntry {
    FriendEntry {
        renamed: f.renamed,
        world_name: world_name(f.world),
        name: f.name,
        previous_name: f.prev,
        world: f.world,
        rank: f.rank,
        #[cfg(since_742)]
        referred: false,
        platform: PLATFORM_RUNESCAPE,
        #[cfg(since_742)]
        world_flags: i32::from(f.members),
    }
}

/// 647 replaces its list with a full one, then adds temporary ignores one at
/// a time (the full list marks every entry permanent); 742 has one packet that
/// adds or updates entries.
#[cfg(before_742)]
fn send_ignores<C: SocialClient>(client: &mut C, full: bool, ignores: Vec<NamedIgnore>) {
    let (temporary, permanent): (Vec<_>, Vec<_>) = ignores
        .into_iter()
        .take(MAX_IGNORES)
        .partition(|i| i.flags & IGNORE_TEMPORARY != 0);
    let partial = if full {
        client.write(UpdateIgnorelistFull {
            entries: permanent.into_iter().map(ignore_entry).collect(),
        });
        temporary
    } else {
        permanent.into_iter().chain(temporary).collect()
    };
    for i in partial {
        client.write(UpdateIgnorelistPartial {
            flags: i.flags & IGNORE_TEMPORARY,
            entry: ignore_entry(i),
        });
    }
}

#[cfg(before_742)]
fn ignore_entry(ignore: NamedIgnore) -> IgnoreEntry {
    IgnoreEntry {
        name: ignore.name,
        display_name: String::new(),
        prev_name: ignore.prev,
        prev_display_name: String::new(),
    }
}

#[cfg(since_742)]
fn send_ignores<C: SocialClient>(client: &mut C, _full: bool, ignores: Vec<NamedIgnore>) {
    client.write(UpdateIgnoreList {
        entries: ignores
            .into_iter()
            .take(MAX_IGNORES)
            .map(|i| IgnoreEntry {
                flags: i.flags,
                name: i.name,
                prev_name: i.prev,
            })
            .collect(),
    });
}

/// A member joined, changed or left the friends chat. The client removes a
/// member by name and the world it lists them on, and drops the connection
/// if it is told of a member while it holds no channel.
fn friendchat_user<C: SocialClient>(client: &mut C, member: ChannelMember) {
    let Some(channel) = client.social().friendchat.as_mut() else {
        return;
    };
    let at = channel.members.iter().position(|m| m.name == member.name);
    let changed = if member.rank == FRIENDCHAT_REMOVED {
        match at {
            Some(at) if channel.members[at].world == member.world => {
                channel.members.remove(at);
                true
            }
            _ => false,
        }
    } else {
        match at {
            Some(at) => {
                let old = std::mem::replace(&mut channel.members[at], member.clone());
                old.rank != member.rank || old.world != member.world
            }
            None => {
                channel.members.push(member.clone());
                true
            }
        }
    };
    if changed {
        client.write(UpdateFriendchatChannelSingleuser {
            member: friendchat_member(&member),
        });
    }
}

/// A clan channel change. The client drops the connection on a delta for a
/// channel it doesn't hold, at another update number, or for a slot it hasn't
/// got, so one that doesn't fit is not sent.
fn clan_delta<C: SocialClient>(
    client: &mut C,
    affined: bool,
    hash: i64,
    update_num: i64,
    ops: Vec<ClanOp>,
) {
    let Some(channel) = client.social().clans[affined as usize].as_mut() else {
        return;
    };
    if channel.hash != hash || channel.update_num != update_num {
        warn!(
            "Clan channel delta {update_num} for {hash} doesn't follow {} for {}",
            channel.update_num, channel.hash
        );
        return;
    }
    let mut users = channel.users.clone();
    let mut packet = Vec::with_capacity(ops.len());
    for op in ops {
        packet.push(match op {
            ClanOp::Add { name, world, rank } => {
                users.push(ClanUser {
                    name: name.clone(),
                    rank,
                    world,
                });
                ClanChannelDeltaOp::AddUser {
                    display_name: name,
                    world,
                    rank,
                }
            }
            ClanOp::Update {
                slot,
                rank,
                world,
                name,
            } => {
                let Some(user) = users.get_mut(slot as usize) else {
                    return;
                };
                *user = ClanUser {
                    name: name.clone(),
                    rank,
                    world,
                };
                ClanChannelDeltaOp::UpdateUserDetails {
                    slot,
                    rank,
                    world,
                    display_name: name,
                }
            }
            ClanOp::Delete { slot } => {
                if slot as usize >= users.len() {
                    return;
                }
                users.remove(slot as usize);
                ClanChannelDeltaOp::DeleteUser { slot }
            }
            ClanOp::Base {
                name,
                rank_talk,
                rank_kick,
            } => ClanChannelDeltaOp::UpdateClanChannelBaseSettings {
                name,
                rank_talk,
                rank_kick,
            },
        });
    }
    for op in &packet {
        if let ClanChannelDeltaOp::UpdateClanChannelBaseSettings {
            name: Some(name),
            rank_talk,
            rank_kick,
        } = op
        {
            channel.name = name.clone();
            channel.rank_talk = *rank_talk;
            channel.rank_kick = *rank_kick;
        }
    }
    channel.users = users;
    channel.update_num += 1;
    client.write(ClanchannelDelta {
        affined,
        clan_hash: hash,
        update_num,
        ops: packet,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_compare_as_the_client_does() {
        assert_eq!(name_key(" Mod Ash ").as_deref(), Some("mod_ash"));
        assert_eq!(name_key("mod-ash").as_deref(), Some("mod_ash"));
        assert_eq!(name_key("Zézima").as_deref(), Some("zezima"));
        assert_eq!(name_key("[a]#b").as_deref(), Some("[a]#b"));
        assert_eq!(name_key("___"), None);
        assert_eq!(name_key("abcdefghijklm"), None);
        assert_eq!(name_key("!!"), None);
    }

    #[test]
    fn lobbies_count_from_1100() {
        assert_eq!(lobby_world(1001), 1100);
        assert_eq!(world_name(lobby_world(1003)), "Lobby 3");
        assert_eq!(world_name(1), "RuneScape 1");
        assert_eq!(world_name(1100), "Lobby 1");
        assert_eq!(world_name(1102), "Lobby 3");
    }

    #[test]
    fn a_pending_message_comes_back_once() {
        let mut social = Social::default();
        let first = social.pending_pm("one".to_string());
        let second = social.pending_pm("two".to_string());
        assert_eq!(social.take_pm(second).as_deref(), Some("two"));
        assert_eq!(social.take_pm(second), None);
        assert_eq!(social.take_pm(first).as_deref(), Some("one"));
        for i in 0..PENDING_PMS + 1 {
            social.pending_pm(i.to_string());
        }
        assert_eq!(social.pending.len(), PENDING_PMS);
    }
}
