#[cfg(since_647)]
use crate::social::{
    ChannelMember, ClanChannel, ClanOp, ClanUser, FriendChannel, MessageUid, NamedFriend,
    NamedIgnore, name_key,
};
use rs_io::Packet;
use rs_util::net::limit_address;
use std::io::{self, Error, ErrorKind};
use std::net::IpAddr;
#[cfg(since_647)]
use std::str::from_utf8;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::time::{Duration, Instant, sleep, timeout_at};
use tracing::{info, warn};

/// Opcodes for messages sent from this game world to the ether sidecar.
#[repr(u8)]
pub enum EtherOutboundOp {
    WorldRegister = 0,
    PlayerLogin = 1,
    PlayerLogout = 2,
    FriendAdd = 3,
    FriendDel = 4,
    IgnoreAdd = 5,
    IgnoreDel = 6,
    PrivateMessage = 7,
    RequestLists = 8,
    ChatModeUpdate = 9,
    PlayerResync = 10,
    LoginCheck = 11,
    RefreshAll = 12,
    LoginAbort = 13,
    #[cfg(since_647)]
    LobbyTransfer = 14,
    #[cfg(since_647)]
    Profile = 15,
    #[cfg(since_647)]
    FriendAddName = 16,
    #[cfg(since_647)]
    FriendDelName = 17,
    #[cfg(since_647)]
    IgnoreAddName = 18,
    #[cfg(since_647)]
    IgnoreDelName = 19,
    #[cfg(since_647)]
    FriendSetRank = 20,
    #[cfg(since_647)]
    PmSend = 21,
    #[cfg(since_647)]
    FriendchatJoin = 22,
    #[cfg(since_647)]
    FriendchatLeave = 23,
    #[cfg(since_647)]
    FriendchatSay = 24,
    #[cfg(since_647)]
    FriendchatKick = 25,
    #[cfg(since_647)]
    ClanSay = 26,
    #[cfg(since_647)]
    ClanKick = 27,
    #[cfg(since_647)]
    ClanBan = 28,
}

/// The most names an ignore list holds, as in the client and ether.
pub const MAX_IGNORES: usize = 100;

#[cfg(before_647)]
#[cfg_attr(rev = "225", allow(unused_variables))]
pub fn max_friends_cap(is_member: bool) -> u16 {
    #[cfg(since_244)]
    if is_member {
        return 200;
    }

    100
}

/// 200 for free players too since 25 May 2010.
#[cfg(since_647)]
pub fn max_friends_cap(_is_member: bool) -> u16 {
    200
}

/// What ether needs to start a player's session, from a world or a lobby.
pub struct EtherSession {
    pub user37: u64,
    pub pid: u16,
    pub member: bool,
    pub private_mode: u8,
    pub ip: Option<IpAddr>,
    /// What friends see of the player.
    #[cfg(since_647)]
    pub profile: EtherOutbound,
}

impl EtherSession {
    /// Starts the session at login: the player, then their private filter
    /// (ether starts a session with private chat on, and the saved mode has to
    /// reach it before the lists announce the player as online), from 647
    /// their profile, then a request for their lists.
    pub fn login(self, tx: &UnboundedSender<EtherOutbound>) {
        let user37 = self.user37;
        let _ = tx.send(EtherOutbound::PlayerLogin {
            user37,
            pid: self.pid,
            max_friends: max_friends_cap(self.member),
            ip: self.ip,
        });
        let _ = tx.send(EtherOutbound::ChatModeUpdate {
            user37,
            private_mode: self.private_mode,
        });
        #[cfg(since_647)]
        let _ = tx.send(self.profile);
        let _ = tx.send(EtherOutbound::RequestLists { user37 });
    }

    /// Starts the session again on a sidecar that came back, from 647 with
    /// the profile and the friends chat the player was in. Once every
    /// session is back, [`finish_resync`] ends it.
    pub fn resync(
        self,
        tx: &UnboundedSender<EtherOutbound>,
        #[cfg(since_647)] friendchat: Option<&FriendChannel>,
    ) {
        let user37 = self.user37;
        let _ = tx.send(EtherOutbound::PlayerResync {
            user37,
            pid: self.pid,
            private_mode: self.private_mode,
            max_friends: max_friends_cap(self.member),
            ip: self.ip,
        });
        #[cfg(since_647)]
        {
            let _ = tx.send(self.profile);
            if let Some(name) = friendchat.and_then(|c| name_key(&c.owner)) {
                let _ = tx.send(EtherOutbound::FriendchatJoin { user37, name });
            }
        }
    }
}

/// Ends a resync: ether refreshes every list, then gets what was sent while
/// it was down.
pub fn finish_resync(tx: &UnboundedSender<EtherOutbound>, replay: Vec<EtherOutbound>) {
    let _ = tx.send(EtherOutbound::RefreshAll);
    for message in replay {
        let _ = tx.send(message);
    }
}

/// Opcodes for messages received from the ether sidecar.
#[repr(u8)]
pub enum EtherInboundOp {
    UpdateFriendList = 128,
    UpdateIgnoreList = 129,
    MessagePrivate = 130,
    FriendListComplete = 131,
    LoginCheckResponse = 132,
    WorldReady = 133,
    #[cfg(since_647)]
    LobbyTransfer = 134,
    #[cfg(since_647)]
    Friends = 135,
    #[cfg(since_647)]
    Ignores = 136,
    #[cfg(since_647)]
    PmNamed = 137,
    #[cfg(since_647)]
    PmResult = 138,
    #[cfg(since_647)]
    SocialMessage = 139,
    #[cfg(since_647)]
    FriendchatChannel = 140,
    #[cfg(since_647)]
    FriendchatUser = 141,
    #[cfg(since_647)]
    FriendchatMessage = 142,
    #[cfg(since_647)]
    ClanChannel = 143,
    #[cfg(since_647)]
    ClanDelta = 144,
    #[cfg(since_647)]
    ClanMessage = 145,
}

/// How a node registers with its sidecar. From 647 it also says the world
/// number friends see it as (a lobby's from 1100) and whether it is a members
/// world, which makes the sidecar speak the named protocol.
#[derive(Clone, Copy, Debug)]
pub struct EtherRegistration {
    pub node_id: u8,
    pub lobby: bool,
    #[cfg(since_647)]
    pub world: u16,
    #[cfg(since_647)]
    pub members: bool,
}

