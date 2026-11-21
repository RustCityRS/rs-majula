use crate::script::LobbyVm;
use rs_db::ChatLog;
use rs_engine::social::{
    self, CHAT_MODE_GUEST_CLAN, CHAT_TEXT, ChatTarget, MAX_PRIVATE_CHARS, Social, SocialClient,
    SocialRequest, chat_filters_valid, name_key,
};
use rs_engine::{ClientHandle, EtherOutbound, MAX_CHAT_CHARS, SubTable, chat_bytes, chat_text};
use rs_pack::cache::VarValue;
use rs_pack::startup::LobbyAssets;
use rs_pack::types::IfEvents;
use rs_protocol::network::game::client::ClientProtMessage;
use rs_protocol::network::game::client_prot::*;
use rs_protocol::network::game::server::ServerProtMessage;
use rs_protocol::network::game::server::cam_smoothreset::CamSmoothreset;
use rs_protocol::network::game::server::if_closesub::IfClosesub;
use rs_protocol::network::game::server::if_opensub::IfOpensub;
use rs_protocol::network::game::server::logout_full::LogoutFull;
use rs_protocol::network::game::server::message_game::MessageGame;
use rs_protocol::world_list::WorldList;
use rs_vm::trigger::ServerTriggerType;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;

const SESSION_LIMIT: Duration = Duration::from_secs(3600);
const NO_RESPONSE: Duration = Duration::from_secs(60);
/// One chat message a game tick, as on a world.
const CHAT_GAP: Duration = Duration::from_millis(600);

pub(crate) struct LobbyPlayer {
    pub handle: ClientHandle,
    pub user37: u64,
    pub ip: IpAddr,
    pub client_session: i64,
    pub member: bool,
    pub chat_filters: [u8; 3],
    pub display: String,
    pub staff: u8,
    pub muted_until: Option<Instant>,
    pub social: Social,
    pub chat_logs: Vec<ChatLog>,
    chat_mode: u8,
    last_chat: Option<Instant>,
    pub entered: Instant,
    last_response: Instant,
    pub top: u16,
    pub subs: SubTable,
    modals: Vec<i32>,
    pub varps: HashMap<u16, VarValue>,
    pub varcs: HashMap<u16, i32>,
    pub strings: HashMap<u16, String>,
    pub resume_buttons: Vec<i32>,
    pub tabs: HashMap<u8, u16>,
}

pub(crate) enum Disconnect {
    Closed,
    Logout,
}

impl LobbyPlayer {
    pub fn new(
        handle: ClientHandle,
        user37: u64,
        ip: IpAddr,
        client_session: i64,
        top: u16,
    ) -> Self {
        let entered = Instant::now();
        Self {
            handle,
            user37,
            ip,
            client_session,
            member: false,
            chat_filters: [0; 3],
            display: String::new(),
            staff: 0,
            muted_until: None,
            social: Social::default(),
            chat_logs: Vec::new(),
            chat_mode: 0,
            last_chat: None,
            entered,
            last_response: entered,
            top,
            subs: SubTable::default(),
            modals: Vec::new(),
            varps: HashMap::new(),
            varcs: HashMap::new(),
            strings: HashMap::new(),
            resume_buttons: Vec::new(),
            tabs: HashMap::new(),
        }
    }

    pub fn open_sub(&mut self, com: i32, interface: u16, kind: u8) {
        self.subs.opened(com, interface, kind);
        self.handle.write(IfOpensub {
            interface,
            com,
            walkable: kind,
            #[cfg(since_742)]
            xtea_keys: [0; 4],
        });
    }

    /// Opens a main, chat or side modal on `com`, which
    /// [`close_modals`](Self::close_modals) closes.
    pub fn open_modal(&mut self, com: i32, interface: u16, kind: u8) {
        if !self.modals.contains(&com) {
            self.modals.push(com);
        }
        self.open_sub(com, interface, kind);
    }

    pub fn close_sub(&mut self, com: i32) {
        self.subs.closed(com);
        self.handle.write(IfClosesub { com });
    }

