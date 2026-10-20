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
    /// Lookups in the game database, which any world can answer.
    pub const PLAYER: &str = "/v1/lookup/player";
    pub const MODERATION: &str = "/v1/lookup/moderation";
    pub const SANCTIONS: &str = "/v1/lookup/sanctions";
    pub const CHAT: &str = "/v1/lookup/chat";
    pub const REPORTS: &str = "/v1/lookup/reports";
    /// A player's history across every log, and the accounts linked to them.
    pub const TIMELINE: &str = "/v1/lookup/timeline";
    pub const LINKS: &str = "/v1/lookup/links";
    /// Session recordings: the list, one, and one's replay file.
    pub const REPLAYS: &str = "/v1/lookup/replays";
    pub const REPLAY: &str = "/v1/lookup/replay";
    pub const REPLAY_DATA: &str = "/v1/lookup/replay/data";
    /// Player editing: answered by the world the player is on, or by any
    /// world while they're logged out.
    pub const PLAYER_SHEET: &str = "/v1/player/sheet";
    pub const PLAYER_EDIT: &str = "/v1/player/edit";
    pub const OBJ_SEARCH: &str = "/v1/objs";
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
    // Defaulted: worlds on an older release don't send them.
    /// Whether new logins are refused (staff can still log in).
    #[serde(default)]
    pub maintenance: bool,
    /// The reason given with the pending reboot, if any.
    #[serde(default)]
    pub reboot_reason: Option<String>,
    #[serde(default)]
    pub health: Option<Health>,
    #[serde(default)]
    pub controls: Option<WorldControls>,
}

/// A world's kill switches and login lockdown. They last until it restarts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldControls {
    /// Script-name patterns (`*` matches anything) that players can't start.
    pub switches: Vec<String>,
    /// Non-staff accounts registered less than this long ago can't log in;
    /// 0 for no lockdown.
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
    /// Mean time a tick's work took.
    pub tick_ms: f32,
    /// The slowest tick.
    pub tick_max_ms: f32,
    /// Ticks that took longer than the 600ms tick.
    pub overruns: u32,
    /// The process's resident memory, where the platform reports it.
    pub memory_bytes: Option<u64>,
    /// Open game connections.
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
    /// Shown to players with the countdown: "Rebooting in 5:00: update".
    #[serde(default)]
    pub reason: Option<String>,
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
    /// Why, for the moderation log.
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerRecord {
    pub name: String,
    pub staff_level: u8,
    pub registered_ms: i64,
    /// The node they're on now.
    pub online_node: Option<u8>,
    pub online_since_ms: Option<i64>,
    /// Where and when they last logged out.
    pub last_node: Option<u8>,
    pub last_seen_ms: Option<i64>,
    /// When the mute or ban in force ends.
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
}

/// `POST /v1/lookup/moderation`: moderation commands newest first, all or
/// only those on `name`, older than the entry `before` when paging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModerationLookup {
    pub name: Option<String>,
    pub before: Option<i64>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModerationRecord {
    pub id: i64,
    pub moderator: String,
    pub target: String,
    /// `mute`, `unmute`, `ban`, `unban` or `kick`.
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
    /// Which kinds of row to include (see [`TimelineEntry::kind`]).
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
    /// The row's id in its own log (for `replay`, the recording's).
    pub id: String,
    pub at_ms: i64,
    pub node: Option<u8>,
    pub x: Option<u16>,
    pub z: Option<u16>,
    pub level: Option<u8>,
    pub event_type: Option<i16>,
    pub text: Option<String>,
    /// Another player: the moderator, reporter, recipient or offender.
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginAddress {
    pub ip: String,
    pub logins: i64,
    pub first_ms: i64,
    pub last_ms: i64,
    /// Other accounts that logged in or registered from it.
    pub others: i64,
}

/// An account sharing at least one address with the player looked up.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkedAccount {
    pub name: String,
    pub staff_level: u8,
    /// The addresses they share.
    pub shared: Vec<String>,
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
    /// The latest mute or ban put on them, when it was logged.
    pub last_action: Option<String>,
    pub moderator: Option<String>,
    pub at_ms: Option<i64>,
}

/// `POST /v1/lookup/chat`: chat newest first, everyone's or only `name`'s,
/// on any world or only `node`, older than the message `before` when paging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatLookup {
    pub name: Option<String>,
    pub node: Option<u8>,
    pub before: Option<i64>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRecord {
    pub id: i64,
    pub from: String,
    /// The recipient of a private message, `None` for public chat.
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlayerEdit {
    SetStat { stat: u8, level: u8 },
    Give { inv: String, obj: u16, count: u32 },
    Take { inv: String, obj: u16, count: u32 },
    Move { x: u16, z: u16, level: u8 },
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
    /// `mute` or `ban`.
    pub sanction: String,
    pub sanction_until_ms: i64,
    /// The moderator who set it, `None` when nobody did (an abuse report's
    /// mute, the anti-cheat's ban).
    pub issued_by: Option<String>,
    /// The reason they logged with it.
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
    /// `upheld` or `overturned`.
    pub decision: String,
    pub note: String,
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