/// Strongly-typed outbound messages to the ether sidecar.
///
/// Each variant corresponds to an [`EtherOutboundOp`] and carries its
/// payload fields.
#[derive(Debug)]
pub enum EtherOutbound {
    #[cfg(since_647)]
    LobbyTransfer {
        user37: u64,
        client_session: i64,
        ip: IpAddr,
        world: u8,
        revision: u16,
    },
    WorldRegister(EtherRegistration),
    PlayerLogin {
        user37: u64,
        pid: u16,
        max_friends: u16,
        ip: Option<IpAddr>,
    },
    PlayerLogout {
        user37: u64,
    },
    FriendAdd {
        owner37: u64,
        friend37: u64,
    },
    FriendDel {
        owner37: u64,
        friend37: u64,
    },
    IgnoreAdd {
        owner37: u64,
        ignore37: u64,
    },
    IgnoreDel {
        owner37: u64,
        ignore37: u64,
    },
    PrivateMessage {
        sender37: u64,
        target37: u64,
        level: u8,
        bytes: Vec<u8>,
    },
    RequestLists {
        user37: u64,
    },
    ChatModeUpdate {
        user37: u64,
        private_mode: u8,
    },
    PlayerResync {
        user37: u64,
        pid: u16,
        private_mode: u8,
        max_friends: u16,
        ip: Option<IpAddr>,
    },
    LoginCheck {
        user37: u64,
        max_per_ip: u8,
        ip: Option<IpAddr>,
    },
    LoginAbort {
        user37: u64,
        ip: Option<IpAddr>,
    },
    RefreshAll,
    #[cfg(since_647)]
    Profile {
        user37: u64,
        tier: u8,
        combat: u8,
        display: String,
    },
    #[cfg(since_647)]
    FriendAddName {
        owner37: u64,
        name: String,
    },
    #[cfg(since_647)]
    FriendDelName {
        owner37: u64,
        name: String,
    },
    #[cfg(since_647)]
    IgnoreAddName {
        owner37: u64,
        name: String,
        temporary: bool,
    },
    #[cfg(since_647)]
    IgnoreDelName {
        owner37: u64,
        name: String,
    },
    #[cfg(since_647)]
    FriendSetRank {
        owner37: u64,
        rank: u8,
        name: String,
    },
    #[cfg(since_647)]
    PmSend {
        sender37: u64,
        seq: u16,
        tier: u8,
        kind: u8,
        name: String,
        payload: Vec<u8>,
    },
    #[cfg(since_647)]
    FriendchatJoin {
        user37: u64,
        name: String,
    },
    #[cfg(since_647)]
    FriendchatLeave {
        user37: u64,
    },
    #[cfg(since_647)]
    FriendchatSay {
        user37: u64,
        tier: u8,
        kind: u8,
        payload: Vec<u8>,
    },
    #[cfg(since_647)]
    FriendchatKick {
        user37: u64,
        name: String,
    },
    #[cfg(since_647)]
    ClanSay {
        user37: u64,
        affined: bool,
        tier: u8,
        kind: u8,
        payload: Vec<u8>,
    },
    #[cfg(since_647)]
    ClanKick {
        user37: u64,
        affined: bool,
        slot: u16,
        name: String,
    },
    #[cfg(since_647)]
    ClanBan {
        user37: u64,
        slot: u16,
        name: String,
    },
}

/// Strongly-typed inbound messages received from the ether sidecar.
///
/// Each variant corresponds to an [`EtherInboundOp`] and carries its
/// decoded payload. The `EtherReconnected` variant is synthetic -- it is
/// emitted locally when the connection is re-established.
#[derive(Debug)]
pub enum EtherInbound {
    #[cfg(since_647)]
    LobbyTransfer {
        user37: u64,
        client_session: i64,
        ip: IpAddr,
        world: u8,
        revision: u16,
    },
    UpdateFriendList {
        target37: u64,
        friend37: u64,
        node: u8,
    },
    UpdateIgnoreList {
        target37: u64,
        users37: Vec<u64>,
    },
    MessagePrivate {
        recipient37: u64,
        sender37: u64,
        msg_id: i32,
        level: u8,
        bytes: Vec<u8>,
    },
    FriendListComplete {
        target37: u64,
    },
    LoginCheckResponse {
        user37: u64,
        allowed: bool,
        ip_limited: bool,
        in_lobby: bool,
    },
    WorldReady,
    #[cfg(since_647)]
    Friends {
        target37: u64,
        friends: Vec<NamedFriend>,
    },
    #[cfg(since_647)]
    Ignores {
        target37: u64,
        full: bool,
        ignores: Vec<NamedIgnore>,
    },
    #[cfg(since_647)]
    PmNamed {
        recipient37: u64,
        uid: MessageUid,
        tier: u8,
        kind: u8,
        sender: String,
        payload: Vec<u8>,
    },
    #[cfg(since_647)]
    PmResult {
        sender37: u64,
        seq: u16,
        target37: u64,
        code: u8,
        kind: u8,
        target: String,
        payload: Vec<u8>,
    },
    #[cfg(since_647)]
    SocialMessage {
        target37: u64,
        kind: u16,
        text: String,
    },
    #[cfg(since_647)]
    FriendchatChannel {
        target37: u64,
        channel: Option<FriendChannel>,
    },
    #[cfg(since_647)]
    FriendchatUser {
        target37: u64,
        member: ChannelMember,
    },
    #[cfg(since_647)]
    FriendchatMessage {
        target37: u64,
        channel37: i64,
        uid: MessageUid,
        tier: u8,
        kind: u8,
        sender: String,
        payload: Vec<u8>,
    },
    #[cfg(since_647)]
    ClanChannel {
        target37: u64,
        affined: bool,
        channel: Option<ClanChannel>,
    },
    #[cfg(since_647)]
    ClanDelta {
        target37: u64,
        affined: bool,
        hash: i64,
        update_num: i64,
        ops: Vec<ClanOp>,
    },
    #[cfg(since_647)]
    ClanMessage {
        target37: u64,
        affined: bool,
        uid: MessageUid,
        tier: u8,
        kind: u8,
        sender: String,
        payload: Vec<u8>,
    },
    /// The link is back. It carries what was sent while it was down that the
    /// sidecar still needs, to send once every player is resynced.
    EtherReconnected(Vec<EtherOutbound>),
    EtherDisconnected,
}

