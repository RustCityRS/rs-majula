#[cfg(since_647)]
use crate::clients::client_ether::EtherOutbound;
use crate::clients::client_ether::{EtherInbound, EtherSession, finish_resync};
use crate::core::Engine;
#[cfg(since_647)]
use crate::social;
use rs_entity::build::MAX_PLAYERS;
use rs_protocol::LoginResponse;
#[cfg(before_647)]
use rs_protocol::network::game::server::message_private::MessagePrivate;
#[cfg(before_647)]
use rs_protocol::network::game::server::update_friendlist::UpdateFriendList;
#[cfg(before_647)]
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
    ///   From 647, a refusal because the player is in a lobby instead asks
    ///   that lobby to hand the player over (`EtherOutbound::LobbyTransfer`)
    ///   once this world has a slot for them, and marks the login to retry
    ///   its check (see `Engine::logins`). Any other refusal rejects with
    ///   [`LoginResponse::IpLimit`] when the address is at its limit, else
    ///   [`LoginResponse::Duplicate`].
    /// * `EtherDisconnected` -- disables logins and rejects every pending
    ///   login with [`LoginResponse::LoginServerOffline`].
    /// * `EtherReconnected` -- handles reconnection to the ether service
    ///   by failing all in-flight logins and re-syncing all active
    ///   players.
    ///
    /// # Side Effects
    ///
    /// * Writes network packets to player output buffers.
    /// * Completes or rejects entries in `self.pending_logins`.
    /// * From 647, sends `LobbyTransfer` to the ether for a login whose
    ///   player is in a lobby.
    /// * On reconnect, sends `PlayerResync` and `RefreshAll` to the ether.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `try_complete_login`, `login_slot`, `reject_pending_login`,
    ///   `EtherOutbound::LobbyTransfer`, `EtherOutbound::PlayerResync`,
    ///   `EtherOutbound::RefreshAll`
    pub(crate) fn ether(&mut self) {
        for _ in 0..MAX_PLAYERS {
            let msg = match self.ether_rx.try_recv() {
                Ok(msg) => msg,
                Err(_) => break,
            };
            #[cfg(since_647)]
            if let Some(target37) = msg.named_target() {
                if let Some(pid) = self.find_pid_by_user37(target37)
                    && let Some(active) = self.get_player_mut(pid)
                {
                    social::deliver(active, msg);
                }
                continue;
            }
            match msg {
                #[cfg(before_647)]
                EtherInbound::UpdateFriendList {
                    target37,
                    friend37,
                    node,
                } => {
                    if let Some(pid) = self.find_pid_by_user37(target37)
                        && let Some(active) = self.get_player_mut(pid)
                    {
                        let friend37 = friend37 as i64;
                        if !active.friends.contains(&friend37) {
                            active.friends.push(friend37);
                        }
                        #[cfg(before_530)]
                        active.write(UpdateFriendList {
                            user37: friend37,
                            node,
                        });
                        // The 530 client names the friend's world itself: world N
                        // is node N + 9, and world 0 is offline.
                        #[cfg(all(since_530, before_647))]
                        {
                            let world = u16::from(node).saturating_sub(9);
                            active.write(UpdateFriendList {
                                user37: friend37,
                                world,
                                rank: 0,
                                world_name: &format!("World {world}"),
                                world_flag: 1,
                            });
                        }
                    }
                }
                #[cfg(since_647)]
                EtherInbound::UpdateFriendList { .. } => {}
                #[cfg(before_647)]
                EtherInbound::UpdateIgnoreList { target37, users37 } => {
                    if let Some(pid) = self.find_pid_by_user37(target37)
                        && let Some(active) = self.get_player_mut(pid)
                    {
                        let signed: Vec<i64> = users37.iter().map(|&h| h as i64).collect();
                        active.write(UpdateIgnoreList { users37: &signed });
                        active.ignores = signed;
                    }
                }
                #[cfg(since_647)]
                EtherInbound::UpdateIgnoreList { .. } => {}
                #[cfg(before_647)]
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
                #[cfg(since_647)]
                EtherInbound::MessagePrivate { .. } => {}
                // From 647 a named session's friends list itself tells the
                // client it has loaded; FRIENDLIST_LOADED would set it back to
                // connecting.
                EtherInbound::FriendListComplete { target37 } => {
                    #[cfg(all(since_254, before_647))]
                    if let Some(pid) = self.find_pid_by_user37(target37)
                        && let Some(active) = self.get_player_mut(pid)
                    {
                        active.friendlist_loaded(2);
                    }
                    #[cfg(any(before_254, since_647))]
                    let _ = target37;
                }
                #[cfg(since_647)]
                EtherInbound::LobbyTransfer { .. } => {}
                EtherInbound::WorldReady => {}
                EtherInbound::LoginCheckResponse {
                    user37,
                    allowed,
                    ip_limited,
                    in_lobby,
                } => {
                    #[cfg(before_647)]
                    let _ = in_lobby;
                    if let Some(idx) = self
                        .pending_logins
                        .iter()
                        .position(|p| p.user37 == user37 && !p.ether_allowed)
                    {
                        let online_here = self.find_pid_by_user37(user37).is_some();
                        let reconnect = self.pending_logins[idx].request.reconnect;
                        if (allowed && !online_here) || (reconnect && online_here && !ip_limited) {
                            #[cfg(since_647)]
                            {
                                self.pending_logins[idx].retry_ether = false;
                            }
                            self.pending_logins[idx].ether_allowed = true;
                            self.try_complete_login(idx);
                        } else {
                            #[cfg(since_647)]
                            if in_lobby
                                && !online_here
                                && !reconnect
                                && self.pending_logins[idx].auth.is_some()
                            {
                                if let Err(code) =
                                    self.login_slot(self.pending_logins[idx].auth.unwrap())
                                {
                                    self.reject_pending_login(idx, code);
                                    continue;
                                }
                                let pending = &mut self.pending_logins[idx];
                                if let Some(tx) = &self.ether_tx {
                                    let _ = tx.send(EtherOutbound::LobbyTransfer {
                                        user37,
                                        client_session: pending.request.client_session,
                                        ip: pending.request.remote_addr.ip(),
                                        world: self.node_id,
                                        revision: env!("REV").parse().unwrap(),
                                    });
                                }
                                pending.retry_ether = true;
                                continue;
                            }
                            let response = if ip_limited {
                                LoginResponse::IpLimit
                            } else {
                                LoginResponse::Duplicate
                            };
                            self.reject_pending_login(idx, response);
                        }
                    }
                }
                EtherInbound::EtherDisconnected => {
                    self.ether_ready = false;
                    while !self.pending_logins.is_empty() {
                        self.reject_pending_login(
                            self.pending_logins.len() - 1,
                            LoginResponse::LoginServerOffline,
                        );
                    }
                    warn!("Ether disconnected - logins disabled");
                }
                EtherInbound::EtherReconnected(replay) => {
                    self.ether_ready = true;
                    let clock = self.clock;
                    let mut i = self.pending_logins.len();
                    while i > 0 {
                        i -= 1;
                        if self.pending_logins[i].clock < clock {
                            self.reject_pending_login(i, LoginResponse::BadSave);
                        }
                    }

                    if let Some(tx) = &self.ether_tx {
                        for &pid in self.player_list.processing.iter() {
                            if let Some(active) = self.player_list.players[pid as usize].as_ref() {
                                EtherSession {
                                    user37: active.uid().username37(),
                                    pid,
                                    member: active.player.is_member,
                                    private_mode: active.player.private_chat(),
                                    ip: active.remote_ip,
                                    #[cfg(since_647)]
                                    profile: active.ether_profile(),
                                }
                                .resync(
                                    tx,
                                    #[cfg(since_647)]
                                    active.social.friendchat.as_ref(),
                                );
                            }
                        }
                        finish_resync(tx, replay);
                    }
                }
                #[cfg(since_647)]
                _ => {}
            }
        }
    }
}
