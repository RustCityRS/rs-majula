pub mod media;

use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Unix seconds the request was signed at.
pub const TIMESTAMP_HEADER: &str = "x-admin-timestamp";
/// A random, never reused hex string.
pub const NONCE_HEADER: &str = "x-admin-nonce";
/// Hex HMAC-SHA256 of [`signed_message`].
pub const SIGNATURE_HEADER: &str = "x-admin-signature";

/// How far a request's timestamp may be from the world's clock.
pub const MAX_SKEW_SECS: u64 = 30;

/// Paths of the world admin API.
pub mod path {
    pub const STATUS: &str = "/v1/status";
    pub const PLAYERS: &str = "/v1/players";
    pub const REBOOT: &str = "/v1/reboot";
    pub const CANCEL_REBOOT: &str = "/v1/reboot/cancel";
    pub const BROADCAST: &str = "/v1/broadcast";
    pub const KICK: &str = "/v1/kick";
    pub const MODERATE: &str = "/v1/moderate";
    pub const MAINTENANCE: &str = "/v1/maintenance";
    /// Kill switches and the login lockdown.
    pub const CONTROLS: &str = "/v1/controls";
    /// How many scripts kill switch patterns would match, changing nothing.
    pub const CONTROLS_CHECK: &str = "/v1/controls/check";
    /// Lookups in the game database, which any world can answer.
    pub const PLAYER: &str = "/v1/lookup/player";
    pub const PLAYER_SEARCH: &str = "/v1/lookup/player-search";
    pub const MODERATION: &str = "/v1/lookup/moderation";
    pub const SANCTIONS: &str = "/v1/lookup/sanctions";
    pub const CHAT: &str = "/v1/lookup/chat";
    pub const REPORTS: &str = "/v1/lookup/reports";
    /// A player's history across every log, and the accounts linked to them.
    pub const TIMELINE: &str = "/v1/lookup/timeline";
    pub const LINKS: &str = "/v1/lookup/links";
    /// Every account a chain of shared browsers and addresses reaches, and
    /// logins from banned accounts' browsers since their bans.
    pub const LINK_CHAIN: &str = "/v1/lookup/link-chain";
    pub const EVASION: &str = "/v1/lookup/evasion";
    /// Every account with a staff level: `StaffAccountsLookup`, answered
    /// with `Vec<StaffAccountRecord>`.
    pub const STAFF_ACCOUNTS: &str = "/v1/lookup/staff-accounts";
    /// Whether two accounts share a browser or address: `LinkedLookup`,
    /// answered with `LinkBetween`.
    pub const LINKED: &str = "/v1/lookup/linked";
    /// Session recordings: the list, one, and one's replay file.
    pub const REPLAYS: &str = "/v1/lookup/replays";
    pub const REPLAY: &str = "/v1/lookup/replay";
    pub const REPLAY_DATA: &str = "/v1/lookup/replay/data";
    /// How much the recordings take up: `ReplayStorageLookup`, answered
    /// with `ReplayStorage`.
    pub const REPLAY_STORAGE: &str = "/v1/lookup/replay-storage";
    /// A session going on now, to watch live: answered by the world it's on.
    pub const LIVE_REPLAY: &str = "/v1/live/replay";
    /// The players the anti-cheat checks flag most, and accounts' activity
    /// to group bot farms by.
    pub const ANTICHEAT: &str = "/v1/lookup/anticheat";
    pub const ACCOUNTS: &str = "/v1/lookup/accounts";
    /// Player editing: answered by the world the player is on, or by any
    /// world while they're logged out.
    pub const PLAYER_SHEET: &str = "/v1/player/sheet";
    pub const PLAYER_EDIT: &str = "/v1/player/edit";
    pub const OBJ_SEARCH: &str = "/v1/objs";
    /// Player support. A player's session as a world holds it (the world
    /// that owns it, or any while nobody does), and freeing it.
    pub const SESSION: &str = "/v1/player/session";
    pub const UNSTICK: &str = "/v1/player/unstick";
    /// Their earlier saves, one of them as a sheet, and rolling back to one.
    pub const SAVE_POINTS: &str = "/v1/player/history";
    pub const SAVE_POINT: &str = "/v1/player/history/sheet";
    pub const ROLLBACK: &str = "/v1/player/rollback";
    pub const PASSWORD_RESET: &str = "/v1/player/password-reset";
    /// One wealth event, by id.
    pub const WEALTH: &str = "/v1/lookup/wealth";
    /// A player's XP and wealth at each save point, for charting.
    pub const PROGRESS: &str = "/v1/player/progress";
    /// How active a player has been, hour by hour.
    pub const ACTIVITY: &str = "/v1/player/activity";
    /// The economy: where a player's item came from, who holds an item and
    /// how much of it there is, and who hands items to whom.
    pub const ITEM_TRACE: &str = "/v1/economy/trace";
    pub const ITEM_HOLDERS: &str = "/v1/economy/holders";
    pub const ITEM_TOTALS: &str = "/v1/economy/totals";
    pub const TRADE_GRAPH: &str = "/v1/economy/trades";
    pub const TOP_RECEIVERS: &str = "/v1/economy/receivers";
    /// A player's hiscore entries, keeping them off the tables, and every
    /// account kept off.
    pub const HISCORES: &str = "/v1/hiscores/player";
    pub const HISCORES_CHANGE: &str = "/v1/hiscores/change";
    pub const HISCORES_KEPT_OFF: &str = "/v1/hiscores/kept-off";
    /// Recompiles the world's scripts and swaps them in: answered by that
    /// world only.
    pub const RELOAD_SCRIPTS: &str = "/v1/scripts/reload";
    /// The website's admin API (`rs-web`), signed the same way.
    pub const NEWS: &str = "/v1/news";
    pub const NEWS_SAVE: &str = "/v1/news/save";
    pub const NEWS_DELETE: &str = "/v1/news/delete";
    pub const NEWS_RENDER: &str = "/v1/news/render";
    /// `POST` an image or video as the raw body to store it; `GET` it back
    /// under `/<name>`.
    pub const NEWS_MEDIA: &str = "/v1/news/media";
    /// Ban and mute appeals players send from the website: those not yet
    /// taken into a case, noting the case, and the decision.
    pub const APPEALS_WAITING: &str = "/v1/appeals/waiting";
    pub const APPEAL_LINK: &str = "/v1/appeals/link";
    pub const APPEAL_DECIDE: &str = "/v1/appeals/decide";
}