    /// The interfaces open as modals.
    pub fn open_modals(&self) -> impl Iterator<Item = u16> + '_ {
        self.modals
            .iter()
            .filter_map(|com| self.subs.open.get(com).map(|&(id, _)| id))
    }

    /// Closes the main, chat and side modals, as a world's `if_close` does,
    /// and returns the interfaces closed.
    pub fn close_modals(&mut self) -> Vec<u16> {
        let mut closed = Vec::new();
        for com in std::mem::take(&mut self.modals) {
            if let Some(&(id, _)) = self.subs.open.get(&com) {
                closed.push(id);
                self.close_sub(com);
            }
        }
        closed
    }

    fn component_events(&self, assets: &LobbyAssets, com: i32, slot: u16) -> Option<IfEvents> {
        if !self.subs.is_open(self.top, (com as u32 >> 16) as u16) {
            return None;
        }
        let component = assets.interfaces.get_by_id(com)?;
        self.subs
            .events(com, slot, component.active_properties)
            .map(IfEvents)
    }

    pub fn tick(
        &mut self,
        now: Instant,
        assets: &LobbyAssets,
        vm: &LobbyVm,
        worlds: &WorldList,
        ether: &UnboundedSender<EtherOutbound>,
    ) -> Option<Disconnect> {
        if session_expired(self.entered, self.last_response, now) || self.handle.backlogged() {
            self.handle.write(LogoutFull);
            return Some(Disconnect::Logout);
        }
        if self.handle.disconnect_rx.try_recv().is_ok() || self.handle.inbox.is_closed() {
            return Some(Disconnect::Closed);
        }
        self.handle.pull_input();
        let mut sent_worlds = false;
        for _ in 0..32 {
            let (prot, mut buf) = match self.handle.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => break,
                Err(e) => {
                    warn!("{e}");
                    return Some(Disconnect::Logout);
                }
            };
            if !prot.validate(&buf.data) {
                return Some(Disconnect::Logout);
            }
            self.last_response = self.handle.arrived().unwrap_or(now);
            let len = buf.len();
            macro_rules! button {
                ($message:ty, $op:literal) => {{
                    let b = <$message>::decode(&mut buf, len);
                    if self
                        .component_events(assets, b.com, b.slot)
                        .is_some_and(|e| e.allows_op($op))
                    {
                        let trigger = ServerTriggerType::inv_button($op).unwrap();
                        if !self.trigger(assets, vm, trigger, Some(b.com)) && $op == 1 {
                            self.trigger(assets, vm, ServerTriggerType::IfButton, Some(b.com));
                        }
                    }
                }};
            }
            macro_rules! request {
                ($request:ident) => {{
                    if let Some(message) = $request::decode(&mut buf, len).ether(self) {
                        let _ = ether.send(message);
                    }
                }};
            }
            match prot {
                ClientProt::NoTimeout => {
                    self.handle.write(CamSmoothreset);
                }
                ClientProt::WorldlistFetch if !sent_worlds => {
                    sent_worlds = true;
                    let checksum = WorldlistFetch::decode(&mut buf, len).checksum;
                    for reply in worlds.reply(checksum) {
                        self.handle.write(reply);
                    }
                }
                ClientProt::IfButton1 => button!(IfButton1, 1),
                ClientProt::IfButton2 => button!(IfButton2, 2),
                ClientProt::IfButton3 => button!(IfButton3, 3),
                ClientProt::IfButton4 => button!(IfButton4, 4),
                ClientProt::IfButton5 => button!(IfButton5, 5),
                ClientProt::IfButton6 => button!(IfButton6, 6),
                ClientProt::IfButton7 => button!(IfButton7, 7),
                ClientProt::IfButton8 => button!(IfButton8, 8),
                ClientProt::IfButton9 => button!(IfButton9, 9),
                ClientProt::IfButton10 => button!(IfButton10, 10),
                ClientProt::IfButtonD => {
                    let b = IfButtonD::decode(&mut buf, len);
                    if self
                        .component_events(assets, b.com, b.slot)
                        .is_some_and(|e| e.is_draggable())
                        && self
                            .component_events(assets, b.com2, b.slot2)
                            .is_some_and(|e| e.is_drop_target())
                    {
                        self.trigger(assets, vm, ServerTriggerType::InvButtonD, Some(b.com));
                    }
                }
                ClientProt::FriendListAdd => request!(FriendListAdd),
                ClientProt::FriendListDel => request!(FriendListDel),
                ClientProt::IgnoreListAdd => request!(IgnoreListAdd),
                ClientProt::IgnoreListDel => request!(IgnoreListDel),
                ClientProt::FriendSetRank => request!(FriendSetRank),
                ClientProt::ClanJoinchatLeavechat => request!(ClanJoinchatLeavechat),
                ClientProt::ClanKickUser => request!(ClanKickUser),
                ClientProt::ClanchannelKickUser => request!(ClanchannelKickUser),
                #[cfg(since_742)]
                ClientProt::AffinedClansettingsAddBannedFromChannel => {
                    request!(AffinedClansettingsAddBannedFromChannel)
                }
                ClientProt::ChatSetMode => {
                    let mode = ChatSetMode::decode(&mut buf, len).mode;
                    if mode <= CHAT_MODE_GUEST_CLAN {
                        self.chat_mode = mode;
                    }
                }
                ClientProt::SetChatfiltersettings => {
                    let f = SetChatfiltersettings::decode(&mut buf, len);
                    self.set_chat_filters(f.public, f.private, f.trade, ether);
                }
                ClientProt::MessagePrivate => {
                    let pm = MessagePrivate::decode(&mut buf, len);
                    self.private_chat(now, assets, pm, ether);
                }
                ClientProt::MessagePublic => {
                    let chat = MessagePublic::decode(&mut buf, len);
                    self.channel_chat(now, assets, chat, ether);
                }
                ClientProt::CloseModal => {
                    for id in self.close_modals() {
                        self.trigger(assets, vm, ServerTriggerType::IfClose, Some(id.into()));
                    }
                }
                _ => {}
            }
            self.handle.read_buf = buf.data;
        }
        self.handle.flush();
        None
    }
}

