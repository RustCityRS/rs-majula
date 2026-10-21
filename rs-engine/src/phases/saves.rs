use crate::active_player::ActivePlayer;
use crate::clients::client_db::{AuthResult, DbResponse, LoadResult, ModOutcome, SaveOutcome};
use crate::engine::Engine;
use crate::player_save::delete_save_file;
use rs_db::ModAction;
use rs_entity::StaffModLevel;
use rs_protocol::LoginResponse;
use rs_util::base37::{to_raw_username, to_screen_name};
use rustc_hash::FxHashMap;
use std::time::Duration;
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
    ///   client counts down before retrying on its own. During a lockdown
    ///   (see [`crate::controls`]) an account too new is rejected with
    ///   [`LoginResponse::CouldNotComplete`].
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
    /// * `Sanctions` -- syncs online players' mutes with the database and
    ///   logs out banned players, so what was done elsewhere reaches them.
    /// * `Moderated` -- a staff command's outcome: applies it to the target if
    ///   online here, and tells the moderator (see `Engine::moderated`).
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
                                account_age,
                            } => {
                                let staff = StaffModLevel::from_u8(staff_mod_level);
                                if !self.controls.admits(staff, account_age) {
                                    self.reject_pending_login(idx, LoginResponse::CouldNotComplete);
                                    continue;
                                }
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
                                self.pending_logins[idx].auth = Some(staff);
                                self.pending_logins[idx].muted_until =
                                    muted_for.map(ActivePlayer::mute_deadline);
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
                        active.muted_until = Some(ActivePlayer::mute_deadline(remaining));
                    }
                }
                DbResponse::Sanctions { players } => {
                    let players: FxHashMap<u64, _> = players
                        .into_iter()
                        .map(|(user37, muted_for, banned)| (user37, (muted_for, banned)))
                        .collect();
                    for &pid in self.player_list.processing.iter() {
                        if let Some(active) = self.player_list.players[pid as usize].as_mut()
                            && let Some(&(muted_for, banned)) =
                                players.get(&active.player.uid.username37())
                        {
                            active.muted_until = muted_for.map(ActivePlayer::mute_deadline);
                            if banned && !active.player.logout_sent {
                                active.logout();
                            }
                        }
                    }
                }
                DbResponse::Moderated {
                    moderator37,
                    target37,
                    action,
                    outcome,
                } => self.moderated(moderator37, target37, action, outcome),
            }
        }
    }
}

impl Engine {
    /// Finishes a staff command once the database has answered: applies a
    /// mute, unmute, ban or kick to the target if they are online here, and
    /// tells the moderator how it went.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::saves`
    fn moderated(
        &mut self,
        moderator37: u64,
        target37: u64,
        action: ModAction,
        outcome: ModOutcome,
    ) {
        let name = to_screen_name(&to_raw_username(target37));
        let message = match outcome {
            ModOutcome::Done => match action {
                ModAction::Mute { hours } => format!("Muted {name} for {hours} hours."),
                ModAction::Unmute => format!("Unmuted {name}."),
                ModAction::Ban { hours } => format!("Banned {name} for {hours} hours."),
                ModAction::Unban => format!("Unbanned {name}."),
                ModAction::Kick => format!("Kicked {name}."),
            },
            ModOutcome::Refused => {
                format!("{name} isn't a registered player, or is staff of your level or above.")
            }
            ModOutcome::Offline => "The database is offline, so nothing was changed.".to_owned(),
        };

        if outcome == ModOutcome::Done
            && let Some(pid) = self.find_pid_by_user37(target37)
            && let Some(target) = self.get_player_mut(pid)
        {
            match action {
                ModAction::Mute { hours } => {
                    target.muted_until = Some(ActivePlayer::mute_deadline(Duration::from_secs(
                        hours as u64 * 3600,
                    )));
                }
                ModAction::Unmute => target.muted_until = None,
                ModAction::Ban { .. } | ModAction::Kick if !target.player.logout_sent => {
                    target.logout();
                }
                _ => {}
            }
        }

        if let Some(pid) = self.find_pid_by_user37(moderator37)
            && let Some(moderator) = self.get_player_mut(pid)
        {
            moderator.message_game(&message);
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
        // Jagex moderators and maintainers skip it.
        assert_eq!(profile_transfer_wait(2, ago(0)), None);
        assert_eq!(profile_transfer_wait(3, ago(0)), None);
    }
}