/// The largest image or video a news post or case can embed.
pub const MAX_MEDIA_BYTES: usize = 50 * 1024 * 1024;

/// Checks signed requests for a server: signature, clock skew and replayed
/// nonces.
pub struct Verifier {
    secret: Box<[u8]>,
    nonces: Mutex<HashMap<String, Instant>>,
}

impl Verifier {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().into(),
            nonces: Mutex::default(),
        }
    }

    /// `Ok` if the request is signed with the shared secret, isn't stale
    /// and doesn't replay a nonce; `header` reads a request header.
    pub fn check<'a>(
        &self,
        header: impl Fn(&str) -> Option<&'a str>,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<(), &'static str> {
        let timestamp: u64 = header(TIMESTAMP_HEADER)
            .and_then(|v| v.parse().ok())
            .ok_or("no timestamp")?;
        let nonce = header(NONCE_HEADER)
            .filter(|n| (16..=64).contains(&n.len()))
            .ok_or("no nonce")?;
        let signature = header(SIGNATURE_HEADER).ok_or("no signature")?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        if now.abs_diff(timestamp) > MAX_SKEW_SECS {
            return Err("timestamp out of range");
        }
        if !verify(
            &self.secret,
            timestamp,
            nonce,
            method,
            path,
            body,
            signature,
        ) {
            return Err("bad signature");
        }

        // Only signed requests get this far, so only they can fill the map.
        let mut nonces = self.nonces.lock().unwrap_or_else(|e| e.into_inner());
        let window = Duration::from_secs(MAX_SKEW_SECS * 2);
        nonces.retain(|_, seen| seen.elapsed() < window);
        if nonces.insert(nonce.to_owned(), Instant::now()).is_some() {
            return Err("nonce replayed");
        }
        Ok(())
    }
}

/// What's signed: every part of a request a world acts on.
fn signed_message(timestamp: u64, nonce: &str, method: &str, path: &str, body: &[u8]) -> Vec<u8> {
    let mut message = format!("{timestamp}\n{nonce}\n{method}\n{path}\n").into_bytes();
    message.extend_from_slice(body);
    message
}

fn mac(secret: &[u8]) -> Hmac<Sha256> {
    // HMAC takes a key of any length.
    Hmac::<Sha256>::new_from_slice(secret).expect("HMAC accepts any key length")
}

/// The [`SIGNATURE_HEADER`] value for a request.
pub fn sign(
    secret: &[u8],
    timestamp: u64,
    nonce: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> String {
    let mut mac = mac(secret);
    mac.update(&signed_message(timestamp, nonce, method, path, body));
    hex(&mac.finalize().into_bytes())
}

/// Whether `signature` is the request's, compared in constant time.
pub fn verify(
    secret: &[u8],
    timestamp: u64,
    nonce: &str,
    method: &str,
    path: &str,
    body: &[u8],
    signature: &str,
) -> bool {
    let Some(signature) = unhex(signature) else {
        return false;
    };
    let mut mac = mac(secret);
    mac.update(&signed_message(timestamp, nonce, method, path, body));
    mac.verify_slice(&signature).is_ok()
}

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0xf) as usize] as char);
    }
    out
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// `GET /v1/status`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldStatus {
    pub node_id: u8,
    pub revision: String,
    pub members: bool,
    pub players: u32,
    pub uptime_secs: u64,
    pub reboot_in_secs: Option<u32>,
    pub shutting_down: bool,
    #[serde(default)]
    pub stays_down: bool,
    #[serde(default)]
    pub maintenance: bool,
    #[serde(default)]
    pub reboot_reason: Option<String>,
    #[serde(default)]
    pub health: Option<Health>,
    #[serde(default)]
    pub controls: Option<WorldControls>,
    #[serde(default)]
    pub control_matches: Option<Vec<u32>>,
}

