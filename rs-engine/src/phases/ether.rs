use crate::clients::client_ether::{EtherInbound, EtherOutbound, max_friends_cap};
use crate::engine::Engine;
use rs_entity::build::MAX_PLAYERS;
use rs_protocol::LoginResponse;
use rs_protocol::network::game::server::message_private::MessagePrivate;
use rs_protocol::network::game::server::update_friendlist::UpdateFriendList;
use rs_protocol::network::game::server::update_ignorelist::UpdateIgnoreList;
use rs_vm::engine::ScriptPlayer;
use tracing::warn;

impl Engine {
    /// Processes the ether phase of the engine tick cycle.
    ///
    /// Drains inbound messages from the cross-server ether service (up to
    /// `MAX_PLAYERS` per tick to prevent starvation) and dispatches each
    /// message:
    ///
    /// * `UpdateFriendList` -- forwards a friend-list update packet to the
    ///   target player.
    /// * `UpdateIgnoreList` -- forwards an ignore-list update packet to
    ///   the target player.
    /// * `MessagePrivate` -- delivers a private message from another world
    ///   to the recipient player.
    /// * `LoginCheckResponse` -- completes or rejects a pending login
    ///   based on whether the ether allows it (not logged in elsewhere).
    /// * `EtherReconnected` -- handles reconnection to the ether service
    ///   by failing all in-flight logins and re-syncing all active
    ///   players.
    ///
    /// # Side Effects
    ///
    /// * Writes network packets to player output buffers.
    /// * Completes or rejects entries in `self.pending_logins`.
    /// * On reconnect, sends `PlayerResync` and `RefreshAll` to the ether.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `try_complete_login`, `EtherOutbound::PlayerResync`,
    ///   `EtherOutbound::RefreshAll`
    pub(crate) fn ether(&mut self) {
        for _ in 0..MAX_PLAYERS {
            let msg = match self.ether_rx.try_recv() {
                Ok(msg) => msg,
                Err(_) => break,
            };
            match msg {
                EtherInbound::UpdateFriendList {
                    target37,
                    friend37,
                    node,
                } => {
                    if let Some(pid) = self.find_pid_by_user37(target37)
                        && let Some(active) = self.get_player_mut(pid)
                    {
                        active.write(UpdateFriendList {
                            user37: friend37 as i64,
                            node,
                        });
                    }
                }
                EtherInbound::UpdateIgnoreList { target37, users37 } => {
                    if let Some(pid) = self.find_pid_by_user37(target37)
                        && let Some(active) = self.get_player_mut(pid)
                    {
                        let signed: Vec<i64> = users37.iter().map(|&h| h as i64).collect();
                        active.write(UpdateIgnoreList { users37: &signed });
                    }
                }
                EtherInbound::MessagePrivate {
                    recipient37,
                    sender37,
                    msg_id,
                    level,
                    bytes,
                } => {
                    if let Some(pid) = self.find_pid_by_user37(recipient37)
                        && let Some(active) = self.get_player_mut(pid)
                    {
                        active.write(MessagePrivate {
                            user37: sender37 as i64,
                            id: msg_id,
                            level,
                            bytes: &bytes,
                        });
                    }
                }
                EtherInbound::FriendListComplete { target37 } => {
                    #[cfg(since_254)]
                    if let Some(pid) = self.find_pid_by_user37(target37)
                        && let Some(active) = self.get_player_mut(pid)
                    {
                        active.friendlist_loaded(2);
                    }
                    #[cfg(before_254)]
                    let _ = target37;
                }
                EtherInbound::WorldReady => {}
                EtherInbound::LoginCheckResponse {
                    user37,
                    allowed,
                    ip_limited,
                } => {
                    if let Some(idx) = self
                        .pending_logins
                        .iter()
                        .position(|p| p.user37 == user37 && !p.ether_allowed)
                    {
                        let online_here = self.find_pid_by_user37(user37).is_some();
                        let reconnect = self.pending_logins[idx].request.reconnect;
                        if (allowed && !online_here) || (reconnect && online_here && !ip_limited) {
                            self.pending_logins[idx].ether_allowed = true;
                            self.try_complete_login(idx);
                        } else {
                            let response = if ip_limited {
                                LoginResponse::TooManyConnections
                            } else {
                                LoginResponse::AlreadyLoggedIn
                            };
                            let pending = self.pending_logins.swap_remove(idx);
                            let _ = pending.request.handle.outbox.send(vec![response as u8]);
                        }
                    }
                }
                EtherInbound::EtherDisconnected => {
                    self.ether_ready = false;
                    warn!("Ether disconnected - logins disabled");
                }
                EtherInbound::EtherReconnected => {
                    self.ether_ready = true;
                    let clock = self.clock;
                    let mut i = self.pending_logins.len();
                    while i > 0 {
                        i -= 1;
                        if self.pending_logins[i].clock < clock {
                            self.reject_pending_login(i, LoginResponse::CouldNotComplete);
                        }
                    }

                    if let Some(tx) = &self.ether_tx {
                        for &pid in self.player_list.processing.iter() {
                            if let Some(active) = self.player_list.players[pid as usize].as_mut() {
                                let _ = tx.send(EtherOutbound::PlayerResync {
                                    user37: active.uid().username37(),
                                    pid,
                                    private_mode: active.player.private as u8,
                                    max_friends: max_friends_cap(active.player.is_member),
                                    ip: active
                                        .remote_ip
                                        .map(|ip| ip.to_string())
                                        .unwrap_or_default(),
                                });
                            }
                        }
                        let _ = tx.send(EtherOutbound::RefreshAll);
                    }
                }
            }
        }
    }
}
