use crate::clients::client_db::{AuthResult, DbResponse, LoadResult, SaveOutcome};
use crate::engine::Engine;
use crate::player_save::delete_save_file;
use rs_entity::StaffModLevel;
use rs_protocol::LoginResponse;
use rustc_hash::FxHashMap;
#[cfg(since_254)]
use std::time::Duration;
use std::time::Instant;
use tracing::{info, warn};

/// How long a player who logged out of one world must wait before logging
/// into another, so the old world's logout save settles before the new one
/// loads the profile (Lost City's `NODE_HOP_TIME`).
#[cfg(since_254)]
const PROFILE_TRANSFER_DELAY: Duration = Duration::from_secs(45);

/// The [`LoginResponse::ProfileTransfer`] countdown in whole seconds, or
/// `None` if the login may go ahead. Jagex moderators and up are exempt.
#[cfg(since_254)]
fn profile_transfer_wait(staff_mod_level: u8, left_other_world: Option<Duration>) -> Option<u8> {
    if staff_mod_level >= StaffModLevel::JagexModerator as u8 {
        return None;
    }
    let remaining = PROFILE_TRANSFER_DELAY.checked_sub(left_other_world?)?;
    let secs = u8::try_from(remaining.as_millis().div_ceil(1000)).unwrap_or(u8::MAX);
    (secs > 0).then_some(secs)
}