/// The most scripts one kill switch pattern may turn off: more is a pattern
/// too broad to be meant (`*` would stop every player script, logins
/// included).
pub const MAX_SWITCH_SCRIPTS: usize = 300;

/// `POST /v1/controls/check`: answered with how many scripts each pattern
/// matches on that world, in order. Nothing is switched off.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckPatterns {
    pub patterns: Vec<String>,
}

/// A world's kill switches and login lockdown. They last until it restarts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldControls {
    pub switches: Vec<String>,
    pub min_account_age_hours: u32,
}

/// `POST /v1/controls`: replaces a world's controls.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetControls {
    pub actor: Actor,
    pub controls: WorldControls,
}

/// How a [`SetControls`] went: how many scripts each switch matched.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlsSet {
    pub matched: Vec<u32>,
}

/// How the world is running, over roughly the last minute.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Health {
    pub tick_ms: f32,
    pub tick_max_ms: f32,
    pub overruns: u32,
    pub memory_bytes: Option<u64>,
    pub connections: u32,
}

/// One entry of `GET /v1/players`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnlinePlayer {
    pub name: String,
    pub staff_level: u8,
    pub x: u16,
    pub z: u16,
    pub level: u8,
    pub muted: bool,
    #[serde(default)]
    pub replay: Option<String>,
}

/// Who a command is from, for the world's logs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub name: String,
    pub staff_level: u8,
}

/// `POST /v1/reboot`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reboot {
    pub actor: Actor,
    pub seconds: u32,
    #[serde(default)]
    pub reason: Option<String>,
    /// A shutdown: the world exits with `STAY_DOWN_EXIT`, which its service
    /// manager is set not to restart, so it stays off until started again.
    #[serde(default)]
    pub stay_down: bool,
}

/// `POST /v1/maintenance`: closes (or reopens) the world to new logins.
/// Players already on stay, and staff can still log in.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Maintenance {
    pub actor: Actor,
    pub enabled: bool,
}

/// `POST /v1/reboot/cancel`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CancelReboot {
    pub actor: Actor,
}

/// `POST /v1/broadcast`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Broadcast {
    pub actor: Actor,
    pub message: String,
}

/// `POST /v1/kick`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kick {
    pub actor: Actor,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModAction {
    Mute,
    Unmute,
    Ban,
    Unban,
}

/// `POST /v1/moderate`: applied through the game database by whichever
/// world gets it, exactly as the in-game staff command would be.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Moderate {
    pub actor: Actor,
    pub name: String,
    pub action: ModAction,
    pub hours: Option<i32>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModOutcome {
    Done,
    Refused,
    Offline,
    Pending,
}

/// `POST /v1/lookup/player`: answered with `Option<PlayerRecord>`, `None`
/// when no such player is registered.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerLookup {
    pub name: String,
}

/// `POST /v1/lookup/player-search`: the registered players whose names
/// start with `prefix`, that name first then shortest first, answered with
/// `Vec<PlayerRecord>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSearch {
    pub prefix: String,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerRecord {
    pub name: String,
    pub staff_level: u8,
    pub registered_ms: i64,
    pub online_node: Option<u8>,
    pub online_since_ms: Option<i64>,
    pub last_node: Option<u8>,
    pub last_seen_ms: Option<i64>,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
    #[serde(default)]
    pub reset_until_ms: Option<i64>,
}

/// `POST /v1/lookup/moderation`: moderation commands newest first, all or
/// only those on `name`, older than the entry `before` when paging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModerationLookup {
    pub name: Option<String>,
    #[serde(default)]
    pub moderator: Option<String>,
    pub before: Option<i64>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModerationRecord {
    pub id: i64,
    pub moderator: String,
    pub target: String,
    pub action: String,
    pub hours: Option<i32>,
    pub node: u8,
    pub at_ms: i64,
    #[serde(default)]
    pub reason: Option<String>,
}

/// `POST /v1/lookup/timeline`: a player's history, newest first.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineLookup {
    pub name: String,
    pub kinds: Vec<String>,
    pub before: Option<TimelineCursor>,
    pub limit: u32,
}

/// The last row of a timeline page, to read on from.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineCursor {
    pub at_ms: i64,
    pub kind: String,
    pub id: i64,
}

/// One row of a player's timeline. `kind` is one of `login`, `session`,
/// `wealth`, `chat`, `moderation`, `reported`, `report` or `replay`; which
/// other fields are set depends on it (see `rs_db::TimelineRow`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub kind: String,
    pub id: String,
    pub at_ms: i64,
    pub node: Option<u8>,
    pub x: Option<u16>,
    pub z: Option<u16>,
    pub level: Option<u8>,
    pub event_type: Option<i16>,
    pub text: Option<String>,
    pub other: Option<String>,
    pub count: Option<i64>,
    pub value: Option<i64>,
    pub hours: Option<i32>,
    pub detail: Option<String>,
}