/// Ether's strings end in 0 on every revision.
#[cfg(since_647)]
const TERMINATOR: u8 = 0;

/// More than the fixed fields of any frame [`EtherOutbound::encode`] writes.
const FIXED: usize = 32;

/// The address ether counts `ip` under for its per-address limits (see
/// [`limit_address`]), or nothing for a player without one.
fn ip_text(ip: Option<IpAddr>) -> String {
    ip.map(|ip| limit_address(ip).to_string())
        .unwrap_or_default()
}

/// Grows `buf` to hold [`FIXED`] bytes and `rest` more from its position.
fn reserve(buf: &mut Packet, rest: usize) {
    let len = buf.pos + FIXED + rest;
    if buf.data.len() < len {
        buf.data.resize(len, 0);
    }
}

/// Starts the frame `op` for the player `user37`, with room for `rest` bytes
/// of strings and payload after its fixed fields.
fn head(buf: &mut Packet, op: EtherOutboundOp, user37: u64, rest: usize) {
    reserve(buf, rest);
    buf.p1(op as u8);
    buf.p8(user37 as i64);
}

impl EtherOutbound {
    /// Writes this message's frame into `buf` from its position, growing it
    /// to fit: the opcode, then the fields big-endian.
    pub fn encode(&self, buf: &mut Packet) {
        match self {
            Self::WorldRegister(registration) => {
                reserve(buf, 0);
                buf.p1(EtherOutboundOp::WorldRegister as u8);
                buf.p1(registration.node_id);
                #[cfg(before_647)]
                if registration.lobby {
                    buf.p1(1);
                }
                #[cfg(since_647)]
                {
                    buf.p1(registration.lobby as u8);
                    buf.p2(registration.world);
                    buf.p1(registration.members as u8);
                }
            }
            Self::PlayerLogin {
                user37,
                pid,
                max_friends,
                ip,
            } => {
                let ip = ip_text(*ip);
                head(buf, EtherOutboundOp::PlayerLogin, *user37, ip.len());
                buf.p2(*pid);
                buf.p2(*max_friends);
                buf.pdata(ip.as_bytes(), 0, ip.len());
            }
            Self::PlayerLogout { user37 } => head(buf, EtherOutboundOp::PlayerLogout, *user37, 0),
            Self::FriendAdd { owner37, friend37 } => {
                head(buf, EtherOutboundOp::FriendAdd, *owner37, 0);
                buf.p8(*friend37 as i64);
            }
            Self::FriendDel { owner37, friend37 } => {
                head(buf, EtherOutboundOp::FriendDel, *owner37, 0);
                buf.p8(*friend37 as i64);
            }
            Self::IgnoreAdd { owner37, ignore37 } => {
                head(buf, EtherOutboundOp::IgnoreAdd, *owner37, 0);
                buf.p8(*ignore37 as i64);
            }
            Self::IgnoreDel { owner37, ignore37 } => {
                head(buf, EtherOutboundOp::IgnoreDel, *owner37, 0);
                buf.p8(*ignore37 as i64);
            }
            Self::PrivateMessage {
                sender37,
                target37,
                level,
                bytes,
            } => {
                head(buf, EtherOutboundOp::PrivateMessage, *sender37, bytes.len());
                buf.p8(*target37 as i64);
                buf.p1(*level);
                buf.pdata(bytes, 0, bytes.len());
            }
            Self::RequestLists { user37 } => head(buf, EtherOutboundOp::RequestLists, *user37, 0),
            Self::ChatModeUpdate {
                user37,
                private_mode,
            } => {
                head(buf, EtherOutboundOp::ChatModeUpdate, *user37, 0);
                buf.p1(*private_mode);
            }
            Self::PlayerResync {
                user37,
                pid,
                private_mode,
                max_friends,
                ip,
            } => {
                let ip = ip_text(*ip);
                head(buf, EtherOutboundOp::PlayerResync, *user37, ip.len());
                buf.p2(*pid);
                buf.p1(*private_mode);
                buf.p2(*max_friends);
                buf.pdata(ip.as_bytes(), 0, ip.len());
            }
            Self::LoginCheck {
                user37,
                max_per_ip,
                ip,
            } => {
                let ip = ip_text(*ip);
                head(buf, EtherOutboundOp::LoginCheck, *user37, ip.len());
                buf.p1(*max_per_ip);
                buf.pdata(ip.as_bytes(), 0, ip.len());
            }
            Self::LoginAbort { user37, ip } => {
                let ip = ip_text(*ip);
                head(buf, EtherOutboundOp::LoginAbort, *user37, ip.len());
                buf.pdata(ip.as_bytes(), 0, ip.len());
            }
            #[cfg(since_647)]
            Self::LobbyTransfer {
                user37,
                client_session,
                ip,
                world,
                revision,
            } => {
                let ip = ip.to_canonical().to_string();
                head(buf, EtherOutboundOp::LobbyTransfer, *user37, ip.len());
                buf.p8(*client_session);
                buf.p1(*world);
                buf.p2(*revision);
                buf.pdata(ip.as_bytes(), 0, ip.len());
            }
            Self::RefreshAll => {
                reserve(buf, 0);
                buf.p1(EtherOutboundOp::RefreshAll as u8);
            }
            #[cfg(since_647)]
            Self::Profile {
                user37,
                tier,
                combat,
                display,
            } => {
                head(
                    buf,
                    EtherOutboundOp::Profile,
                    *user37,
                    Packet::sizejstr(display),
                );
                buf.p1(*tier);
                buf.p1(*combat);
                buf.pjstr(display, TERMINATOR);
            }
            #[cfg(since_647)]
            Self::FriendAddName { owner37, name } => {
                head(
                    buf,
                    EtherOutboundOp::FriendAddName,
                    *owner37,
                    Packet::sizejstr(name),
                );
                buf.pjstr(name, TERMINATOR);
            }
            #[cfg(since_647)]
            Self::FriendDelName { owner37, name } => {
                head(
                    buf,
                    EtherOutboundOp::FriendDelName,
                    *owner37,
                    Packet::sizejstr(name),
                );
                buf.pjstr(name, TERMINATOR);
            }
            #[cfg(since_647)]
            Self::IgnoreAddName {
                owner37,
                name,
                temporary,
            } => {
                head(
                    buf,
                    EtherOutboundOp::IgnoreAddName,
                    *owner37,
                    Packet::sizejstr(name),
                );
                buf.p1(*temporary as u8);
                buf.pjstr(name, TERMINATOR);
            }
            #[cfg(since_647)]
            Self::IgnoreDelName { owner37, name } => {
                head(
                    buf,
                    EtherOutboundOp::IgnoreDelName,
                    *owner37,
                    Packet::sizejstr(name),
                );
                buf.pjstr(name, TERMINATOR);
            }
            #[cfg(since_647)]
            Self::FriendSetRank {
                owner37,
                rank,
                name,
            } => {
                head(
                    buf,
                    EtherOutboundOp::FriendSetRank,
                    *owner37,
                    Packet::sizejstr(name),
                );
                buf.p1(*rank);
                buf.pjstr(name, TERMINATOR);
            }
            #[cfg(since_647)]
            Self::PmSend {
                sender37,
                seq,
                tier,
                kind,
                name,
                payload,
            } => {
                head(
                    buf,
                    EtherOutboundOp::PmSend,
                    *sender37,
                    Packet::sizejstr(name) + payload.len(),
                );
                buf.p2(*seq);
                buf.p1(*tier);
                buf.p1(*kind);
                buf.pjstr(name, TERMINATOR);
                buf.pdata(payload, 0, payload.len());
            }
            #[cfg(since_647)]
            Self::FriendchatJoin { user37, name } => {
                head(
                    buf,
                    EtherOutboundOp::FriendchatJoin,
                    *user37,
                    Packet::sizejstr(name),
                );
                buf.pjstr(name, TERMINATOR);
            }
            #[cfg(since_647)]
            Self::FriendchatLeave { user37 } => {
                head(buf, EtherOutboundOp::FriendchatLeave, *user37, 0)
            }
            #[cfg(since_647)]
            Self::FriendchatSay {
                user37,
                tier,
                kind,
                payload,
            } => {
                head(buf, EtherOutboundOp::FriendchatSay, *user37, payload.len());
                buf.p1(*tier);
                buf.p1(*kind);
                buf.pdata(payload, 0, payload.len());
            }
            #[cfg(since_647)]
            Self::FriendchatKick { user37, name } => {
                head(
                    buf,
                    EtherOutboundOp::FriendchatKick,
                    *user37,
                    Packet::sizejstr(name),
                );
                buf.pjstr(name, TERMINATOR);
            }
            #[cfg(since_647)]
            Self::ClanSay {
                user37,
                affined,
                tier,
                kind,
                payload,
            } => {
                head(buf, EtherOutboundOp::ClanSay, *user37, payload.len());
                buf.p1(*affined as u8);
                buf.p1(*tier);
                buf.p1(*kind);
                buf.pdata(payload, 0, payload.len());
            }
            #[cfg(since_647)]
            Self::ClanKick {
                user37,
                affined,
                slot,
                name,
            } => {
                head(
                    buf,
                    EtherOutboundOp::ClanKick,
                    *user37,
                    Packet::sizejstr(name),
                );
                buf.p1(*affined as u8);
                buf.p2(*slot);
                buf.pjstr(name, TERMINATOR);
            }
            #[cfg(since_647)]
            Self::ClanBan { user37, slot, name } => {
                head(
                    buf,
                    EtherOutboundOp::ClanBan,
                    *user37,
                    Packet::sizejstr(name),
                );
                buf.p2(*slot);
                buf.pjstr(name, TERMINATOR);
            }
        }
    }

