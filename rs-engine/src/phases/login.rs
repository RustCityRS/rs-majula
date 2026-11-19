use crate::clients::client_db::DbRequest;
use crate::clients::client_ether::EtherOutbound;
use crate::core::{Engine, LoginFailures, PendingLogin};
use rs_protocol::LoginResponse;
use rs_util::base37::to_userhash;
use rs_util::net::limit_address;
use std::net::IpAddr;
use tracing::warn;

/// Number of ticks a pending login may remain unresolved before being
/// discarded: with [`LoginResponse::Duplicate`] when, from 647, it is still
/// waiting for a lobby to hand the player over, else with
/// [`LoginResponse::BadSave`].
const LOGIN_TIMEOUT_TICKS: u32 = 10;

/// Maximum playing sessions (distinct accounts) allowed per IP across all
/// nodes, enforced cluster-wide by the ether sidecar during the login check.
/// Distinct from the transport socket cap in rs-server, which only limits
/// concurrent TCP/WS connections (and is sized at twice this so a reconnect's
/// old and new socket can briefly coexist per playing session).
#[cfg(debug_assertions)]
pub const MAX_PLAYING_PER_IP: u8 = 2;
#[cfg(not(debug_assertions))]
pub const MAX_PLAYING_PER_IP: u8 = 1;

/// Wrong-password attempts an IP may make before further logins are refused
/// with [`LoginResponse::TooManyAttempts`]. Enforced twice: per world here,
/// and across all worlds by the database.
pub const MAX_LOGIN_FAILURES: u8 = 5;

/// The failure window: 1 minute, as the client's message promises.
pub const LOGIN_FAILURE_WINDOW_SECS: i32 = 60;

/// [`LOGIN_FAILURE_WINDOW_SECS`] in 600ms ticks. This world forgets an IP's
/// failures once its most recent one is this old.
const LOGIN_FAILURE_WINDOW_TICKS: u32 = LOGIN_FAILURE_WINDOW_SECS as u32 * 1000 / 600;

