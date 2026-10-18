use crate::clients::client_db::DbRequest;
use crate::clients::client_ether::EtherOutbound;
use crate::engine::{Engine, LoginFailures, PendingLogin};
use rs_protocol::LoginResponse;
use rs_util::base37::to_userhash;
use std::net::IpAddr;
use tracing::warn;

/// Number of ticks a pending login may remain unresolved before being
/// discarded with a [`LoginResponse::CouldNotComplete`] error.
const LOGIN_TIMEOUT_TICKS: u32 = 10;

/// Maximum playing sessions (distinct accounts) allowed per IP across all
/// nodes, enforced cluster-wide by the ether sidecar during the login check.
/// Distinct from the transport socket cap in rs-server, which only limits
/// concurrent TCP/WS connections (and is sized at twice this so a reconnect's
/// old and new socket can briefly coexist per playing session).
#[cfg(debug_assertions)]
const MAX_PLAYING_PER_IP: u8 = 2;
#[cfg(not(debug_assertions))]
const MAX_PLAYING_PER_IP: u8 = 1;

/// Wrong-password attempts an IP may make before further logins are refused
/// with [`LoginResponse::TooManyAttempts`].
const MAX_LOGIN_FAILURES: u8 = 5;

/// Ticks (1 minute, as the client's message promises) after the most recent
/// failure before an IP's failure count is forgotten.
const LOGIN_FAILURE_WINDOW_TICKS: u32 = 100;

impl Engine {
    /// Processes the login phase of the engine tick cycle.
    ///
    /// Drains the `new_player_rx` channel for incoming login requests and,
    /// for each request:
    ///
    /// 1. Rejects immediately if the database is not ready
    ///    ([`LoginResponse::LoginServerOffline`]).
    /// 2. Rejects if the IP has made [`MAX_LOGIN_FAILURES`] wrong-password
    ///    attempts within [`LOGIN_FAILURE_WINDOW_TICKS`]
    ///    ([`LoginResponse::TooManyAttempts`]).
    /// 3. Rejects if the player is already logged in on this world
    ///    ([`LoginResponse::AlreadyLoggedIn`]).
    /// 4. Sends an ether cross-world login-check and a database
    ///    authentication request, then parks the request in
    ///    `pending_logins`.
    ///
    /// After processing new requests, any pending login that has exceeded
    /// [`LOGIN_TIMEOUT_TICKS`] is removed and the client is notified with
    /// [`LoginResponse::CouldNotComplete`].
    ///
    /// # Side Effects
    ///
    /// * Sends [`EtherOutbound::LoginCheck`] and [`DbRequest::Authenticate`]
    ///   messages for each valid login attempt.
    /// * Appends to `self.pending_logins`.
    /// * Removes timed-out entries from `self.pending_logins`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `EtherOutbound::LoginCheck`, `DbRequest::Authenticate`
    pub(crate) fn logins(&mut self) {
        while let Ok(request) = self.new_player_rx.try_recv() {
            let user37 = to_userhash(&request.username);

            if !self.db_ready || !self.ether_ready {
                let _ = request
                    .handle
                    .outbox
                    .send(vec![LoginResponse::LoginServerOffline as u8]);
                continue;
            }

            if self.login_attempts_exceeded(request.remote_addr.ip()) {
                let _ = request
                    .handle
                    .outbox
                    .send(vec![LoginResponse::TooManyAttempts as u8]);
                continue;
            }

            if self.find_pid_by_user37(user37).is_some() && !request.reconnect {
                let _ = request
                    .handle
                    .outbox
                    .send(vec![LoginResponse::AlreadyLoggedIn as u8]);
                continue;
            }

            if let Some(tx) = &self.ether_tx {
                let _ = tx.send(EtherOutbound::LoginCheck {
                    user37,
                    max_per_ip: MAX_PLAYING_PER_IP,
                    ip: request.remote_addr.ip().to_string(),
                });
                if let Some(db_tx) = &self.db_tx {
                    let _ = db_tx.send(DbRequest::Authenticate {
                        user37,
                        password: request.password.clone(),
                    });
                }
                self.pending_logins.push(PendingLogin {
                    user37,
                    request,
                    clock: self.clock,
                    ether_allowed: false,
                    auth: None,
                    profile: None,
                });
            } else {
                let _ = request
                    .handle
                    .outbox
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
                self.reject_pending_login(i, LoginResponse::CouldNotComplete);
            }
        }

        if clock.is_multiple_of(LOGIN_FAILURE_WINDOW_TICKS) {
            self.login_failures
                .retain(|_, f| clock - f.last < LOGIN_FAILURE_WINDOW_TICKS);
        }
    }

    /// Counts a wrong-password attempt against `ip`, restarting the count
    /// if the previous failure has aged out of the window.
    pub(crate) fn record_login_failure(&mut self, ip: IpAddr) {
        let clock = self.clock;
        let failures = self.login_failures.entry(ip).or_insert(LoginFailures {
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
        self.login_failures.get(&ip).is_some_and(|f| {
            f.count >= MAX_LOGIN_FAILURES && self.clock - f.last < LOGIN_FAILURE_WINDOW_TICKS
        })
    }
}
