use crate::users::name_key;
use rs_admin_api::hex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A signed-in session ends after this long without a request...
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
/// ...and this long after sign-in regardless.
pub const MAX_AGE: Duration = Duration::from_secs(12 * 60 * 60);
/// How long a sign-in may sit between its password and second factor.
pub const PENDING_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Failures counted against an IP or an account over this window...
const FAILURE_WINDOW: Duration = Duration::from_secs(15 * 60);
/// ...beyond which further attempts are refused until some age out.
const MAX_IP_FAILURES: usize = 20;
const MAX_ACCOUNT_FAILURES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Totp,
    Enroll,
    SignedIn,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub id: u64,
    pub user: String,
    pub stage: Stage,
    pub csrf: String,
    pub ip: IpAddr,
    pub user_agent: String,
    pub pending_totp: Option<Vec<u8>>,
    pub created_ms: u64,
    created: Instant,
    last_seen: Instant,
    last_seen_ms: u64,
}

impl Session {
    pub fn last_seen_ms(&self) -> u64 {
        self.last_seen_ms
    }

    fn expired(&self, now: Instant) -> bool {
        match self.stage {
            Stage::SignedIn => now - self.last_seen > IDLE_TIMEOUT || now - self.created > MAX_AGE,
            _ => now - self.created > PENDING_TIMEOUT,
        }
    }
}

/// A new random token, hex encoded.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS has no randomness");
    hex(&bytes)
}

fn digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[derive(Default)]
struct Inner {
    by_token: HashMap<[u8; 32], Session>,
    next_id: u64,
    ip_failures: HashMap<IpAddr, Vec<Instant>>,
    account_failures: HashMap<String, Vec<Instant>>,
    /// The last TOTP step each account used, so no code works twice.
    totp_steps: HashMap<String, u64>,
}

#[derive(Default)]
pub struct Sessions {
    inner: Mutex<Inner>,
}

impl Sessions {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Opens a session for `user` at `stage`. Returns its cookie token.
    pub fn create(&self, user: &str, stage: Stage, ip: IpAddr, user_agent: &str) -> String {
        let token = random_token();
        let mut inner = self.lock();
        inner.next_id += 1;
        let now = Instant::now();
        let session = Session {
            id: inner.next_id,
            user: user.to_owned(),
            stage,
            csrf: random_token(),
            ip,
            user_agent: user_agent.chars().take(300).collect(),
            pending_totp: None,
            created_ms: unix_ms(),
            created: now,
            last_seen: now,
            last_seen_ms: unix_ms(),
        };
        inner.by_token.insert(digest(&token), session);
        token
    }

    /// The live session behind `token`; looking it up counts as activity.
    pub fn get(&self, token: &str) -> Option<Session> {
        let key = digest(token);
        let mut inner = self.lock();
        let now = Instant::now();
        let session = inner.by_token.get_mut(&key)?;
        if session.expired(now) {
            inner.by_token.remove(&key);
            return None;
        }
        session.last_seen = now;
        session.last_seen_ms = unix_ms();
        Some(session.clone())
    }

    /// Moves the session behind `token` on to `stage` under a fresh token
    /// and CSRF token, restarting its clock. Returns the new token.
    pub fn advance(&self, token: &str, stage: Stage) -> Option<String> {
        let mut inner = self.lock();
        let mut session = inner.by_token.remove(&digest(token))?;
        let now = Instant::now();
        session.stage = stage;
        session.pending_totp = None;
        session.csrf = random_token();
        session.created = now;
        session.created_ms = unix_ms();
        session.last_seen = now;
        let token = random_token();
        inner.by_token.insert(digest(&token), session);
        Some(token)
    }

    pub fn set_pending_totp(&self, token: &str, secret: Vec<u8>) {
        if let Some(session) = self.lock().by_token.get_mut(&digest(token)) {
            session.pending_totp = Some(secret);
        }
    }

    pub fn end(&self, token: &str) {
        self.lock().by_token.remove(&digest(token));
    }

    /// Ends `user`'s session `id`. Returns whether there was one.
    pub fn end_by_id(&self, user: &str, id: u64) -> bool {
        let key = name_key(user);
        let mut inner = self.lock();
        let before = inner.by_token.len();
        inner
            .by_token
            .retain(|_, s| !(s.id == id && name_key(&s.user) == key));
        inner.by_token.len() != before
    }

    /// Ends every session of `user` except `keep`. Returns how many.
    pub fn end_all(&self, user: &str, keep: Option<u64>) -> usize {
        let key = name_key(user);
        let mut inner = self.lock();
        let before = inner.by_token.len();
        inner
            .by_token
            .retain(|_, s| name_key(&s.user) != key || Some(s.id) == keep);
        before - inner.by_token.len()
    }

