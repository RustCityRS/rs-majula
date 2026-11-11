use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::clients::client_db::{DB_FENCE, DbRequest};
use crate::clients::client_ether::{EtherOutbound, max_friends_cap};
use crate::clients::client_game::ClientHandle;
#[cfg(since_647)]
use crate::console::home_for;
use crate::core::Engine;
#[cfg(since_647)]
use crate::region::place_returning;
use crate::save::*;
use crate::watch::schedule::{self, End, Played, Schedule};
use crate::watch::{Target, Watch};
use rs_db::LoginDevice;
use rs_entity::StaffModLevel;
use rs_protocol::LoginResponse;
#[cfg(since_647)]
use rs_protocol::login::{game_login_reply, reconnect_reply};
use rs_protocol::network::game::server::update_reboot_timer::UpdateRebootTimer;
use rs_util::base37::to_raw_username;
use rs_vm::engine::ScriptPlayer;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rsmod::rsmod::flag::zone_flag::ZoneFlag;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{error, info};

/// Represents an incoming player login attempt received from the network layer.
///
/// Contains all the information needed to authenticate a player and establish
/// their session: the network handle for sending responses, credentials, and
/// client capability flags.
pub struct LoginRequest {
    pub handle: ClientHandle,
    pub username: Box<str>,
    pub password: Box<str>,
    pub low_memory: bool,
    pub remote_addr: SocketAddr,
    pub uid: Option<i32>,
    pub reconnect: bool,
    pub device: LoginDevice,
    #[cfg(since_647)]
    pub display_mode: u8,
}

/// Tracks the state of a login that is waiting for asynchronous validation.
///
/// A login requires both ether (cross-world) authorisation and database
/// profile loading before it can be finalised. Each response arrives
/// independently, so this struct accumulates the results until both are
/// available, at which point [`Engine::try_complete_login`] promotes it
/// into a full player session.
///
/// The `profile` field uses `Option<Option<PlayerProfile>>`:
/// - `None` -- profile has not been fetched yet.
/// - `Some(None)` -- profile was fetched but does not exist (new player).
/// - `Some(Some(..))` -- profile was fetched and contains saved data.
pub struct PendingLogin {
    pub id: u32,
    pub user37: u64,
    pub request: LoginRequest,
    pub clock: u32,
    pub ether_allowed: bool,
    pub auth: Option<StaffModLevel>,
    pub muted_until: Option<Instant>,
    pub unread_messages: u16,
    pub claim_sent: bool,
    pub profile: Option<Option<PlayerProfile>>,
    pub played: Vec<rs_db::PlayedSession>,
}

/// A departed player's final save, held until the database stores it.
pub struct LogoutSave {
    pub username: String,
    pub profile: PlayerProfile,
    pub failed: bool,
}

/// Recent wrong-password attempts from one IP, for the
/// [`LoginResponse::TooManyAttempts`] lockout.
pub struct LoginFailures {
    pub count: u8,
    pub last: u32,
}

impl Engine {
    /// The profile `active` saves: where it stands and, in region space, the
    /// exit it logs back in at, as no region outlives the world.
    pub(crate) fn profile_of(&self, active: &ActivePlayer) -> PlayerProfile {
        #[cfg_attr(before_647, allow(unused_mut))]
        let mut profile = extract_profile(&active.player, self.cache);
        #[cfg(since_647)]
        {
            profile.exit = self.saved_exit(active.player.pathing.coord, active.player.region_entry);
        }
        profile
    }