    /// Whether the sidecar still needs this after the link drops and comes
    /// back: a change a player made, as opposed to session and login traffic
    /// the reconnect resync replaces.
    fn replayable(&self) -> bool {
        match self {
            Self::FriendAdd { .. }
            | Self::FriendDel { .. }
            | Self::IgnoreAdd { .. }
            | Self::IgnoreDel { .. }
            | Self::PrivateMessage { .. } => true,
            #[cfg(since_647)]
            Self::FriendAddName { .. }
            | Self::FriendDelName { .. }
            | Self::IgnoreAddName { .. }
            | Self::IgnoreDelName { .. }
            | Self::FriendSetRank { .. }
            | Self::PmSend { .. }
            | Self::FriendchatSay { .. }
            | Self::FriendchatKick { .. }
            | Self::ClanSay { .. }
            | Self::ClanKick { .. }
            | Self::ClanBan { .. } => true,
            _ => false,
        }
    }
}

/// Whether `n` more bytes are left to read in `buf`.
fn has(buf: &Packet<&[u8]>, n: usize) -> Option<()> {
    (buf.len() - buf.pos >= n).then_some(())
}

/// The string at `buf`'s position, if it ends inside the frame.
#[cfg(since_647)]
fn jstr(buf: &mut Packet<&[u8]>) -> Option<String> {
    buf.data[buf.pos..]
        .contains(&TERMINATOR)
        .then(|| buf.gjstr(TERMINATOR))
}

/// The rest of the frame.
fn rest(buf: &mut Packet<&[u8]>) -> Vec<u8> {
    let len = buf.len() - buf.pos;
    let mut bytes = vec![0; len];
    buf.gdata(&mut bytes, 0, len);
    bytes
}

/// `value`, if nothing is left to read in `buf`.
#[cfg(since_647)]
fn done<T>(buf: &Packet<&[u8]>, value: T) -> Option<T> {
    (buf.pos == buf.len()).then_some(value)
}

#[cfg(since_647)]
impl EtherInbound {
    /// The player a named frame is for.
    pub fn named_target(&self) -> Option<u64> {
        match self {
            Self::Friends { target37, .. }
            | Self::Ignores { target37, .. }
            | Self::SocialMessage { target37, .. }
            | Self::FriendchatChannel { target37, .. }
            | Self::FriendchatUser { target37, .. }
            | Self::FriendchatMessage { target37, .. }
            | Self::ClanChannel { target37, .. }
            | Self::ClanDelta { target37, .. }
            | Self::ClanMessage { target37, .. } => Some(*target37),
            Self::PmNamed { recipient37, .. } => Some(*recipient37),
            Self::PmResult { sender37, .. } => Some(*sender37),
            _ => None,
        }
    }