/// `POST /v1/lookup/links`: where a player logs in from, and who else does.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinksLookup {
    pub name: String,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Links {
    pub addresses: Vec<LoginAddress>,
    pub accounts: Vec<LinkedAccount>,
    #[serde(default)]
    pub devices: Vec<LoginDeviceSeen>,
    /// Banned accounts that share a browser or address with them.
    #[serde(default)]
    pub banned: Vec<BannedLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BannedLink {
    pub name: String,
    /// `browser` or `address`.
    pub via: String,
    pub banned_ms: Option<i64>,
    /// Whether they logged in on it after the ban.
    pub after_ban: bool,
}

/// `POST /v1/lookup/link-chain`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkChainLookup {
    pub name: String,
    pub depth: u8,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainLink {
    pub name: String,
    pub staff_level: u8,
    pub depth: u8,
    /// Who it was reached from, and how: `browser` or `address`.
    pub via: String,
    pub link: String,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
    pub last_ms: Option<i64>,
}

/// `POST /v1/lookup/evasion`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvasionLookup {
    pub days: u32,
    #[serde(default)]
    pub max_staff_level: Option<u8>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evasion {
    pub name: String,
    pub staff_level: u8,
    /// The banned account whose browser they logged in with, or from whose
    /// address.
    pub banned: String,
    /// `browser` or `address`: how they're linked.
    #[serde(default = "browser")]
    pub via: String,
    pub banned_ms: i64,
    pub logins: i64,
    pub first_ms: i64,
    pub last_ms: i64,
}

/// `POST /v1/lookup/staff-accounts`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StaffAccountsLookup {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StaffAccountRecord {
    pub name: String,
    pub staff_level: u8,
}

/// `POST /v1/lookup/linked`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkedLookup {
    pub name: String,
    pub other: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkBetween {
    /// `browser` or `address`; `None` when they aren't linked.
    pub via: Option<String>,
}

fn browser() -> String {
    "browser".into()
}

/// A device a player logged in with: one browser or desktop client install,
/// as it described itself. Browsers can claim anything.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginDeviceSeen {
    /// The web client rather than the desktop one; `None` for logins from
    /// before devices were recorded.
    pub web: Option<bool>,
    pub user_agent: Option<String>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub touch: bool,
    /// The install's id, as text so the console can label it.
    pub uid: Option<String>,
    pub logins: i64,
    /// How many addresses it was used from, and up to 20 of them.
    pub addresses: i64,
    pub ips: Vec<String>,
    pub first_ms: i64,
    pub last_ms: i64,
    /// How many other accounts logged in from the same install.
    pub others: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginAddress {
    pub ip: String,
    pub logins: i64,
    pub first_ms: i64,
    pub last_ms: i64,
    pub others: i64,
}

/// An account sharing at least one address or client install with the
/// player looked up.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkedAccount {
    pub name: String,
    pub staff_level: u8,
    pub shared: Vec<String>,
    #[serde(default)]
    pub shared_clients: i64,
    pub logins: i64,
    pub last_ms: i64,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
}

/// `POST /v1/lookup/sanctions`: everyone muted or banned now.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SanctionsLookup {
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sanction {
    pub name: String,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
    pub last_action: Option<String>,
    pub moderator: Option<String>,
    pub at_ms: Option<i64>,
}

/// `POST /v1/lookup/chat`: chat newest first, everyone's or only `name`'s,
/// on any world or only `node`, older than the message `before` when paging.
/// With `after`, only messages newer than that one, oldest first: a live
/// feed's next ones.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatLookup {
    pub name: Option<String>,
    pub node: Option<u8>,
    pub before: Option<i64>,
    #[serde(default)]
    pub after: Option<i64>,
    #[serde(default)]
    pub max_staff_level: Option<u8>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRecord {
    pub id: i64,
    pub from: String,
    pub to: Option<String>,
    pub node: u8,
    pub x: u16,
    pub z: u16,
    pub level: u8,
    pub message: String,
    pub at_ms: i64,
}

/// `POST /v1/lookup/reports`: abuse reports from the in-game Report Abuse
/// screen newest first, everyone's or only those against `offender`, older
/// than the report `before` when paging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportLookup {
    pub offender: Option<String>,
    pub before: Option<i64>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportRecord {
    pub id: i64,
    pub reporter: String,
    pub offender: String,
    pub reason: u8,
    pub node: u8,
    pub x: u16,
    pub z: u16,
    pub level: u8,
    pub muted: bool,
    pub at_ms: i64,
}

/// `POST /v1/lookup/replays`: recorded sessions newest first, everyone's or
/// only `name`'s, only ones the anti-cheat checks flagged if `flagged`,
/// started before `before` (Unix ms) when paging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayLookup {
    pub name: Option<String>,
    pub flagged: bool,
    pub before: Option<i64>,
    #[serde(default)]
    pub max_staff_level: Option<u8>,
    pub limit: u32,
}