    /// Saves a player who is leaving this world and logs them out of ether.
    /// Bot players have no persistent account and are skipped.
    ///
    /// The save ends the player's database session in the same transaction,
    /// and ether only hears of the logout once that save's ack comes back
    /// (see the `SaveAck` arm of [`Engine::saves`]). Until then the account
    /// still reads as logged in across the cluster and as busy on this world
    /// (it sits in `logout_saves`), so nothing can load the profile before
    /// the save lands. A save that fails is kept in `logout_saves` and sent
    /// again once the database is back. Without a database there is nothing
    /// to wait for and ether is told straight away.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `logouts`, `emergency_remove_player`.
    pub(crate) fn save_logout(&mut self, pid: u16) {
        let Some(active) = self.player_list.players[pid as usize].as_ref() else {
            return;
        };
        if active.player.bot {
            return;
        }
        let user37 = active.uid().username37();
        if self.db_tx.is_none() {
            self.release_logout(user37);
            return;
        }
        let profile = self.profile_of(active);
        let username = active.uid().username();
        self.logout_saves.insert(
            user37,
            LogoutSave {
                username,
                profile,
                failed: false,
            },
        );
        self.send_logout_save(user37);
    }

    /// Sends (or resends) the logout save held in `logout_saves`.
    pub(crate) fn send_logout_save(&mut self, user37: u64) {
        let (Some(tx), Some(save)) = (&self.db_tx, self.logout_saves.get_mut(&user37)) else {
            return;
        };
        save.failed = false;
        let _ = tx.send(DbRequest::Save {
            user37,
            username: save.username.clone(),
            binary: save_binary(&save.profile, self.cache),
            profile: Box::new(save.profile.clone()),
            logout: true,
        });
    }

    /// Whether this world's lease on its sessions may have run out: no
    /// heartbeat confirmed within [`DB_FENCE`]. Another world may take over
    /// this world's players [`DB_FENCE`] later, so it must stop serving them
    /// now. Never fenced without a database (no sessions to lose).
    pub(crate) fn lease_expired(&self) -> bool {
        self.db_tx.is_some() && self.db_lease.is_none_or(|at| at.elapsed() > DB_FENCE)
    }