    fn decode_named(op: u8, buf: &mut Packet<&[u8]>) -> Option<Self> {
        has(buf, 8)?;
        let target37 = buf.g8s() as u64;
        let message = match op {
            op if op == EtherInboundOp::Friends as u8 => {
                has(buf, 2)?;
                let count = buf.g2();
                let mut friends = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    has(buf, 1)?;
                    let renamed = buf.g1() != 0;
                    let name = jstr(buf)?;
                    let prev = jstr(buf)?;
                    has(buf, 4)?;
                    friends.push(NamedFriend {
                        renamed,
                        name,
                        prev,
                        world: buf.g2(),
                        members: buf.g1() != 0,
                        rank: buf.g1(),
                    });
                }
                Self::Friends { target37, friends }
            }
            op if op == EtherInboundOp::Ignores as u8 => {
                has(buf, 3)?;
                let full = buf.g1() != 0;
                let count = buf.g2();
                let mut ignores = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    has(buf, 1)?;
                    ignores.push(NamedIgnore {
                        flags: buf.g1(),
                        name: jstr(buf)?,
                        prev: jstr(buf)?,
                    });
                }
                Self::Ignores {
                    target37,
                    full,
                    ignores,
                }
            }
            op if op == EtherInboundOp::PmNamed as u8 => {
                has(buf, 8)?;
                Self::PmNamed {
                    recipient37: target37,
                    uid: (buf.g2(), buf.g4s() as u32),
                    tier: buf.g1(),
                    kind: buf.g1(),
                    sender: jstr(buf)?,
                    payload: rest(buf),
                }
            }
            op if op == EtherInboundOp::PmResult as u8 => {
                has(buf, 12)?;
                Self::PmResult {
                    sender37: target37,
                    seq: buf.g2(),
                    target37: buf.g8s() as u64,
                    code: buf.g1(),
                    kind: buf.g1(),
                    target: jstr(buf)?,
                    payload: rest(buf),
                }
            }
            op if op == EtherInboundOp::SocialMessage as u8 => {
                has(buf, 2)?;
                Self::SocialMessage {
                    target37,
                    kind: buf.g2(),
                    text: jstr(buf)?,
                }
            }
            op if op == EtherInboundOp::FriendchatChannel as u8 => {
                if buf.pos == buf.len() {
                    return Some(Self::FriendchatChannel {
                        target37,
                        channel: None,
                    });
                }
                has(buf, 8)?;
                let owner37 = buf.g8s() as u64;
                let owner = jstr(buf)?;
                has(buf, 10)?;
                let channel37 = buf.g8s();
                let min_kick = buf.g1s();
                let count = buf.g1();
                let mut members = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let name = jstr(buf)?;
                    has(buf, 4)?;
                    members.push(ChannelMember {
                        name,
                        world: buf.g2(),
                        rank: buf.g1s(),
                        combat: buf.g1(),
                    });
                }
                Self::FriendchatChannel {
                    target37,
                    channel: Some(FriendChannel {
                        owner37,
                        owner,
                        channel37,
                        min_kick,
                        members,
                    }),
                }
            }
            op if op == EtherInboundOp::FriendchatUser as u8 => {
                has(buf, 4)?;
                let rank = buf.g1s();
                let world = buf.g2();
                let combat = buf.g1();
                Self::FriendchatUser {
                    target37,
                    member: ChannelMember {
                        name: jstr(buf)?,
                        world,
                        rank,
                        combat,
                    },
                }
            }
            op if op == EtherInboundOp::FriendchatMessage as u8 => {
                has(buf, 16)?;
                Self::FriendchatMessage {
                    target37,
                    channel37: buf.g8s(),
                    uid: (buf.g2(), buf.g4s() as u32),
                    tier: buf.g1(),
                    kind: buf.g1(),
                    sender: jstr(buf)?,
                    payload: rest(buf),
                }
            }
            op if op == EtherInboundOp::ClanChannel as u8 => {
                has(buf, 2)?;
                let affined = buf.g1() != 0;
                if buf.g1() == 0 {
                    return done(
                        buf,
                        Self::ClanChannel {
                            target37,
                            affined,
                            channel: None,
                        },
                    );
                }
                has(buf, 16)?;
                let hash = buf.g8s();
                let update_num = buf.g8s();
                let name = jstr(buf)?;
                has(buf, 4)?;
                let rank_kick = buf.g1s();
                let rank_talk = buf.g1s();
                let count = buf.g2();
                let mut users = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let name = jstr(buf)?;
                    has(buf, 3)?;
                    users.push(ClanUser {
                        name,
                        rank: buf.g1s(),
                        world: buf.g2(),
                    });
                }
                Self::ClanChannel {
                    target37,
                    affined,
                    channel: Some(ClanChannel {
                        hash,
                        update_num,
                        name,
                        rank_kick,
                        rank_talk,
                        users,
                    }),
                }
            }
            op if op == EtherInboundOp::ClanDelta as u8 => {
                has(buf, 19)?;
                let affined = buf.g1() != 0;
                let hash = buf.g8s();
                let update_num = buf.g8s();
                let count = buf.g2();
                let mut ops = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    has(buf, 1)?;
                    ops.push(match buf.g1() {
                        1 => {
                            let name = jstr(buf)?;
                            has(buf, 3)?;
                            ClanOp::Add {
                                name,
                                world: buf.g2(),
                                rank: buf.g1s(),
                            }
                        }
                        2 => {
                            has(buf, 5)?;
                            ClanOp::Update {
                                slot: buf.g2(),
                                rank: buf.g1s(),
                                world: buf.g2(),
                                name: jstr(buf)?,
                            }
                        }
                        3 => {
                            has(buf, 2)?;
                            ClanOp::Delete { slot: buf.g2() }
                        }
                        4 => {
                            has(buf, 1)?;
                            let name = if buf.g1() != 0 {
                                Some(jstr(buf)?)
                            } else {
                                None
                            };
                            let (rank_talk, rank_kick) = match name {
                                Some(_) => {
                                    has(buf, 2)?;
                                    (buf.g1s(), buf.g1s())
                                }
                                None => (0, 0),
                            };
                            ClanOp::Base {
                                name,
                                rank_talk,
                                rank_kick,
                            }
                        }
                        _ => return None,
                    });
                }
                Self::ClanDelta {
                    target37,
                    affined,
                    hash,
                    update_num,
                    ops,
                }
            }
            op if op == EtherInboundOp::ClanMessage as u8 => {
                has(buf, 9)?;
                Self::ClanMessage {
                    target37,
                    affined: buf.g1() != 0,
                    uid: (buf.g2(), buf.g4s() as u32),
                    tier: buf.g1(),
                    kind: buf.g1(),
                    sender: jstr(buf)?,
                    payload: rest(buf),
                }
            }
            _ => return None,
        };
        done(buf, message)
    }
}