/// `POST /v1/lookup/replay`: answered with `Option<ReplaySummary>`.
/// `POST /v1/lookup/replay/data`: answered with the replay file
/// (`application/octet-stream`), 404 when it's gone.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayGet {
    pub id: String,
}

/// A recorded session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplaySummary {
    pub id: String,
    pub player: String,
    pub node: u8,
    pub revision: String,
    pub started_ms: i64,
    pub ended_ms: Option<i64>,
    pub updated_ms: i64,
    pub duration_ms: u32,
    pub bytes: u64,
    pub complete: bool,
    pub score: i16,
    pub flags: Vec<String>,
    pub stats: String,
}

/// `POST /v1/lookup/replay-storage`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReplayStorageLookup {}

/// How much the recordings take up.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayStorage {
    pub recordings: u64,
    /// Their chunks, compressed.
    pub bytes: u64,
    /// The tables on disk, indexes and overhead included.
    pub disk_bytes: u64,
    pub oldest_ms: Option<i64>,
    /// How long they run, all told.
    pub play_ms: u64,
    /// Those started in the last 24 hours.
    pub day_recordings: u64,
    pub day_bytes: u64,
}

/// `POST /v1/live/replay`: a session going on now on the world asked, to
/// watch live. Without `token`, the whole recording so far; with the token
/// that came in its header (`live.token`), only what's been recorded since.
/// Answered with a replay file like `REPLAY_DATA`'s; 404 once the token is
/// forgotten (unused for a minute), 409 when it isn't recording that session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveReplay {
    pub id: String,
    pub token: Option<String>,
}

/// `POST /v1/lookup/anticheat`: the players whose sessions the anti-cheat
/// checks flagged since `since_ms`, most flagged first, each with their
/// highest score in each of `buckets` equal stretches of the time since.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnticheatLookup {
    pub since_ms: i64,
    pub buckets: u32,
    #[serde(default)]
    pub max_staff_level: Option<u8>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnticheatLeader {
    pub name: String,
    pub sessions: i64,
    pub flagged: i64,
    pub max_score: i16,
    pub total_score: i64,
    pub play_ms: i64,
    pub flags: Vec<String>,
    pub last_flagged_ms: i64,
    pub trend: Vec<i16>,
    pub online_node: Option<u8>,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
}

/// `POST /v1/lookup/accounts`: non-staff accounts registered or logged in
/// since `since_ms`, most recent first, with what bot farms give away.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountsLookup {
    pub since_ms: i64,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountActivity {
    pub name: String,
    pub registered_ms: i64,
    pub registration_ip: Option<String>,
    pub login_ips: Vec<String>,
    pub sessions: i64,
    pub play_ms: i64,
    pub flagged: i64,
    pub max_score: i16,
    pub rhythm: Option<f64>,
    pub hours: Vec<i32>,
    pub x: Option<u16>,
    pub z: Option<u16>,
    pub level: Option<u8>,
    pub online_node: Option<u8>,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
}

/// `POST /v1/player/sheet`: answered with `Option<PlayerSheet>`, `None` when
/// no such player is registered.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SheetLookup {
    pub name: String,
}

/// A player's stats, items and position, live or from their save.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSheet {
    pub name: String,
    pub online: bool,
    pub x: u16,
    pub z: u16,
    pub level: u8,
    pub stats: Vec<StatLine>,
    pub invs: Vec<InvSheet>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatLine {
    pub id: u8,
    pub name: String,
    pub level: u8,
    pub base: u8,
    pub xp: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvSheet {
    pub name: String,
    pub size: u16,
    pub items: Vec<ItemLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemLine {
    pub slot: u16,
    pub id: u16,
    pub name: String,
    pub count: u32,
}

/// `POST /v1/player/edit`: answered with an [`Outcome`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerEditRequest {
    pub actor: Actor,
    pub name: String,
    pub edit: PlayerEdit,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Maintainers may make any of these; [`PlayerEdit::Restore`] needs an
/// admin and [`PlayerEdit::SendHome`] a moderator.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlayerEdit {
    SetStat { stat: u8, level: u8 },
    Give { inv: String, obj: u16, count: u32 },
    Take { inv: String, obj: u16, count: u32 },
    Move { x: u16, z: u16, level: u8 },
    Restore { items: Vec<RestoreItem> },
    SendHome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreItem {
    pub inv: String,
    pub obj: u16,
    pub count: u32,
}

/// The most items one restoration gives back.
pub const MAX_RESTORE_ITEMS: usize = 28;

/// `POST /v1/player/session`: answered with `Option<SessionState>`, `None`
/// when no such player is registered.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionLookup {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionState {
    pub owner: Option<SessionClaim>,
    pub here: SessionHold,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionClaim {
    pub node: u8,
    pub since_ms: Option<i64>,
    pub heartbeat_age_ms: Option<i64>,
    pub takeover_after_ms: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SessionHold {
    pub online: bool,
    pub logging_in: bool,
    pub logout_save: Option<bool>,
    pub editing: bool,
    pub local_save: bool,
}

/// `POST /v1/player/unstick`: answered with an [`Outcome`]. Sent to the
/// world that owns their session, or to any if that world is down (which
/// then releases it only once the owner's heartbeat has run out).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unstick {
    pub actor: Actor,
    pub name: String,
}

/// `POST /v1/player/history`: a player's earlier saves, newest first, older
/// than point `before` when paging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavePointsLookup {
    pub name: String,
    pub before: Option<i64>,
    #[serde(default)]
    pub saved_by_ms: Option<i64>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavePointLine {
    pub id: i64,
    pub kind: String,
    pub node: u8,
    pub note: Option<String>,
    pub saved_ms: i64,
}

/// `POST /v1/player/history/sheet`: answered with `Option<PlayerSheet>`
/// (`online` false), `None` if it isn't theirs or was pruned.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavePointLookup {
    pub name: String,
    pub id: i64,
}

/// `POST /v1/player/rollback`: puts an earlier save back in place of their
/// current one (which is kept in the history), answered with an
/// [`Outcome`]. Admins and up. A player online on the world it's sent to is
/// logged out first; one online on another world is refused.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rollback {
    pub actor: Actor,
    pub name: String,
    pub point: i64,
    pub reason: String,
}