    /// Fences this world when its lease has run out: removes every player
    /// (saving them, which falls back to local files while the database is
    /// unreachable) before any other world could take them over, so no
    /// player is ever live in two worlds at once. Runs first in every cycle,
    /// so not even one tick of input is served past the lease.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    pub(crate) fn fence(&mut self) {
        if !self.lease_expired() {
            return;
        }
        let pids: Vec<u16> = self
            .player_list
            .pids()
            .into_iter()
            .filter(|&pid| {
                self.player_list.players[pid as usize]
                    .as_ref()
                    .is_some_and(|active| !active.player.bot)
            })
            .collect();
        if pids.is_empty() {
            return;
        }
        error!(
            "No database heartbeat confirmed for {:?} - removing {} player(s) before another world may take them over",
            DB_FENCE,
            pids.len()
        );
        for pid in pids {
            self.emergency_remove_player(pid);
        }
    }

    /// Tells ether a departed player's session is over.
    pub(crate) fn release_logout(&self, user37: u64) {
        if let Some(tx) = &self.ether_tx {
            let _ = tx.send(EtherOutbound::PlayerLogout { user37 });
        }
    }

    /// Removes an online player whose session another world has taken over,
    /// without saving: the database refuses this world's copy, and the new
    /// owner's is the one that counts.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `saves` (`SessionsLost`, refused autosave).
    pub(crate) fn drop_unowned_player(&mut self, user37: u64) {
        let Some(pid) = self.find_pid_by_user37(user37) else {
            return;
        };
        error!(
            "Player '{}' (pid={}) lost their session to another world - removing unsaved",
            to_raw_username(user37),
            pid
        );
        self.remove_player(pid);
        self.release_logout(user37);
    }

    /// What the anti-cheat's schedule check goes by for a login now, after
    /// the week's sessions `played`: when the last was ended for it (not by
    /// the player), how long the player took to come back. That's from when
    /// the world let go of them, as a lost connection keeps the player in
    /// the world a while (and the client reconnects itself meanwhile). A
    /// `reconnect` login is that: the client's own, not the player's.
    fn schedule(&self, played: &[rs_db::PlayedSession], reconnect: bool) -> Schedule {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let end = |p: &rs_db::PlayedSession| p.started_ms + i64::from(p.duration_ms);
        let return_ms = played.last().filter(|_| !reconnect).and_then(|last| {
            let reason = End::from_name(last.end_reason.as_deref()?)?;
            schedule::return_ms(end(last), reason, now, self.date_origin_ms)
        });
        Schedule {
            history: played
                .iter()
                .map(|p| Played {
                    start_ms: p.started_ms,
                    end_ms: end(p),
                    return_ms: p.return_ms.and_then(|r| u32::try_from(r).ok()),
                })
                .collect(),
            login_ms: now,
            return_ms,
        }
    }

    /// Finalises a validated login by creating the player session and entering
    /// the world.
    ///
    /// Constructs an [`ActivePlayer`] for slot `pid`, applies the saved
    /// profile (or new-player defaults), sends the login success reply to the
    /// client, adds the player to the engine, calls `on_login`, notifies
    /// ether, and executes the `Login` trigger script.
    ///
    /// # Arguments
    ///
    /// * `request` -- The original [`LoginRequest`] with the client handle.
    /// * `profile` -- The player's saved profile, or `None` for a new player.
    /// * `staff_mod_level` -- From the player's database row, via authentication.
    /// * `muted_until` -- When the player's chat mute ends, likewise.
    /// * `unread_messages` -- Their unread Message Centre messages, likewise.
    /// * `pid` -- The accepted pid to use for the active player.
    /// * `played` -- The sessions the player played over the last week.
    ///
    /// # Side Effects
    ///
    /// - Sends the login success reply for the player's staff level to the client.
    /// - Starts the player's `Watch` from their login device and recent sessions.
    /// - Calls `add_player` and `on_login`.
    /// - Sends `UpdateRebootTimer` when a shutdown is pending.
    /// - Sends `PlayerLogin`, `ChatModeUpdate` and `RequestLists` to ether.
    /// - Executes the `Login` trigger script.
    /// - Tells isolated staff that they start visible.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `try_complete_login`, `harness_login_with`.
    /// **Calls:** `apply_profile`, `add_player`, `on_login`, `run_script_by_trigger`.
    pub(crate) fn accept_login(
        &mut self,
        request: LoginRequest,
        profile: Option<PlayerProfile>,
        staff_mod_level: StaffModLevel,
        muted_until: Option<Instant>,
        unread_messages: u16,
        pid: u16,
        played: &[rs_db::PlayedSession],
    ) {
        let mut active = ActivePlayer::new(
            request.handle,
            pid,
            request.username,
            request.low_memory,
            false,
        );

        if let Some(profile) = &profile {
            apply_profile(profile, &mut active.player, self.cache);
        }
        if active.player.stats.xp.iter().all(|&s| s == 0) {
            apply_new_player_defaults(&mut active.player);
        }
        active.player.staff_mod_level = staff_mod_level;
        active.muted_until = muted_until;
        active.unread_messages = unread_messages;

        active.player.last_date = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        active.player.last_response = self.clock;
        active.player.last_active = self.clock;

        let remote_ip = request.remote_addr.ip();
        active.remote_ip = Some(remote_ip);

        let uid = active.player.uid;

        #[cfg(since_254)]
        let level = active.staffmodlevel().min(2);
        #[cfg(all(since_254, before_647))]
        let reply = vec![LoginResponse::SuccessNormal as u8, level, 1];
        #[cfg(since_647)]
        let reply = {
            active.player.window_mode = request.display_mode;
            active.player.pathing.last_coord = active.player.pathing.coord;
            active.player.build_area.init_nsn_state(pid);
            game_login_reply(level, pid)
        };
        #[cfg(before_254)]
        let reply = {
            let response = match active.player.staff_mod_level {
                StaffModLevel::Normal => LoginResponse::SuccessNormal,
                StaffModLevel::PlayerModerator => LoginResponse::SuccessModerator,
                #[cfg(rev = "225")]
                StaffModLevel::JagexModerator | StaffModLevel::Maintainer => {
                    LoginResponse::SuccessModerator
                }
                #[cfg(since_244)]
                StaffModLevel::JagexModerator | StaffModLevel::Maintainer => {
                    LoginResponse::SuccessJagexModerator
                }
            };
            vec![response as u8]
        };
        if !active.player.bot {
            active.watch = Box::new(Watch::start(
                uid.username37(),
                self.node_id,
                self.replay_tx.as_ref(),
            ));
            active.watch.anticheat.touch = request.device.touch;
            active.watch.anticheat.automated = crate::watch::automated(&request.device);
            active.watch.anticheat.device = Arc::new(request.device.clone());
            active.watch.anticheat.schedule = self.schedule(played, request.reconnect);
            active.watch.login(self.clock, &reply);
        }
        active.handle.send(reply);

        info!(
            "Player '{}' joined at ({},{},{}) (uid={:?}, pid={})",
            active.player.uid.username(),
            active.player.pathing.coord.x(),
            active.player.pathing.coord.z(),
            active.player.pathing.coord.y(),
            uid,
            pid,
        );

        let key = match remote_ip {
            IpAddr::V4(ip) => u32::from(ip) as i64,
            IpAddr::V6(ip) => {
                let octets = ip.octets();
                u32::from_be_bytes([octets[12], octets[13], octets[14], octets[15]]) as i64
            }
        };
        self.add_player(pid, active, key);

        let login_info = if let Some(active) = self.get_player_mut(pid) {
            active.on_login();
            Some((
                active.uid().username37(),
                active.player.is_member,
                active.player.private_chat(),
            ))
        } else {
            None
        };

        if self.is_pending_shutdown() {
            let clocks = self.reboot_timer_clocks();
            if let Some(active) = self.get_player_mut(pid) {
                active.write(UpdateRebootTimer { clocks });
            }
        }

        if let (Some((user37, is_member, private_mode)), Some(tx)) = (login_info, &self.ether_tx) {
            let _ = tx.send(EtherOutbound::PlayerLogin {
                user37,
                pid,
                max_friends: max_friends_cap(is_member),
                ip: Some(remote_ip),
            });
            // Ether starts a session with private chat on; the saved mode has
            // to reach it before the lists announce the player as online.
            let _ = tx.send(EtherOutbound::ChatModeUpdate {
                user37,
                private_mode,
            });
            let _ = tx.send(EtherOutbound::RequestLists { user37 });
        }

        let result = self.run_script_by_trigger(
            (ServerTriggerType::Login, None, None),
            Some(ScriptSubject::Player(uid)),
            None,
            Some(true),
            None,
            None,
        );
        if result.is_err() {
            error!("{:?}", result);
        }

        // Visibility isn't saved, so whoever can hide starts every login seen.
        if let Some(active) = self.get_player_mut(pid)
            && active.player.staff_mod_level.is_isolated()
        {
            active.message_game("You're visible to other players. Use ::setvis 2 to hide.");
        }
    }

    /// Attaches a reconnecting client to its still-in-world player session.
    ///
    /// Called when a validated reconnect login (correct password, ether
    /// approved) finds its player still present on this node -- either
    /// lingering after a disconnect or still nominally connected (in which
    /// case the old socket is kicked and the new one takes over). The
    /// existing player keeps its pid, uid, position, inventories, and all
    /// in-flight gameplay state; nothing is saved or reloaded, so there is
    /// no rollback and no save/load race.
    ///
    /// 1. Sends [`LoginResponse::Reconnect`] (15) on the new connection --
    ///    the client resumes its session instead of resetting.
    /// 2. Swaps the new [`ClientHandle`] onto the player. Dropping the old
    ///    handle closes the old socket task's channels, which terminates the
    ///    old connection (and frees its per-IP permit).
    /// 3. Revives the session: clears the logout/disconnect flags so the
    ///    logout phase stops counting the player down.
    /// 4. Forgets the per-observer view state (tracked players/NPCs and
    ///    appearance hashes), decrementing NPC observer counts exactly like
    ///    [`remove_player`](Engine::remove_player) so the re-adds during the
    ///    next info phase keep the counts balanced. The client rebuilds its
    ///    own entity lists from each info packet, so a clean server-side
    ///    slate is self-healing -- stale entities vanish, missed ones appear.
    /// 5. Calls [`EnginePlayer::on_reconnect`] to re-send all volatile client
    ///    state (varps, map rebuild, modal close, tabs, invs, stats, energy,
    ///    anims) and flag an absolute local-player re-place.
    /// 6. Re-announces the session to ether (`PlayerResync` is idempotent and
    ///    re-sends friend/ignore lists), covering an ether restart during the
    ///    disconnect gap.
    ///
    /// The `Login` script trigger is intentionally not re-run -- the session
    /// never ended.
    ///
    /// # Arguments
    ///
    /// * `pid` -- The live player slot being reclaimed.
    /// * `request` -- The validated reconnect login carrying the new handle.
    ///
    /// # Side Effects
    ///
    /// * Sends the reconnect response and full state refresh to the client.
    /// * Drops the previous client connection.
    /// * Decrements NPC observer counts for the player's old view set.
    /// * Sends [`EtherOutbound::PlayerResync`] to the ether sidecar.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`try_complete_login`](Engine::try_complete_login)
    /// **Calls:** `EnginePlayer::on_reconnect`
    pub(crate) fn adopt_session(&mut self, pid: u16, mut request: LoginRequest) {
        let clock = self.clock;

        let Some(active) = self
            .player_list
            .players
            .get_mut(pid as usize)
            .and_then(|slot| slot.as_mut())
        else {
            // find_pid_by_user37 returned this pid in the same phase, so the
            // slot cannot be empty; guard anyway rather than panic.
            error!("adopt_session: pid {pid} vanished before adoption");
            request
                .handle
                .send(vec![LoginResponse::CouldNotComplete as u8]);
            return;
        };

        #[cfg(before_647)]
        let reply = vec![LoginResponse::Reconnect as u8];
        #[cfg(since_647)]
        let reply = {
            let last = active.player.pathing.last_coord;
            reconnect_reply(
                pid,
                last.x(),
                last.z(),
                last.y(),
                self.player_renderer.regions(),
            )
        };
        active.watch.reconnect(clock, &reply);
        request.handle.send(reply);

        // Swapping the handle drops the old one: its channels close and the
        // old socket task (dead or alive) shuts down. Buffered packets queued
        // earlier this tick survive on `active.buffered` and flush through
        // the new handle's ISAAC cipher at the output phase.
        *active.handle = request.handle;
        active.player.low_memory = request.low_memory;
        active.remote_ip = Some(request.remote_addr.ip());
        // Taps from before the reconnect still look like jumps.
        active.watch.anticheat.touch |= request.device.touch;
        active.watch.anticheat.automated |= crate::watch::automated(&request.device);

        active.watch.anticheat.resumed();
        active.player.logout_requested = false;
        active.player.logout_idle_requested = false;
        active.player.logout_sent = false;
        active.player.disconnected_at = None;
        active.player.last_response = clock;
        active.player.last_active = clock;

        // Forget the per-observer view state (reference: cleanupPlayerBuildArea).
        // NPC observer counts are decremented further down, outside this
        // borrow; the info phase re-increments them as NPCs are re-tracked.
        let nids: Vec<u16> = active.player.build_area.npcs.iter().to_vec();
        // Clicks made before the reconnect still count as on what it showed.
        for &nid in &nids {
            active.watch.anticheat.lost(Target::Npc(nid), clock);
        }
        for pid in active.player.build_area.players.iter().to_vec() {
            active.watch.anticheat.lost(Target::Player(pid), clock);
        }
        #[cfg(before_647)]
        active.player.build_area.players.clear();
        #[cfg(since_647)]
        active.player.build_area.init_nsn_state(pid);
        active.player.build_area.npcs.clear();
        active.player.build_area.appearances.fill(0);

        if let Err(e) = active.on_reconnect() {
            error!("error during reconnect for player {pid}: {e}");
        }

        let user37 = active.uid().username37();
        let private_mode = active.player.private_chat();
        let is_member = active.player.is_member;
        let username = active.player.uid.username();

        for nid in nids {
            if let Some(npc) = self
                .npc_list
                .npcs
                .get_mut(nid as usize)
                .and_then(|n| n.as_mut())
            {
                npc.npc.observers = npc.npc.observers.saturating_sub(1);
            }
        }

        if let Some(tx) = &self.ether_tx {
            let _ = tx.send(EtherOutbound::PlayerResync {
                user37,
                pid,
                private_mode,
                max_friends: max_friends_cap(is_member),
                ip: Some(request.remote_addr.ip()),
            });
        }

        info!("Player '{}' reconnected (pid={})", username, pid);
    }

    /// Removes a pending login, sends the given response to its client, and
    /// releases everything the attempt holds across the cluster.
    ///
    /// When a login check is granted, the ether sidecar registers account and
    /// IP reservations (10-second TTL) that are normally consumed when the
    /// session starts. A pending that dies after being granted -- failed
    /// password, timeout, world full, database loss -- would otherwise leave
    /// those reservations behind, making an immediate retry fail with
    /// "already logged in" until the TTL expires. The abort is only sent when
    /// `ether_allowed` is set: before that, this attempt owns no reservations
    /// (and an unconditional abort could release a *concurrent* attempt's).
    ///
    /// Likewise, once the attempt has asked the database to claim the
    /// player's session, the claim is released: the database handles
    /// requests in order, so the release lands after the claim whether or
    /// not the claim's reply has arrived yet.
    ///
    /// # Arguments
    ///
    /// * `idx` -- Index into `self.pending_logins` (removed via `swap_remove`).
    /// * `response` -- The login response byte to send to the client.
    ///
    /// # Call Stack
    ///
    /// **Called by:** login timeout sweep, auth-failure handler,
    /// db-disconnect drain, ether-reconnect sweep, `try_complete_login`.
    pub(crate) fn reject_pending_login(&mut self, idx: usize, response: LoginResponse) {
        self.reject_pending_login_with(idx, vec![response as u8]);
    }

    /// [`reject_pending_login`](Engine::reject_pending_login) for a response
    /// that carries a payload after its code byte.
    pub(crate) fn reject_pending_login_with(&mut self, idx: usize, reply: Vec<u8>) {
        let mut pending = self.pending_logins.swap_remove(idx);
        pending.request.handle.send(reply);
        if pending.ether_allowed
            && let Some(tx) = &self.ether_tx
        {
            let _ = tx.send(EtherOutbound::LoginAbort {
                user37: pending.user37,
                ip: Some(pending.request.remote_addr.ip()),
            });
        }
        if pending.claim_sent {
            self.release_claim(pending.user37);
        }
    }

    /// Releases this world's database claim on a player who never entered
    /// it. Never for a player who is online here or still being logged out
    /// of here: their claim is live, and their logout save ends it.
    pub fn release_claim(&self, user37: u64) {
        if self.find_pid_by_user37(user37).is_some()
            || self.logout_saves.contains_key(&user37)
            || self.offline_edits.contains(&user37)
        {
            return;
        }
        if let Some(tx) = &self.db_tx {
            let _ = tx.send(DbRequest::Release { user37 });
        }
    }

    /// Attempts to complete a pending login if all async prerequisites are met.
    ///
    /// A login requires both `ether_allowed` and `auth` to be set. A reconnect
    /// whose player is still in this world goes to [`Engine::adopt_session`].
    /// Any other login is rejected with `ServerUpdating` while a shutdown is
    /// due within 50 ticks (or, in maintenance mode, for a normal player), and
    /// with `WorldFull` when the world holds 2000 or more players or has no
    /// free pid. Then the player's session is claimed and their profile loaded
    /// with one `DbRequest::Load` (sent once). Once the profile is in, a free
    /// world rejects a profile standing in a members area with `MembersArea`;
    /// otherwise the pending login is removed from the list and forwarded to
    /// [`Engine::accept_login`] with the next free pid.
    ///
    /// # Arguments
    ///
    /// * `idx` -- Index into `self.pending_logins`.
    ///
    /// # Side Effects
    ///
    /// - May send a `DbRequest::Load` to claim and fetch the player profile.
    /// - May reject the login through `reject_pending_login`.
    /// - On rev 647, moves a profile saved in region space to its saved exit
    ///   (or home when no player can stand there), and one in an unallocated
    ///   zone to its home.
    /// - On completion, calls `accept_login` (or `adopt_session`) and removes
    ///   the entry from `pending_logins` via `swap_remove`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ether` (a granted login check), `saves` (the auth and
    /// load replies).
    /// **Calls:** `adopt_session`, `reject_pending_login`, `accept_login`.
    pub(crate) fn try_complete_login(&mut self, idx: usize) {
        let pending = &self.pending_logins[idx];
        if !pending.ether_allowed {
            return;
        }
        let Some(staff_mod_level) = pending.auth else {
            return;
        };

        if pending.request.reconnect
            && let Some(pid) = self.find_pid_by_user37(pending.user37)
        {
            let pending = self.pending_logins.swap_remove(idx);
            self.adopt_session(pid, pending.request);
            return;
        }

        if self.is_shutdown_soon()
            || (self.maintenance_mode && staff_mod_level == StaffModLevel::Normal)
        {
            self.reject_pending_login(idx, LoginResponse::ServerUpdating);
            return;
        }

        if self.player_list.count() >= 2000 {
            self.reject_pending_login(idx, LoginResponse::WorldFull);
            return;
        }

        let Some(pid) = self.player_list.next_pid() else {
            self.reject_pending_login(idx, LoginResponse::WorldFull);
            return;
        };

        if pending.profile.is_none() {
            if !pending.claim_sent
                && let Some(tx) = &self.db_tx
            {
                let _ = tx.send(DbRequest::Load {
                    login: pending.id,
                    user37: pending.user37,
                });
                self.pending_logins[idx].claim_sent = true;
            }
            return;
        }

        // A player saved in region space comes back at the exit it saved,
        // before any packet goes to it.
        #[cfg(since_647)]
        if let Some(Some(profile)) = self.pending_logins[idx].profile.as_mut() {
            place_returning(profile);
        }

        // A free world cannot place a player saved inside a members area.
        if !self.members
            && let Some(Some(profile)) = &self.pending_logins[idx].profile
            && !rsmod::is_zone_flagged(profile.x, profile.z, profile.y, ZoneFlag::Free as u8)
        {
            self.reject_pending_login(idx, LoginResponse::MembersArea);
            return;
        }

        #[cfg(since_647)]
        if let Some(Some(profile)) = self.pending_logins[idx].profile.as_mut()
            && !rsmod::is_zone_allocated(profile.x, profile.z, profile.y)
        {
            (profile.x, profile.z, profile.y) = home_for(profile.x, profile.z);
        }

        let pending = self.pending_logins.swap_remove(idx);
        let profile = pending.profile.unwrap();
        self.accept_login(
            pending.request,
            profile,
            staff_mod_level,
            pending.muted_until,
            pending.unread_messages,
            pid,
            &pending.played,
        );
    }
}