    /// `user`'s signed-in sessions, most recently active first.
    pub fn of(&self, user: &str) -> Vec<Session> {
        let key = name_key(user);
        let now = Instant::now();
        let mut sessions: Vec<Session> = self
            .lock()
            .by_token
            .values()
            .filter(|s| s.stage == Stage::SignedIn && name_key(&s.user) == key && !s.expired(now))
            .cloned()
            .collect();
        sessions.sort_by_key(|s| std::cmp::Reverse(s.last_seen));
        sessions
    }

    /// Whether a sign-in from `ip` as `user` may be tried now.
    pub fn may_attempt(&self, ip: IpAddr, user: &str) -> bool {
        let mut inner = self.lock();
        let now = Instant::now();
        let ip_count = recent(inner.ip_failures.entry(ip).or_default(), now);
        let account_count = recent(
            inner.account_failures.entry(name_key(user)).or_default(),
            now,
        );
        ip_count < MAX_IP_FAILURES && account_count < MAX_ACCOUNT_FAILURES
    }

    pub fn record_failure(&self, ip: IpAddr, user: &str) {
        let mut inner = self.lock();
        let now = Instant::now();
        inner.ip_failures.entry(ip).or_default().push(now);
        inner
            .account_failures
            .entry(name_key(user))
            .or_default()
            .push(now);
    }

    /// Accepts a TOTP code of time step `step` for `user`, unless that step
    /// or a later one was already used.
    pub fn accept_totp_step(&self, user: &str, step: u64) -> bool {
        let mut inner = self.lock();
        let last = inner.totp_steps.entry(name_key(user)).or_insert(0);
        if step <= *last {
            return false;
        }
        *last = step;
        true
    }

    /// Drops expired sessions and aged-out failures.
    pub fn prune(&self) {
        let mut inner = self.lock();
        let now = Instant::now();
        inner.by_token.retain(|_, s| !s.expired(now));
        inner.ip_failures.retain(|_, times| recent(times, now) > 0);
        inner
            .account_failures
            .retain(|_, times| recent(times, now) > 0);
    }
}

/// Drops failures older than the window; returns how many remain.
fn recent(times: &mut Vec<Instant>, now: Instant) -> usize {
    times.retain(|&t| now - t < FAILURE_WINDOW);
    times.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    const IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

    #[test]
    fn advancing_replaces_the_token() {
        let sessions = Sessions::default();
        let pending = sessions.create("Ash", Stage::Totp, IP, "test");
        let csrf = sessions.get(&pending).unwrap().csrf;
        let signed_in = sessions.advance(&pending, Stage::SignedIn).unwrap();
        assert!(sessions.get(&pending).is_none());
        let session = sessions.get(&signed_in).unwrap();
        assert_eq!(session.stage, Stage::SignedIn);
        assert_ne!(session.csrf, csrf);
    }

    #[test]
    fn sessions_end_singly_or_together() {
        let sessions = Sessions::default();
        let a = sessions.create("Ash", Stage::SignedIn, IP, "a");
        let b = sessions.create("ash", Stage::SignedIn, IP, "b");
        let other = sessions.create("Bob", Stage::SignedIn, IP, "c");
        let a_id = sessions.get(&a).unwrap().id;
        assert_eq!(sessions.of("ASH").len(), 2);
        assert!(!sessions.end_by_id("Bob", a_id));
        assert_eq!(sessions.end_all("Ash", Some(a_id)), 1);
        assert!(sessions.get(&b).is_none());
        assert!(sessions.get(&a).is_some());
        assert!(sessions.end_by_id("Ash", a_id));
        assert!(sessions.get(&a).is_none());
        assert!(sessions.get(&other).is_some());
    }

    #[test]
    fn repeated_failures_lock_the_account() {
        let sessions = Sessions::default();
        for _ in 0..MAX_ACCOUNT_FAILURES {
            assert!(sessions.may_attempt(IP, "ash"));
            sessions.record_failure(IP, "Ash");
        }
        assert!(!sessions.may_attempt(IP, "ash"));
        let elsewhere = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        assert!(!sessions.may_attempt(elsewhere, "ash"));
        assert!(sessions.may_attempt(IP, "bob"));
    }

    #[test]
    fn totp_steps_are_single_use() {
        let sessions = Sessions::default();
        assert!(sessions.accept_totp_step("ash", 10));
        assert!(!sessions.accept_totp_step("Ash", 10));
        assert!(!sessions.accept_totp_step("ash", 9));
        assert!(sessions.accept_totp_step("ash", 11));
    }
}
