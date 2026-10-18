use crate::clients::client_db::DbRequest;
use crate::clients::client_ether::EtherOutbound;
use crate::engine::{Engine, PendingLogin};
use rs_protocol::LoginResponse;
use rs_util::base37::to_userhash;
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

impl Engine {
    /// Processes the login phase of the engine tick cycle.
    ///
    /// Drains the `new_player_rx` channel for incoming login requests and,
    /// for each request:
    ///
    /// 1. Rejects immediately if the database is not ready
    ///    ([`LoginResponse::LoginServerOffline`]).
    /// 2. Rejects if the player is already logged in on this world
    ///    ([`LoginResponse::AlreadyLoggedIn`]).
    /// 3. Sends an ether cross-world login-check and a database
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
    }
}