impl EtherInbound {
    /// Decodes an inbound message from raw frame bytes.
    ///
    /// The first byte is the opcode; the remainder is the payload. Returns
    /// `None` if the data is empty, the opcode is unknown, or the payload
    /// is too short for the given opcode.
    ///
    /// # Arguments
    /// * `data` - The raw frame data (opcode + payload).
    ///
    /// # Returns
    /// `Some(message)` on success, or `None` if decoding fails.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let mut buf = Packet::from(data);
        has(&buf, 1)?;
        let op = buf.g1();
        match op {
            op if op == EtherInboundOp::UpdateFriendList as u8 => {
                has(&buf, 17)?;
                Some(Self::UpdateFriendList {
                    target37: buf.g8s() as u64,
                    friend37: buf.g8s() as u64,
                    node: buf.g1(),
                })
            }
            op if op == EtherInboundOp::UpdateIgnoreList as u8 => {
                has(&buf, 10)?;
                let target37 = buf.g8s() as u64;
                let count = buf.g2();
                let mut users37 = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    if has(&buf, 8).is_none() {
                        break;
                    }
                    users37.push(buf.g8s() as u64);
                }
                Some(Self::UpdateIgnoreList { target37, users37 })
            }
            op if op == EtherInboundOp::MessagePrivate as u8 => {
                has(&buf, 21)?;
                Some(Self::MessagePrivate {
                    recipient37: buf.g8s() as u64,
                    sender37: buf.g8s() as u64,
                    msg_id: buf.g4s(),
                    level: buf.g1(),
                    bytes: rest(&mut buf),
                })
            }
            op if op == EtherInboundOp::FriendListComplete as u8 => {
                has(&buf, 8)?;
                Some(Self::FriendListComplete {
                    target37: buf.g8s() as u64,
                })
            }
            op if op == EtherInboundOp::LoginCheckResponse as u8 => {
                has(&buf, 10)?;
                let user37 = buf.g8s() as u64;
                let allowed = buf.g1() != 0;
                let reason = buf.g1();
                Some(Self::LoginCheckResponse {
                    user37,
                    allowed,
                    ip_limited: reason == 2,
                    in_lobby: reason == 3,
                })
            }
            #[cfg(since_647)]
            op if op == EtherInboundOp::LobbyTransfer as u8 => {
                has(&buf, 20)?;
                Some(Self::LobbyTransfer {
                    user37: buf.g8s() as u64,
                    client_session: buf.g8s(),
                    world: buf.g1(),
                    revision: buf.g2(),
                    ip: from_utf8(&rest(&mut buf)).ok()?.parse().ok()?,
                })
            }
            op if op == EtherInboundOp::WorldReady as u8 => Some(Self::WorldReady),
            #[cfg(since_647)]
            op if (EtherInboundOp::Friends as u8..=EtherInboundOp::ClanMessage as u8)
                .contains(&op) =>
            {
                let message = Self::decode_named(op, &mut buf);
                if message.is_none() {
                    warn!(
                        "Malformed ether frame 0x{op:02X} ({} bytes)",
                        data.len() - 1
                    );
                }
                message
            }
            _ => {
                warn!("Unknown ether inbound opcode: 0x{:02X}", op);
                None
            }
        }
    }
}

/// Long-running async task that maintains a TCP connection to the ether
/// (cross-server communication) sidecar.
///
/// Connects to `127.0.0.1:{port}` with exponential backoff. On each
/// successful connection, performs a handshake (sends `WorldRegister`,
/// waits for `WorldReady`), then enters a bidirectional message relay loop.
///
/// On the first successful handshake, signals `ready_tx` so the engine
/// knows the ether link is available. On subsequent reconnects, sends an
/// `EtherReconnected` event.
///
/// # Arguments
/// * `port` - The TCP port of the ether sidecar.
/// * `registration` - How this node registers with its sidecar.
/// * `outbound_rx` - Channel receiver for messages to send to the sidecar.
/// * `inbound_tx` - Channel sender for messages received from the sidecar.
/// * `ready_tx` - One-shot sender signalled when the first handshake completes.
///
/// # Call Stack
/// **Calls:** [`run_connection`]
pub async fn ether_client_task(
    port: u16,
    registration: EtherRegistration,
    mut outbound_rx: UnboundedReceiver<EtherOutbound>,
    inbound_tx: UnboundedSender<EtherInbound>,
    ready_tx: oneshot::Sender<()>,
) {
    let addr = format!("127.0.0.1:{}", port);
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);
    let mut ready_tx = Some(ready_tx);

    loop {
        info!("Ether client connecting to {}...", addr);
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                info!("Ether client connected to {}", addr);
                backoff = Duration::from_secs(1);
                match run_connection(
                    stream,
                    registration,
                    &mut outbound_rx,
                    &inbound_tx,
                    &mut ready_tx,
                )
                .await
                {
                    Ok(()) => {
                        info!("Ether outbound channel closed -- stopping ether client task");
                        return;
                    }
                    Err(e) => {
                        warn!("Ether connection lost: {}", e);
                        let _ = inbound_tx.send(EtherInbound::EtherDisconnected);
                    }
                }
            }
            Err(e) => {
                warn!("Ether connect failed: {} (retry in {:?})", e, backoff);
            }
        }
        sleep(backoff).await;
        backoff = (backoff * 2).min(max_backoff);
    }
}