/// `POST /v1/player/password-reset`: issues a one-time code the player
/// redeems on the website to set a new password. Admins and up. With
/// `lock`, their password stops working now and they're logged out of
/// this world.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasswordReset {
    pub actor: Actor,
    pub name: String,
    pub lock: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResetIssued {
    pub ok: bool,
    pub message: String,
    pub code: Option<String>,
    pub until_ms: Option<i64>,
}

/// `POST /v1/lookup/wealth`: answered with `Option<WealthLine>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WealthLookup {
    pub name: String,
    pub id: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WealthLine {
    pub id: i64,
    pub node: u8,
    pub x: u16,
    pub z: u16,
    pub level: u8,
    pub event_type: i16,
    pub item: String,
    pub count: i32,
    pub value: i32,
    pub at_ms: i64,
}

/// `POST /v1/player/progress`: answered with [`ProgressPoint`]s, oldest
/// first: the newest `limit` save points, then their save as it stands.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProgressLookup {
    pub name: String,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProgressPoint {
    pub kind: String,
    pub at_ms: i64,
    pub xp: i64,
    pub total_level: u32,
    pub wealth: i64,
}

/// `POST /v1/player/activity`: answered with [`ActivityHour`]s, oldest
/// first: how many events their scripts logged (`session_log`) in each
/// hour of the last `days` that had any.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityLookup {
    pub name: String,
    pub days: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityHour {
    pub hour_ms: i64,
    pub events: i64,
}

/// `POST /v1/objs`: items whose name or config name contains `query`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjSearch {
    pub query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjLine {
    pub id: u16,
    pub name: String,
    pub debugname: String,
    pub stackable: bool,
}

/// `POST /v1/news`: posts newest first, older than post `before` when paging,
/// with the categories posts go in.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsList {
    pub before: Option<i32>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsPage {
    pub categories: Vec<NewsCategory>,
    pub posts: Vec<NewsItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsCategory {
    pub id: i16,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: i32,
    pub category_id: i16,
    pub title: String,
    pub markdown: String,
    pub created_ms: i64,
}

/// `POST /v1/news/save`: a new post, or an edit of post `id`, in Markdown.
/// Answered with the [`NewsItem`] saved.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsSave {
    pub actor: Actor,
    pub id: Option<i32>,
    pub category_id: i16,
    pub title: String,
    pub markdown: String,
}

/// `POST /v1/news/render`: the HTML a post's Markdown shows as, for a preview.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsRender {
    pub markdown: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsRendered {
    pub html: String,
}

/// The answer to a `POST /v1/news/media` upload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaSaved {
    pub url: String,
    pub kind: String,
}

/// `POST /v1/news/delete`: answered with an [`Outcome`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsDelete {
    pub actor: Actor,
    pub id: i32,
}

/// `POST /v1/appeals/waiting`: appeals not yet taken into a case, oldest
/// first.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppealsWaiting {
    pub limit: u32,
}

/// An appeal against a mute or ban, as the player sent it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Appeal {
    pub id: i64,
    pub player: String,
    pub sanction: String,
    pub sanction_until_ms: i64,
    pub issued_by: Option<String>,
    pub reason: Option<String>,
    pub statement: String,
    pub filed_ms: i64,
}

/// `POST /v1/appeals/link`: notes the case an appeal was taken into.
/// Answered with an [`Outcome`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppealLink {
    pub id: i64,
    pub case_id: u64,
}

/// `POST /v1/appeals/decide`: the decision the player sees. Answered with
/// an [`Outcome`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppealDecide {
    pub actor: Actor,
    pub id: i64,
    pub decision: String,
    pub note: String,
}