impl LobbyPlayer {
    /// Whether the player said something under a game tick ago.
    fn chat_limited(&self, now: Instant) -> bool {
        self.last_chat
            .is_some_and(|last| now.duration_since(last) < CHAT_GAP)
    }

    fn muted(&self, now: Instant) -> bool {
        self.muted_until.is_some_and(|until| until > now)
    }

    /// A private message, as a world sends one: within a game tick of other
    /// chat, from a muted player, or to a name the client would never send,
    /// it is dropped.
    fn private_chat(
        &mut self,
        now: Instant,
        assets: &LobbyAssets,
        pm: MessagePrivate,
        ether: &UnboundedSender<EtherOutbound>,
    ) {
        let chars = pm.chars as usize;
        if self.chat_limited(now) || !(1..=MAX_PRIVATE_CHARS).contains(&chars) || self.muted(now) {
            return;
        }
        let Some(name) = name_key(&pm.name) else {
            return;
        };
        let Some(text) = chat_text(&assets.huffman, pm.bytes, chars) else {
            return;
        };
        let payload = chat_bytes(&assets.huffman, &assets.wordenc, &text);
        self.last_chat = Some(now);
        let message = social::say(self, ChatTarget::Friend(name), CHAT_TEXT, payload, text);
        let _ = ether.send(message);
    }

    /// Typed chat in the lobby goes to the friends chat or a clan chat; the
    /// lobby has no public chat.
    fn channel_chat(
        &mut self,
        now: Instant,
        assets: &LobbyAssets,
        chat: MessagePublic,
        ether: &UnboundedSender<EtherOutbound>,
    ) {
        let chars = chat.chars as usize;
        if self.chat_limited(now)
            || chat.colour > 11
            || chat.effect > 5
            || !(1..=MAX_CHAT_CHARS).contains(&chars)
        {
            return;
        }
        let Some(target) = social::channel_target(self, self.chat_mode) else {
            return;
        };
        if self.muted(now) {
            return;
        }
        let Some(text) = chat_text(&assets.huffman, chat.bytes, chars) else {
            return;
        };
        let payload = chat_bytes(&assets.huffman, &assets.wordenc, &text);
        self.last_chat = Some(now);
        let message = social::say(self, target, CHAT_TEXT, payload, text);
        let _ = ether.send(message);
    }

    /// The public, private and trade filters, as a world sets them: a value
    /// out of range drops the change. The account keeps them when the player
    /// leaves the lobby.
    pub fn set_chat_filters(
        &mut self,
        public: u8,
        private: u8,
        trade: u8,
        ether: &UnboundedSender<EtherOutbound>,
    ) {
        if !chat_filters_valid(public, private, trade) {
            return;
        }
        self.chat_filters = [public, private, trade];
        self.send_chat_filters(public, private, trade);
        let _ = ether.send(EtherOutbound::ChatModeUpdate {
            user37: self.user37,
            private_mode: private,
        });
    }
}

impl SocialClient for LobbyPlayer {
    fn write<M: ServerProtMessage>(&mut self, message: M) {
        self.handle.write(message);
    }

    fn social(&mut self) -> &mut Social {
        &mut self.social
    }

    fn social_message(&mut self, kind: i32, text: &str) {
        self.handle.write(MessageGame { kind, text });
    }

    fn log_chat(&mut self, channel: i16, to37: Option<u64>, text: String) {
        self.chat_logs.push(ChatLog {
            name_hash: self.user37,
            channel,
            to_name_hash: to37,
            x: 0,
            z: 0,
            y: 0,
            message: text,
        });
    }

    fn user37(&self) -> u64 {
        self.user37
    }

    fn tier(&self) -> u8 {
        self.staff.min(2)
    }

    fn shown_name(&self) -> String {
        self.display.clone()
    }
}

fn session_expired(entered: Instant, last_response: Instant, now: Instant) -> bool {
    now.duration_since(entered) >= SESSION_LIMIT || now.duration_since(last_response) >= NO_RESPONSE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traffic_keeps_the_socket_alive_until_the_one_hour_deadline() {
        let t = Instant::now();
        let last = t + SESSION_LIMIT - Duration::from_secs(1);
        assert!(!session_expired(t, last, last));
        assert!(session_expired(t, last, t + SESSION_LIMIT));
        assert!(!session_expired(
            t,
            t,
            t + NO_RESPONSE - Duration::from_millis(1)
        ));
        assert!(session_expired(t, t, t + NO_RESPONSE));
    }
}