/// Runs a single ether connection lifecycle: handshake then bidirectional
/// message relay until the connection drops.
///
/// Sends a `WorldRegister` frame, waits up to 10 seconds for a `WorldReady`
/// response during the handshake phase (a sidecar that never acknowledges
/// the registration fails the connection, so the caller retries), then
/// enters a `tokio::select!` loop that forwards outbound messages to the
/// sidecar and decodes inbound frames.
///
/// # Arguments
/// * `stream` - The connected TCP stream to the sidecar.
/// * `registration` - How this node registers with its sidecar.
/// * `outbound_rx` - Channel receiver for messages to send.
/// * `inbound_tx` - Channel sender for received messages.
/// * `ready_tx` - One-shot sender consumed on first successful handshake.
///
/// # Returns
/// `Ok(())` when the outbound channel closes, or an I/O error when the
/// connection is lost.
///
/// # Call Stack
/// **Called by:** [`ether_client_task`]
/// **Calls:** [`write_frame`], [`try_read_frame`], [`EtherInbound::decode`]
async fn run_connection(
    mut stream: TcpStream,
    registration: EtherRegistration,
    outbound_rx: &mut UnboundedReceiver<EtherOutbound>,
    inbound_tx: &UnboundedSender<EtherInbound>,
    ready_tx: &mut Option<oneshot::Sender<()>>,
) -> io::Result<()> {
    stream.set_nodelay(true)?;

    let mut frame = Packet::new(256);
    write_frame(
        &mut stream,
        &mut frame,
        &EtherOutbound::WorldRegister(registration),
    )
    .await?;

    let mut read_buf = vec![0u8; 4096];
    let mut pending = Vec::new();

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let n = timeout_at(deadline, stream.read(&mut read_buf))
            .await
            .map_err(|_| Error::new(ErrorKind::TimedOut, "Ether registration timed out"))??;
        if n == 0 {
            return Err(Error::new(
                ErrorKind::ConnectionReset,
                "ether connection closed during handshake",
            ));
        }
        pending.extend_from_slice(&read_buf[..n]);
        let mut ready = false;
        while let Some(frame) = try_read_frame(&mut pending) {
            if let Some(msg) = EtherInbound::decode(&frame) {
                match msg {
                    EtherInbound::WorldReady => {
                        info!("Ether sidecar ready (handshake complete)");
                        ready = true;
                    }
                    other => {
                        let _ = inbound_tx.send(other);
                    }
                }
            }
        }
        if ready {
            break;
        }
    }

    let mut replay = Vec::new();
    let mut dropped = 0;
    while let Ok(msg) = outbound_rx.try_recv() {
        if msg.replayable() {
            replay.push(msg);
        } else {
            dropped += 1;
        }
    }
    if dropped + replay.len() > 0 {
        info!(
            "Ether reconnect: {} outbound messages to resend, {dropped} stale dropped",
            replay.len()
        );
    }

    if let Some(tx) = ready_tx.take() {
        let _ = tx.send(());
    }
    let _ = inbound_tx.send(EtherInbound::EtherReconnected(replay));

    loop {
        tokio::select! {
            msg = outbound_rx.recv() => {
                let Some(msg) = msg else {
                    return Ok(());
                };
                write_frame(&mut stream, &mut frame, &msg).await?;
            }
            result = stream.read(&mut read_buf) => {
                let n = result?;
                if n == 0 {
                    return Err(Error::new(
                        ErrorKind::ConnectionReset,
                        "ether connection closed",
                    ));
                }
                pending.extend_from_slice(&read_buf[..n]);
                while let Some(frame) = try_read_frame(&mut pending) {
                    if let Some(msg) = EtherInbound::decode(&frame) {
                        let _ = inbound_tx.send(msg);
                    }
                }
            }
        }
    }
}

/// Writes `message` to the TCP stream as one frame: a 2-byte big-endian
/// length, then the message, encoded in `frame`.
///
/// # Returns
/// `Ok(())` on success, or an I/O error.
async fn write_frame(
    stream: &mut TcpStream,
    frame: &mut Packet,
    message: &EtherOutbound,
) -> io::Result<()> {
    frame.pos = 2;
    message.encode(frame);
    frame.psize2((frame.pos - 2) as u16);
    stream.write_all(&frame.data[..frame.pos]).await
}