/// The wealth event types the engine logs itself as items pass between
/// players or through the floor (`rs_db::transfer`). The content's
/// `^wealth_*` constants are all below them.
pub mod transfer {
    /// Handed over in a trade.
    pub const TRADE: i16 = 100;
    /// Lost as a duel stake.
    pub const STAKE: i16 = 101;
    /// Handed straight to another player some other way.
    pub const GIVE: i16 = 102;
    /// Dropped on the floor.
    pub const DROP: i16 = 103;
    /// Picked up from the floor.
    pub const TAKE: i16 = 104;
    /// Made out of nothing by a staff command (`::give` and the like).
    pub const SPAWNED: i16 = 105;
}

/// An item as the economy tools name it. `forms` are it and its noted
/// form, which saves and logs hold apart.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemForms {
    pub id: u16,
    pub name: String,
    pub debugname: String,
    pub forms: Vec<u16>,
}

/// `POST /v1/economy/trace`: where a player's item came from, step by step
/// back through the players it passed through, at most `depth` holders
/// back. Answered with `Option<ItemTrace>`, `None` when there's no such
/// item.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceLookup {
    pub name: String,
    pub obj: u16,
    pub depth: u8,
    #[serde(default)]
    pub max_staff_level: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemTrace {
    pub item: ItemForms,
    pub steps: Vec<TraceLine>,
}

/// One way a holder came by some of the item: from another player (a
/// handover, or a pickup of what they dropped), or from the game.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceLine {
    pub holder: String,
    pub depth: u8,
    pub parent: Option<u32>,
    pub id: i64,
    pub event_type: i16,
    pub at_ms: i64,
    pub count: i64,
    pub value: i64,
    pub node: u8,
    pub x: u16,
    pub z: u16,
    pub level: u8,
    pub from: Option<String>,
    pub detail: Option<String>,
}