impl Engine {
    /// Processes the saves phase of the engine tick cycle.
    ///
    /// Drains inbound responses from the database service and handles each
    /// message:
    ///
    /// * `DbReady` -- marks the database as available, enabling logins, and
    ///   resends any logout save that failed while it was away.
    /// * `DbDisconnected` -- marks the database as unavailable, rejects
    ///   all pending logins with [`LoginResponse::LoginServerOffline`].
    /// * `AuthResponse` -- completes authentication for a pending login.
    ///   On success, records the player's staff level and attempts
    ///   completion. A wrong password or unknown name rejects with
    ///   [`LoginResponse::InvalidCredentials`] and counts toward the IP's
    ///   [`LoginResponse::TooManyAttempts`] lockout, which the database also
    ///   enforces across all worlds; a ban rejects with
    ///   [`LoginResponse::AccountDisabled`]. From 254, a login within
    ///   `PROFILE_TRANSFER_DELAY` of leaving another world rejects with
    ///   `LoginResponse::ProfileTransfer` and the seconds left, which the
    ///   client counts down before retrying on its own.
    /// * `LoadResponse` -- the session claim's outcome: attaches the loaded
    ///   profile and attempts completion, or rejects with
    ///   [`LoginResponse::AlreadyLoggedIn`] when another world owns the
    ///   session.
    /// * `SaveAck` -- on success, deletes the local backup save file. On
    ///   failure, keeps the local file as a fallback. For a logout save,
    ///   ends the ether session -- unless the database failed, in which case
    ///   the session stays held and the save is resent on `DbReady`. An
    ///   autosave refused because another world owns the session removes the
    ///   player unsaved.
    /// * `SessionsLost` -- removes, unsaved, online players whose sessions
    ///   another world took over.
    /// * `Lease` -- renews this world's lease (see `Engine::fence`).
    /// * `Muted` -- a moderator's mute took: mutes the player if online here.
    /// * `Mutes` -- syncs online players' mutes with the database, so mutes
    ///   and unmutes made elsewhere reach them.
    ///
    /// # Side Effects
    ///
    /// * Toggles `self.db_ready`.
    /// * Completes or rejects entries in `self.pending_logins`.
    /// * Deletes local save files on successful DB persistence.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `try_complete_login`, `delete_save_file`
    pub(crate) fn saves(&mut self) {
        while let Ok(msg) = self.db_rx.try_recv() {
            match msg {
                DbResponse::DbReady => {
                    self.db_ready = true;
                    info!("DB ready - logins enabled");
                    let failed: Vec<u64> = self
                        .logout_saves
                        .iter()
                        .filter(|(_, save)| save.failed)
                        .map(|(&user37, _)| user37)
                        .collect();
                    for user37 in failed {
                        self.send_logout_save(user37);
                    }
                }
                DbResponse::DbDisconnected => {
                    self.db_ready = false;
                    warn!("DB disconnected - logins disabled");
                    while !self.pending_logins.is_empty() {
                        let idx = self.pending_logins.len() - 1;
                        self.reject_pending_login(idx, LoginResponse::LoginServerOffline);
                    }
                }
                DbResponse::AuthResponse { user37, result } => {
                    if let Some(idx) = self.pending_logins.iter().position(|p| p.user37 == user37) {
                        match result {
                            AuthResult::Accepted {
                                staff_mod_level,
                                muted_for,
                                left_other_world,
                            } => {
                                #[cfg(since_254)]
                                if let Some(secs) =
                                    profile_transfer_wait(staff_mod_level, left_other_world)
                                {
                                    self.reject_pending_login_with(
                                        idx,
                                        vec![LoginResponse::ProfileTransfer as u8, secs],
                                    );
                                    continue;
                                }
                                #[cfg(before_254)]
                                let _ = left_other_world;
                                self.pending_logins[idx].auth =
                                    Some(StaffModLevel::from_u8(staff_mod_level));
                                self.pending_logins[idx].muted_until =
                                    muted_for.map(|d| Instant::now() + d);
                                self.try_complete_login(idx);
                            }
                            AuthResult::InvalidCredentials => {
                                let ip = self.pending_logins[idx].request.remote_addr.ip();
                                self.record_login_failure(ip);
                                self.reject_pending_login(idx, LoginResponse::InvalidCredentials);
                            }
                            AuthResult::Banned => {
                                self.reject_pending_login(idx, LoginResponse::AccountDisabled);
                            }
                            AuthResult::TooManyAttempts => {
                                self.reject_pending_login(idx, LoginResponse::TooManyAttempts);
                            }
                        }
                    }
                }
                DbResponse::LoadResponse { user37, result } => {
                    let idx = self.pending_logins.iter().position(|p| p.user37 == user37);
                    match (idx, result) {
                        (Some(idx), LoadResult::Loaded(profile)) => {
                            self.pending_logins[idx].profile = Some(profile.map(|p| *p));
                            self.try_complete_login(idx);
                        }
                        (Some(idx), LoadResult::Busy) => {
                            self.reject_pending_login(idx, LoginResponse::AlreadyLoggedIn);
                        }
                        (Some(idx), LoadResult::Unregistered) => {
                            self.reject_pending_login(idx, LoginResponse::CouldNotComplete);
                        }
                        // The login died before its claim came back. Its
                        // rejection already queued a release behind the
                        // claim; this one is a backstop.
                        (None, LoadResult::Loaded(_)) => self.release_claim(user37),
                        (None, _) => {}
                    }
                }
                DbResponse::SaveAck {
                    user37,
                    username,
                    outcome,
                    logout,
                } => {
                    match outcome {
                        SaveOutcome::Stored => delete_save_file(&username),
                        SaveOutcome::Unregistered => {
                            warn!("No registered player '{}', keeping local file", username)
                        }
                        SaveOutcome::NotOwner => {}
                        SaveOutcome::Failed => {
                            warn!("DB save failed for '{}', keeping local file", username)
                        }
                    }
                    if logout {
                        if outcome == SaveOutcome::Failed {
                            // The database copy is stale until this save
                            // lands: keep the session held and resend it.
                            if let Some(save) = self.logout_saves.get_mut(&user37) {
                                save.failed = true;
                            }
                        } else {
                            self.logout_saves.remove(&user37);
                            self.release_logout(user37);
                        }
                    } else if outcome == SaveOutcome::NotOwner {
                        self.drop_unowned_player(user37);
                    }
                }
                DbResponse::SessionsLost { users37 } => {
                    for user37 in users37 {
                        self.drop_unowned_player(user37);
                    }
                }
                DbResponse::Lease { at } => {
                    self.db_lease = Some(self.db_lease.map_or(at, |lease| lease.max(at)));
                }
                DbResponse::Muted { user37, remaining } => {
                    if let Some(pid) = self.find_pid_by_user37(user37)
                        && let Some(active) = self.get_player_mut(pid)
                    {
                        active.muted_until = Some(Instant::now() + remaining);
                    }
                }
                DbResponse::Mutes { mutes } => {
                    let now = Instant::now();
                    let mutes: FxHashMap<u64, _> = mutes.into_iter().collect();
                    for &pid in self.player_list.processing.iter() {
                        if let Some(active) = self.player_list.players[pid as usize].as_mut()
                            && let Some(muted_for) = mutes.get(&active.player.uid.username37())
                        {
                            active.muted_until = muted_for.map(|d| now + d);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(all(test, since_254))]
mod tests {
    use super::*;

    #[test]
    fn profile_transfer_countdown() {
        let ago = |ms| Some(Duration::from_millis(ms));
        // Never left another world, or the delay has passed.
        assert_eq!(profile_transfer_wait(0, None), None);
        assert_eq!(profile_transfer_wait(0, ago(45_000)), None);
        assert_eq!(profile_transfer_wait(0, ago(90_000)), None);
        // Partial seconds round up so the client's retry lands after the delay.
        assert_eq!(profile_transfer_wait(0, ago(0)), Some(45));
        assert_eq!(profile_transfer_wait(0, ago(44_001)), Some(1));
        assert_eq!(profile_transfer_wait(1, ago(10_500)), Some(35));
        // Jagex moderators and developers skip it.
        assert_eq!(profile_transfer_wait(2, ago(0)), None);
        assert_eq!(profile_transfer_wait(3, ago(0)), None);
    }
}