/// Attempts to extract a complete length-prefixed frame from the pending
/// read buffer.
///
/// If at least 2 bytes are available, reads the big-endian length prefix.
/// If the buffer contains enough bytes for the full frame, extracts and
/// returns the payload, draining the consumed bytes from `buf`.
///
/// # Arguments
/// * `buf` - The pending read buffer (accumulated TCP reads).
///
/// # Returns
/// `Some(frame)` if a complete frame was available, `None` otherwise.
fn try_read_frame(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    if buf.len() < 2 {
        return None;
    }
    let len = Packet::from(&buf[..2]).g2() as usize;
    if buf.len() < 2 + len {
        return None;
    }
    let frame = buf[2..2 + len].to_vec();
    buf.drain(..2 + len);
    Some(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(message: EtherOutbound) -> Vec<u8> {
        let mut buf = Packet::new(0);
        message.encode(&mut buf);
        buf.data.truncate(buf.pos);
        buf.data
    }

    /// The frames every revision sends and reads, as rs-ether's
    /// `protocol_test.exs` encodes and decodes them.
    #[test]
    fn frames_match_the_sidecar() {
        #[cfg(before_647)]
        assert_eq!(
            encoded(EtherOutbound::WorldRegister(EtherRegistration {
                node_id: 10,
                lobby: true,
            })),
            [0, 10, 1]
        );
        let mut login = vec![1, 0, 0, 0, 0, 0, 0, 0, 7, 0, 2, 0, 200];
        login.extend_from_slice(b"10.0.0.1");
        assert_eq!(
            encoded(EtherOutbound::PlayerLogin {
                user37: 7,
                pid: 2,
                max_friends: 200,
                ip: Some(IpAddr::from([10, 0, 0, 1])),
            }),
            login
        );
        assert_eq!(
            encoded(EtherOutbound::PlayerResync {
                user37: 7,
                pid: 2,
                private_mode: 1,
                max_friends: 100,
                ip: None,
            }),
            [10, 0, 0, 0, 0, 0, 0, 0, 7, 0, 2, 1, 0, 100]
        );
        let mut pm = vec![7, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 3];
        pm.extend_from_slice(b"hi");
        assert_eq!(
            encoded(EtherOutbound::PrivateMessage {
                sender37: 1,
                target37: 2,
                level: 3,
                bytes: b"hi".to_vec(),
            }),
            pm
        );
        assert_eq!(encoded(EtherOutbound::RefreshAll), [12]);

        let mut message = vec![
            130, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 42, 3,
        ];
        message.extend_from_slice(b"hi");
        assert!(matches!(
            EtherInbound::decode(&message),
            Some(EtherInbound::MessagePrivate { recipient37: 1, sender37: 2, msg_id: 42, level: 3, bytes })
                if bytes == b"hi"
        ));
        message.truncate(22);
        assert!(matches!(
            EtherInbound::decode(&message),
            Some(EtherInbound::MessagePrivate { recipient37: 1, sender37: 2, msg_id: 42, level: 3, bytes })
                if bytes.is_empty()
        ));
        assert!(EtherInbound::decode(&message[..21]).is_none());
        assert!(matches!(
            EtherInbound::decode(&[132, 0, 0, 0, 0, 0, 0, 0, 7, 1, 3]),
            Some(EtherInbound::LoginCheckResponse {
                user37: 7,
                allowed: true,
                ip_limited: false,
                in_lobby: true,
            })
        ));
        assert!(matches!(
            EtherInbound::decode(&[129, 0, 0, 0, 0, 0, 0, 0, 7, 0, 2, 0, 0, 0, 0, 0, 0, 0, 9]),
            Some(EtherInbound::UpdateIgnoreList { target37: 7, users37 }) if users37 == [9]
        ));
        assert!(EtherInbound::decode(&[128, 0, 0]).is_none());
    }

    /// The frames as rs-ether's `named_protocol_test.exs` encodes and decodes them.
    #[cfg(since_647)]
    #[test]
    fn named_frames_match_the_sidecar() {
        let registration = EtherRegistration {
            node_id: 10,
            lobby: true,
            world: 1100,
            members: false,
        };
        assert_eq!(
            encoded(EtherOutbound::WorldRegister(registration)),
            [0, 10, 1, 0x04, 0x4C, 0]
        );
        let mut pm = vec![21, 0, 0, 0, 0, 0, 0, 0, 1, 0, 7, 2, 0];
        pm.extend_from_slice(b"bob\0");
        pm.extend_from_slice(&[9, 8]);
        assert_eq!(
            encoded(EtherOutbound::PmSend {
                sender37: 1,
                seq: 7,
                tier: 2,
                kind: 0,
                name: "bob".into(),
                payload: vec![9, 8],
            }),
            pm
        );

        let mut friends = vec![135, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 1];
        friends.extend_from_slice(b"Zezima\0Old\0");
        friends.extend_from_slice(&[0, 7, 1, 3]);
        match EtherInbound::decode(&friends) {
            Some(EtherInbound::Friends {
                target37: 1,
                friends,
            }) => {
                assert_eq!(friends.len(), 1);
                let f = &friends[0];
                assert!(f.renamed && f.members);
                assert_eq!(
                    (f.name.as_str(), f.prev.as_str(), f.world, f.rank),
                    ("Zezima", "Old", 7, 3)
                );
            }
            other => panic!("{other:?}"),
        }

        let mut channel = vec![140, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 9];
        channel.extend_from_slice(b"Ash\0");
        channel.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 42, 7, 1]);
        channel.extend_from_slice(b"Bob\0");
        channel.extend_from_slice(&[0x04, 0x4C, 255, 99]);
        match EtherInbound::decode(&channel) {
            Some(EtherInbound::FriendchatChannel {
                channel: Some(c), ..
            }) => {
                assert_eq!(
                    (c.owner37, c.owner.as_str(), c.channel37, c.min_kick),
                    (9, "Ash", 42, 7)
                );
                let m = &c.members[0];
                assert_eq!(
                    (m.name.as_str(), m.world, m.rank, m.combat),
                    ("Bob", 1100, -1, 99)
                );
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            EtherInbound::decode(&[140, 0, 0, 0, 0, 0, 0, 0, 1]),
            Some(EtherInbound::FriendchatChannel { channel: None, .. })
        ));

        let mut result = vec![
            138, 0, 0, 0, 0, 0, 0, 0, 1, 0, 7, 0, 0, 0, 0, 0, 0, 0, 2, 1, 0,
        ];
        result.extend_from_slice(b"Bob\0");
        result.push(9);
        assert!(matches!(
            EtherInbound::decode(&result),
            Some(EtherInbound::PmResult { sender37: 1, seq: 7, target37: 2, code: 1, kind: 0, target, payload })
                if target == "Bob" && payload == [9]
        ));

        let mut delta = vec![144, 0, 0, 0, 0, 0, 0, 0, 1, 1];
        delta.extend_from_slice(&5i64.to_be_bytes());
        delta.extend_from_slice(&6i64.to_be_bytes());
        delta.extend_from_slice(&[0, 2, 3, 0, 1, 4, 0]);
        assert!(matches!(
            EtherInbound::decode(&delta),
            Some(EtherInbound::ClanDelta { affined: true, hash: 5, update_num: 6, ops, .. })
                if matches!(ops[..], [ClanOp::Delete { slot: 1 }, ClanOp::Base { name: None, .. }])
        ));
    }

    /// A frame that runs short or long, or a string without its end, is
    /// dropped, not misread.
    #[cfg(since_647)]
    #[test]
    fn a_malformed_named_frame_is_dropped() {
        assert!(EtherInbound::decode(&[135, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1]).is_none());
        let mut message = vec![139, 0, 0, 0, 0, 0, 0, 0, 1, 0, 30];
        message.extend_from_slice(b"Hi");
        assert!(EtherInbound::decode(&message).is_none(), "unended string");
        message.push(0);
        assert!(EtherInbound::decode(&message).is_some());
        message.push(0);
        assert!(EtherInbound::decode(&message).is_none(), "trailing bytes");
    }

    #[cfg(since_647)]
    #[test]
    fn player_changes_outlive_a_reconnect_and_sessions_do_not() {
        assert!(
            EtherOutbound::FriendAdd {
                owner37: 1,
                friend37: 2
            }
            .replayable()
        );
        assert!(
            EtherOutbound::FriendchatSay {
                user37: 1,
                tier: 0,
                kind: 0,
                payload: vec![],
            }
            .replayable()
        );
        assert!(!EtherOutbound::PlayerLogout { user37: 1 }.replayable());
        assert!(!EtherOutbound::RequestLists { user37: 1 }.replayable());
        assert!(
            !EtherOutbound::Profile {
                user37: 1,
                tier: 0,
                combat: 0,
                display: String::new(),
            }
            .replayable()
        );
    }
}