/// `POST /v1/economy/holders`: the accounts whose saves hold at least
/// `min_count` of an item (noted ones counted in), the most first. Answered
/// with `Option<ItemHolders>`, `None` when there's no such item.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HoldersLookup {
    pub obj: u16,
    pub min_count: u32,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemHolders {
    pub item: ItemForms,
    pub total: i64,
    pub holders: i64,
    pub accounts: Vec<Holder>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Holder {
    pub name: String,
    pub staff_level: u8,
    pub total: i64,
    pub held_in: Vec<HeldIn>,
    pub saved_ms: i64,
    pub online_node: Option<u8>,
    pub banned_until_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeldIn {
    pub inv: String,
    pub count: i64,
}

/// `POST /v1/economy/totals`: how many of each item every save holds
/// between them (noted ones counted in), and in how many saves. Unknown
/// items are left out.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TotalsLookup {
    pub objs: Vec<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemCirculation {
    pub item: ItemForms,
    pub total: i64,
    pub holders: i64,
}

/// `POST /v1/economy/trades`: who a player handed items to or got them
/// from since `since_ms`, the `partners` they traded the most value with,
/// and everything handed over among them all. Empty when no such player is
/// registered.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeGraphLookup {
    pub name: String,
    pub since_ms: i64,
    pub partners: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TradeGraph {
    pub accounts: Vec<GraphAccount>,
    pub edges: Vec<TradeEdge>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphAccount {
    pub name: String,
    pub staff_level: u8,
    pub online_node: Option<u8>,
    pub banned_until_ms: Option<i64>,
}

/// Everything one player handed another.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeEdge {
    pub from: String,
    pub to: String,
    pub value: i64,
    pub handovers: i64,
    pub last_ms: i64,
    pub items: Vec<EdgeItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeItem {
    pub item: String,
    pub count: i64,
}

/// `POST /v1/economy/receivers`: the accounts at least `min_givers` players
/// handed items to since `since_ms`, those taking in the most more than they
/// gave first.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiversLookup {
    pub since_ms: i64,
    pub min_givers: u32,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopReceiver {
    pub name: String,
    pub staff_level: u8,
    pub givers: i64,
    pub handovers: i64,
    pub received: i64,
    pub given: i64,
    pub last_ms: i64,
    pub online_node: Option<u8>,
    pub banned_until_ms: Option<i64>,
}

/// `POST /v1/hiscores/player`: answered with `Option<HiscoreStanding>`,
/// `None` when no such player is registered.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HiscoresLookup {
    pub name: String,
}

/// A player's hiscore entries, and what keeps them off the tables: being
/// staff or banned does on its own; staff can hide them from every table or
/// remove their entries from some.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HiscoreStanding {
    pub staff_level: u8,
    pub banned: bool,
    pub hidden: bool,
    pub removed: Vec<i16>,
    pub rows: Vec<HiscoreRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HiscoreRow {
    pub category: i16,
    pub level: i16,
    pub xp: i64,
    pub rank: Option<i64>,
}

/// `POST /v1/hiscores/change`: answered with an [`Outcome`]. Admins and up.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HiscoresChange {
    pub actor: Actor,
    pub name: String,
    pub hidden: Option<bool>,
    pub remove: Vec<i16>,
    pub restore: Vec<i16>,
    pub reason: String,
}

/// `POST /v1/hiscores/kept-off`: the accounts staff keep off the hiscores.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeptOffLookup {
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeptOff {
    pub name: String,
    pub hidden: bool,
    pub removed: Vec<i16>,
}

/// `POST /v1/scripts/reload`: recompiles the world's scripts from its
/// content and, unless `check_only`, swaps them in without a restart. The
/// cache stays as it is, so it's for script fixes only. Maintainers only;
/// answered with [`ScriptsReloaded`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReloadScripts {
    pub actor: Actor,
    pub check_only: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptsReloaded {
    pub ok: bool,
    pub message: String,
    pub applied: bool,
    pub scripts: u32,
    pub compile_ms: u32,
    pub errors: Option<String>,
}

/// `ms` as a UTC date and time, as staff notes and logs give times.
pub fn utc_time(ms: i64) -> String {
    let (year, month, day, secs) = civil(ms);
    format!(
        "{year}-{month:02}-{day:02} {:02}:{:02} UTC",
        secs / 3600,
        secs % 3600 / 60
    )
}

/// `ms` in ISO 8601, to the second: `2026-10-20T09:05:33Z`.
pub fn iso_time(ms: i64) -> String {
    let (year, month, day, secs) = civil(ms);
    format!(
        "{year}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// `ms` as a UTC year, month, day and seconds into the day.
fn civil(ms: i64) -> (i64, i64, i64, i64) {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Days since 1970 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, rem)
}

/// The answer to a command: whether it took, and why not.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub ok: bool,
    pub message: String,
}

impl Outcome {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: message.into(),
        }
    }

    pub fn refused(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_dated_in_utc() {
        assert_eq!(utc_time(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc_time(1_792_497_600_000), "2026-10-20 12:00 UTC");
        assert_eq!(utc_time(951_782_400_000), "2000-02-29 00:00 UTC");
        assert_eq!(iso_time(1_792_497_633_999), "2026-10-20T12:00:33Z");
    }

    #[test]
    fn signatures_cover_every_part() {
        let sig = sign(b"secret", 100, "n1", "POST", "/v1/kick", b"{}");
        assert!(verify(
            b"secret", 100, "n1", "POST", "/v1/kick", b"{}", &sig
        ));
        assert!(!verify(
            b"other", 100, "n1", "POST", "/v1/kick", b"{}", &sig
        ));
        assert!(!verify(
            b"secret", 101, "n1", "POST", "/v1/kick", b"{}", &sig
        ));
        assert!(!verify(
            b"secret", 100, "n2", "POST", "/v1/kick", b"{}", &sig
        ));
        assert!(!verify(
            b"secret", 100, "n1", "GET", "/v1/kick", b"{}", &sig
        ));
        assert!(!verify(
            b"secret", 100, "n1", "POST", "/v1/ban", b"{}", &sig
        ));
        assert!(!verify(
            b"secret", 100, "n1", "POST", "/v1/kick", b"{ }", &sig
        ));
        assert!(!verify(
            b"secret", 100, "n1", "POST", "/v1/kick", b"{}", "zz"
        ));
    }

    #[test]
    fn verifier_refuses_stale_replayed_and_unsigned() {
        let verifier = Verifier::new("secret");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let headers = |timestamp: u64, nonce: &'static str| {
            let signature = sign(b"secret", timestamp, nonce, "POST", "/v1/news", b"{}");
            move |name: &str| -> Option<String> {
                match name {
                    TIMESTAMP_HEADER => Some(timestamp.to_string()),
                    NONCE_HEADER => Some(nonce.to_owned()),
                    SIGNATURE_HEADER => Some(signature.clone()),
                    _ => None,
                }
            }
        };
        let check = |h: &dyn Fn(&str) -> Option<String>, body: &[u8]| {
            let (t, n, s) = (h(TIMESTAMP_HEADER), h(NONCE_HEADER), h(SIGNATURE_HEADER));
            verifier.check(
                |name| match name {
                    TIMESTAMP_HEADER => t.as_deref(),
                    NONCE_HEADER => n.as_deref(),
                    SIGNATURE_HEADER => s.as_deref(),
                    _ => None,
                },
                "POST",
                "/v1/news",
                body,
            )
        };
        let fresh = headers(now, "0123456789abcdef");
        assert_eq!(check(&fresh, b"{}"), Ok(()));
        assert_eq!(check(&fresh, b"{}"), Err("nonce replayed"));
        assert_eq!(
            check(&headers(now, "fedcba9876543210"), b"{ }"),
            Err("bad signature")
        );
        assert_eq!(
            check(&headers(now - 120, "aaaaaaaaaaaaaaaa"), b"{}"),
            Err("timestamp out of range")
        );
        assert_eq!(check(&headers(now, "short"), b"{}"), Err("no nonce"));
    }

    #[test]
    fn hex_round_trips() {
        assert_eq!(hex(&[0, 0xab, 0xff]), "00abff");
        assert_eq!(unhex("00abff"), Some(vec![0, 0xab, 0xff]));
        assert_eq!(unhex("0"), None);
        assert_eq!(unhex("zz"), None);
    }
}