impl Engine {
    /// Processes the login phase of the engine tick cycle.
    ///
    /// Drains the `new_player_rx` channel for incoming login requests and,
    /// for each request:
    ///
    /// 1. Rejects immediately if the database or the ether is not ready or
    ///    this world's lease has run out ([`LoginResponse::LoginServerOffline`]).
    /// 2. Rejects if the IP has made [`MAX_LOGIN_FAILURES`] wrong-password
    ///    attempts within the failure window
    ///    ([`LoginResponse::TooManyAttempts`]).
    /// 3. From 647, rejects a reconnect whose player is not on this world
    ///    ([`LoginResponse::BadSession`]). Rejects if the player is already
    ///    logged in on this world, is still being logged out of it (their
    ///    final save hasn't landed), or already has a login in flight
    ///    ([`LoginResponse::Duplicate`]).
    /// 4. Sends a database authentication request, then parks the request
    ///    in `pending_logins`. Before 647 the ether cross-world login check
    ///    goes out here too; from 647 it goes out once authentication is
    ///    accepted (see `Engine::saves`).
    ///
    /// After processing new requests, any pending login that has exceeded
    /// [`LOGIN_TIMEOUT_TICKS`] is removed and the client is notified with
    /// [`LoginResponse::Duplicate`] if, from 647, it is waiting for a lobby
    /// to hand the player over, else with [`LoginResponse::BadSave`]. From
    /// 647, a login that waits for a lobby resends its ether login check
    /// every tick until the ether allows it.
    ///
    /// # Side Effects
    ///
    /// * Sends [`DbRequest::Authenticate`] for each valid login attempt, and
    ///   before 647 [`EtherOutbound::LoginCheck`] as well.
    /// * From 647, resends [`EtherOutbound::LoginCheck`] for each pending
    ///   login that waits for a lobby.
    /// * Appends to `self.pending_logins`.
    /// * Removes timed-out entries from `self.pending_logins`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `EtherOutbound::LoginCheck`, `DbRequest::Authenticate`,
    ///   `check_login`, `reject_pending_login`
    pub(crate) fn logins(&mut self) {
        while let Ok(mut request) = self.new_player_rx.try_recv() {
            let user37 = to_userhash(&request.username);

            if !self.db_ready || !self.ether_ready || self.lease_expired() {
                request
                    .handle
                    .send(vec![LoginResponse::LoginServerOffline as u8]);
                continue;
            }

            let ip = request.remote_addr.ip();
            if self.login_attempts_exceeded(ip) {
                request
                    .handle
                    .send(vec![LoginResponse::TooManyAttempts as u8]);
                continue;
            }

            let online_here = self.find_pid_by_user37(user37).is_some();
            #[cfg(since_647)]
            if request.reconnect && !online_here {
                request.handle.send(vec![LoginResponse::BadSession as u8]);
                continue;
            }
            let busy = (online_here && !request.reconnect)
                || self.logout_saves.contains_key(&user37)
                || self.offline_edits.contains(&user37)
                || self.pending_logins.iter().any(|p| p.user37 == user37);
            if busy {
                request.handle.send(vec![LoginResponse::Duplicate as u8]);
                continue;
            }

            if let Some(ether) = &self.ether_tx {
                #[cfg(since_647)]
                let _ = ether;
                let id = self.next_login_id;
                self.next_login_id = id.wrapping_add(1);
                #[cfg(before_647)]
                {
                    let _ = ether.send(EtherOutbound::LoginCheck {
                        user37,
                        max_per_ip: MAX_PLAYING_PER_IP,
                        ip: Some(ip),
                    });
                }
                if let Some(db_tx) = &self.db_tx {
                    let _ = db_tx.send(DbRequest::Authenticate {
                        login: id,
                        user37,
                        password: request.password.clone(),
                        ip,
                        uid: request.uid,
                        device: Box::new(request.device.clone()),
                    });
                }
                self.pending_logins.push(PendingLogin {
                    id,
                    user37,
                    request,
                    clock: self.clock,
                    ether_allowed: false,
                    #[cfg(since_647)]
                    retry_ether: false,
                    auth: None,
                    muted_until: None,
                    unread_messages: 0,
                    member: false,
                    #[cfg(since_647)]
                    jcoins: 0,
                    #[cfg(since_530)]
                    last_login_ip: None,
                    claim_sent: false,
                    profile: None,
                    played: Vec::new(),
                });
            } else {
                request
                    .handle
                    .send(vec![LoginResponse::LoginServerOffline as u8]);
            }
        }

        let clock = self.clock;
        let mut i = self.pending_logins.len();
        while i > 0 {
            i -= 1;
            if clock - self.pending_logins[i].clock >= LOGIN_TIMEOUT_TICKS {
                warn!(
                    "Login timed out for '{}'",
                    self.pending_logins[i].request.username
                );
                #[cfg(since_647)]
                let in_lobby = self.pending_logins[i].retry_ether;
                #[cfg(before_647)]
                let in_lobby = false;
                let response = if in_lobby {
                    LoginResponse::Duplicate
                } else {
                    LoginResponse::BadSave
                };
                self.reject_pending_login(i, response);
                continue;
            }
            #[cfg(since_647)]
            if self.pending_logins[i].retry_ether {
                self.check_login(i);
            }
        }

        if clock.is_multiple_of(LOGIN_FAILURE_WINDOW_TICKS) {
            self.login_failures
                .retain(|_, f| clock - f.last < LOGIN_FAILURE_WINDOW_TICKS);
        }
    }

    #[cfg(since_647)]
    pub(crate) fn check_login(&self, idx: usize) {
        let pending = &self.pending_logins[idx];
        if let Some(tx) = &self.ether_tx {
            let _ = tx.send(EtherOutbound::LoginCheck {
                user37: pending.user37,
                max_per_ip: MAX_PLAYING_PER_IP,
                ip: Some(pending.request.remote_addr.ip()),
            });
        }
    }

    /// Counts a wrong-password attempt against `ip` (by [`limit_address`]),
    /// restarting the count if the previous failure has aged out of the window.
    pub(crate) fn record_login_failure(&mut self, ip: IpAddr) {
        let clock = self.clock;
        let failures = self
            .login_failures
            .entry(limit_address(ip))
            .or_insert(LoginFailures {
                count: 0,
                last: clock,
            });
        if clock - failures.last >= LOGIN_FAILURE_WINDOW_TICKS {
            failures.count = 0;
        }
        failures.count = failures.count.saturating_add(1);
        failures.last = clock;
    }

    /// Whether `ip` is locked out by recent wrong-password attempts.
    fn login_attempts_exceeded(&self, ip: IpAddr) -> bool {
        self.login_failures
            .get(&limit_address(ip))
            .is_some_and(|f| {
                f.count >= MAX_LOGIN_FAILURES && self.clock - f.last < LOGIN_FAILURE_WINDOW_TICKS
            })
    }
}
