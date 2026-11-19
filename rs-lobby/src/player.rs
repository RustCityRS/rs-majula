use crate::script::LobbyVm;
use rs_engine::ClientHandle;
use rs_pack::cache::VarValue;
use rs_pack::startup::LobbyAssets;
use rs_pack::types::IfEvents;
use rs_protocol::network::game::client::ClientProtMessage;
use rs_protocol::network::game::client_prot::*;
use rs_protocol::network::game::server::cam_smoothreset::CamSmoothreset;
use rs_protocol::network::game::server::if_closesub::IfClosesub;
use rs_protocol::network::game::server::if_opensub::IfOpensub;
use rs_protocol::network::game::server::logout_full::LogoutFull;
use rs_protocol::world_list::WorldList;
use rs_vm::trigger::ServerTriggerType;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tracing::warn;

const SESSION_LIMIT: Duration = Duration::from_secs(3600);
const NO_RESPONSE: Duration = Duration::from_secs(60);

pub(crate) struct LobbyPlayer {
    pub handle: ClientHandle,
    pub user37: u64,
    pub ip: IpAddr,
    pub client_session: i64,
    pub member: bool,
    pub entered: Instant,
    last_response: Instant,
    pub top: u16,
    pub subs: HashMap<i32, (u16, u8)>,
    pub events: HashMap<(i32, u16), IfEvents>,
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
            entered,
            last_response: entered,
            top,
            subs: HashMap::new(),
            events: HashMap::new(),
            varps: HashMap::new(),
            varcs: HashMap::new(),
            strings: HashMap::new(),
            resume_buttons: Vec::new(),
            tabs: HashMap::new(),
        }
    }

    pub fn open_sub(&mut self, com: i32, interface: u16, kind: u8) {
        self.close_children(com);
        self.subs.insert(com, (interface, kind));
        self.handle.write(IfOpensub {
            interface,
            com,
            walkable: kind,
            #[cfg(since_742)]
            xtea_keys: [0; 4],
        });
    }

    pub(crate) fn close_children(&mut self, com: i32) {
        let Some((id, _)) = self.subs.remove(&com) else {
            return;
        };
        self.events
            .retain(|&(c, _), _| (c as u32 >> 16) != u32::from(id));
        let children: Vec<_> = self
            .subs
            .keys()
            .copied()
            .filter(|&c| c as u32 >> 16 == u32::from(id))
            .collect();
        for child in children {
            self.close_children(child);
        }
    }

    pub fn close_sub(&mut self, com: i32) {
        self.close_children(com);
        self.handle.write(IfClosesub { com });
    }

    pub fn close_modals(&mut self) {
        let modals: Vec<_> = self
            .subs
            .iter()
            .filter(|(_, (_, kind))| *kind == 0)
            .map(|(&c, _)| c)
            .collect();
        for com in modals {
            self.close_sub(com);
        }
    }

    fn component_events(&self, assets: &LobbyAssets, com: i32, slot: u16) -> Option<IfEvents> {
        let group = (com as u32 >> 16) as u16;
        if group != self.top && !self.subs.values().any(|&(id, _)| id == group) {
            return None;
        }
        let component = assets.interfaces.get_by_id(com)?;
        self.events
            .get(&(com, slot))
            .copied()
            .or_else(|| (slot == u16::MAX).then_some(IfEvents(component.active_properties)))
    }

    pub fn tick(
        &mut self,
        now: Instant,
        assets: &LobbyAssets,
        vm: &LobbyVm,
        worlds: &WorldList,
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
            match prot {
                ClientProt::NoTimeout => self.handle.write(CamSmoothreset),
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
                ClientProt::CloseModal => {
                    let modals: Vec<_> = self
                        .subs
                        .values()
                        .filter(|(_, kind)| *kind == 0)
                        .map(|&(id, _)| id)
                        .collect();
                    self.close_modals();
                    for id in modals {
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
