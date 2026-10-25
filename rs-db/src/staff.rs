use crate::links::{CROWDED_ADDRESS, CROWDED_BROWSER, CROWDED_FINGERPRINT, uncrowded};
use crate::{Db, Error};
use sqlx::{Postgres, QueryBuilder};

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// The columns of a [`PlayerRecord`], from `players`.
macro_rules! player_record {
    () => {
        concat!(
            "SELECT name_hash, display_name, staff_mod_level, ",
            ms!("created_at"),
            " AS registered_ms, CASE WHEN muted_until > now() THEN ",
            ms!("muted_until"),
            " END AS muted_until_ms, CASE WHEN banned_until > now() THEN ",
            ms!("banned_until"),
            " END AS banned_until_ms, online_node, ",
            ms!("online_since"),
            " AS online_since_ms, last_logout_node, ",
            ms!("last_logout_at"),
            " AS last_logout_ms, CASE WHEN reset_code_until > now() THEN ",
            ms!("reset_code_until"),
            " END AS reset_until_ms FROM players"
        )
    };
}

/// The longest name, in base37 digits.
const NAME_DIGITS: u32 = 12;

/// The base37 ranges the names starting with `prefix_hash`'s fall in, one
/// per length: a name `m` characters longer is the prefix times `37^m`
/// plus those characters. Ascending, so in name order shortest first.
fn prefix_ranges(prefix_hash: u64) -> impl Iterator<Item = (i64, i64)> {
    let mut digits = 0;
    let mut rest = prefix_hash;
    while rest > 0 {
        rest /= 37;
        digits += 1;
    }
    (0..=NAME_DIGITS.saturating_sub(digits)).map(move |m| {
        let span = 37u64.pow(m);
        let lo = prefix_hash * span;
        (lo as i64, (lo + span - 1) as i64)
    })
}

/// How far the other accounts on an address or install are counted: past
/// it, it's crowded anyway.
pub const MAX_OTHERS: i64 = 100;

/// A registered player, as staff see them.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PlayerRecord {
    pub name_hash: i64,
    pub display_name: String,
    pub staff_mod_level: i16,
    pub registered_ms: i64,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
    pub online_node: Option<i16>,
    pub online_since_ms: Option<i64>,
    pub last_logout_node: Option<i16>,
    pub last_logout_ms: Option<i64>,
    /// When the password reset staff issued them runs out, while it stands.
    pub reset_until_ms: Option<i64>,
}

/// One logged `wealth_event`.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct WealthRecord {
    pub id: i64,
    pub node_id: i16,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub event_type: i16,
    pub item: String,
    pub count: i32,
    pub value: i32,
    pub at_ms: i64,
}

/// One staff moderation command that took.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ModerationRecord {
    pub id: i64,
    pub moderator: String,
    pub target: String,
    pub action: String,
    pub hours: Option<i32>,
    pub node_id: i16,
    pub at_ms: i64,
    pub reason: Option<String>,
}

/// A player under a mute or ban now, with the latest mute or ban put on
/// them (`None` when it predates the log, or came from an abuse report).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct SanctionRecord {
    pub display_name: String,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
    pub last_action: Option<String>,
    pub moderator: Option<String>,
    pub at_ms: Option<i64>,
}

/// One logged chat message.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ChatRecord {
    pub id: i64,
    pub sender: String,
    pub to_name_hash: Option<i64>,
    pub node_id: i16,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub message: String,
    pub at_ms: i64,
}

/// One abuse report from the in-game Report Abuse screen.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ReportRecord {
    pub id: i64,
    pub reporter: String,
    pub offender_name_hash: i64,
    pub offender: Option<String>,
    pub reason: i16,
    pub node_id: i16,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub muted: bool,
    pub at_ms: i64,
}

/// An address a player has logged in from.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct LoginAddress {
    pub ip: String,
    pub logins: i64,
    pub first_ms: i64,
    pub last_ms: i64,
    /// How many other accounts logged in or registered from it, counted up
    /// to [`MAX_OTHERS`] + 1.
    pub others: i64,
}

/// A device a player has logged in with: one browser (or desktop client)
/// install, as its client described it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct DeviceRecord {
    /// `None` for logins from before devices were recorded.
    pub web: Option<bool>,
    pub user_agent: Option<String>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub touch: bool,
    /// The install's id (see `0015_login_uid.sql`), when it sent one.
    pub uid: Option<i32>,
    /// The device's fingerprint (see `0042_multilogging.sql`), when the
    /// client page sent one, and what else it measured.
    pub fingerprint: Option<i64>,
    pub engine: Option<String>,
    pub js_platform: Option<String>,
    pub gpu: Option<String>,
    /// How the check and the browser's claims disagreed, on any of these
    /// logins.
    pub tamper: Vec<String>,
    pub logins: i64,
    /// How many addresses they used it from, and up to 20 of them.
    pub addresses: i64,
    pub ips: Vec<String>,
    pub first_ms: i64,
    pub last_ms: i64,
    /// How many other accounts logged in from the same install, counted up
    /// to [`MAX_OTHERS`] + 1.
    pub others: i64,
    /// And with the same device fingerprint, counted the same way.
    pub fingerprint_others: i64,
}

/// Another account linked to a player: one of them logged in or registered
/// from an address the other did, or they logged in from the same client
/// install (its uid).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct LinkedAccount {
    pub display_name: String,
    pub staff_mod_level: i16,
    /// The addresses they share.
    pub shared: Vec<String>,
    /// How many client installs they share.
    pub shared_clients: i64,
    /// How many (uncrowded) device fingerprints they share.
    pub shared_fingerprints: i64,
    /// Their logins from the shared addresses or installs.
    pub logins: i64,
    /// Their latest login (or registration) from a shared address or install.
    pub last_ms: i64,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
}

/// One row of a player's timeline. Which columns are set depends on `kind`:
///
/// - `login`: `text` is the address.
/// - `session`: a script's `session_log`: `event_type`, `text`.
/// - `wealth`: a script's `wealth_event` or one the engine logs as items
///   pass between players: `event_type`, `text` (the item), `count`,
///   `value`, and for a handover `other` (who it went to, or with `detail`
///   "received", who it came from).
/// - `chat`: `text`, and `other_hash` for a private message's recipient.
/// - `moderation`: done to them: `text` (the action), `other` (the
///   moderator), `hours`, `detail` (the reason).
/// - `reported`: an abuse report against them: `event_type` (the rule),
///   `other` (the reporter).
/// - `report`: one they sent: `event_type` (the rule), `other_hash` (the
///   offender).
/// - `replay`: a recorded session starting: `id` (the recording), `count`
///   (its length in ms), `event_type` (its anti-cheat score), `text` (its
///   flags, comma-separated).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct TimelineRow {
    pub kind: String,
    pub id: i64,
    pub at_ms: i64,
    pub node_id: Option<i16>,
    pub x: Option<i16>,
    pub z: Option<i16>,
    pub y: Option<i16>,
    pub event_type: Option<i16>,
    pub text: Option<String>,
    pub other: Option<String>,
    pub other_hash: Option<i64>,
    pub count: Option<i64>,
    pub value: Option<i64>,
    pub hours: Option<i32>,
    pub detail: Option<String>,
}

/// Where a timeline page starts: rows strictly older than this one.
#[derive(Clone, Debug)]
pub struct TimelineCursor {
    pub at_ms: i64,
    pub kind: String,
    pub id: i64,
}

/// Every kind of row a timeline can show.
pub const TIMELINE_KINDS: [&str; 8] = [
    "login",
    "session",
    "wealth",
    "chat",
    "moderation",
    "reported",
    "report",
    "replay",
];

/// The `SELECT` for one kind of timeline row, with `$P` for the player's id,
/// `$B` for the newest time to include and `$L` for the most rows. Each reads
/// newest first along an index on (player, time) and stops at `$L`. Every
/// column is typed, so any mix of kinds unions into [`TimelineRow`].
fn timeline_select(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "login" => concat!(
            "SELECT 'login'::text, id, ",
            ms!("created_at"),
            ", node_id, NULL::smallint, NULL::smallint, NULL::smallint, NULL::smallint, ",
            "host(ip), NULL::text, NULL::bigint, NULL::bigint, NULL::bigint, NULL::int, ",
            // The device: its user agent, or `desktop` for the desktop client.
            "CASE WHEN web THEN user_agent WHEN NOT web THEN 'desktop' END ",
            "FROM login_logs WHERE player_id = $P AND created_at <= $B ",
            "ORDER BY created_at DESC LIMIT $L"
        ),
        "session" => concat!(
            "SELECT 'session'::text, id, ",
            ms!("created_at"),
            ", node_id, x, z, y, event_type, event, NULL::text, NULL::bigint, NULL::bigint, ",
            "NULL::bigint, NULL::int, NULL::text FROM session_logs ",
            "WHERE player_id = $P AND created_at <= $B ORDER BY created_at DESC LIMIT $L"
        ),
        // Their own, and handovers to them (`detail` "received").
        "wealth" => concat!(
            "SELECT * FROM ((SELECT 'wealth'::text, w.id, ",
            ms!("w.created_at"),
            ", w.node_id, w.x, w.z, w.y, w.event_type, w.item, c.display_name, NULL::bigint, ",
            "w.count::bigint, w.value::bigint, NULL::int, NULL::text FROM wealth_events w ",
            "LEFT JOIN players c ON c.id = w.counterpart_id ",
            "WHERE w.player_id = $P AND w.created_at <= $B ORDER BY w.created_at DESC LIMIT $L) ",
            "UNION ALL (SELECT 'wealth'::text, w.id, ",
            ms!("w.created_at"),
            ", w.node_id, w.x, w.z, w.y, w.event_type, w.item, g.display_name, NULL::bigint, ",
            "w.count::bigint, w.value::bigint, NULL::int, 'received'::text FROM wealth_events w ",
            "JOIN players g ON g.id = w.player_id ",
            "WHERE w.counterpart_id = $P AND w.created_at <= $B ",
            "ORDER BY w.created_at DESC LIMIT $L)) AS wealth"
        ),
        "chat" => concat!(
            "SELECT 'chat'::text, id, ",
            ms!("created_at"),
            ", node_id, x, z, y, NULL::smallint, message, NULL::text, to_name_hash, ",
            "NULL::bigint, NULL::bigint, NULL::int, NULL::text FROM chat_logs ",
            "WHERE player_id = $P AND created_at <= $B ORDER BY created_at DESC LIMIT $L"
        ),
        "moderation" => concat!(
            "SELECT 'moderation'::text, m.id, ",
            ms!("m.created_at"),
            ", m.node_id, NULL::smallint, NULL::smallint, NULL::smallint, NULL::smallint, ",
            "m.action, mp.display_name, NULL::bigint, NULL::bigint, NULL::bigint, m.hours, ",
            "m.reason FROM moderation_logs m JOIN players mp ON mp.id = m.moderator_id ",
            "WHERE m.target_id = $P AND m.created_at <= $B ORDER BY m.created_at DESC LIMIT $L"
        ),
        "reported" => concat!(
            "SELECT 'reported'::text, s.id, ",
            ms!("s.created_at"),
            ", s.node_id, s.x, s.z, s.y, s.reason, NULL::text, r.display_name, NULL::bigint, ",
            "NULL::bigint, NULL::bigint, NULL::int, NULL::text FROM snapshots s ",
            "JOIN players r ON r.id = s.reporter_id ",
            "WHERE s.offender_name_hash = (SELECT name_hash FROM players WHERE id = $P) ",
            "AND s.created_at <= $B ORDER BY s.created_at DESC LIMIT $L"
        ),
        "report" => concat!(
            "SELECT 'report'::text, id, ",
            ms!("created_at"),
            ", node_id, x, z, y, reason, NULL::text, NULL::text, offender_name_hash, ",
            "NULL::bigint, NULL::bigint, NULL::int, NULL::text FROM snapshots ",
            "WHERE reporter_id = $P AND created_at <= $B ORDER BY created_at DESC LIMIT $L"
        ),
        "replay" => concat!(
            "SELECT 'replay'::text, id, ",
            ms!("started_at"),
            ", node_id, NULL::smallint, NULL::smallint, NULL::smallint, score, ",
            "array_to_string(flags, ','), NULL::text, NULL::bigint, duration_ms::bigint, ",
            "NULL::bigint, NULL::int, NULL::text FROM replays ",
            "WHERE player_id = $P AND started_at <= $B ORDER BY started_at DESC LIMIT $L"
        ),
        _ => return None,
    })
}

impl Db {
    /// The addresses `name_hash` has logged in from, most recent first.
    pub async fn login_addresses(
        &self,
        name_hash: u64,
        limit: i64,
    ) -> Result<Vec<LoginAddress>, Error> {
        sqlx::query_as(concat!(
            "WITH me AS (SELECT id FROM players WHERE name_hash = $1) ",
            "SELECT host(l.ip) AS ip, count(*) AS logins, ",
            ms!("min(l.created_at)"),
            " AS first_ms, ",
            ms!("max(l.created_at)"),
            " AS last_ms, least($3 + 1, (SELECT count(*) FROM (SELECT o.player_id ",
            "FROM login_ips o WHERE o.ip = l.ip AND o.player_id <> (SELECT id FROM me) LIMIT $3 + 1) a) ",
            "+ (SELECT count(*) FROM players r WHERE r.registration_ip = l.ip AND r.id <> (SELECT id FROM me) ",
            "AND NOT EXISTS (SELECT 1 FROM login_ips o WHERE o.ip = l.ip AND o.player_id = r.id))) AS others ",
            "FROM login_logs l WHERE l.player_id = (SELECT id FROM me) ",
            "GROUP BY l.ip ORDER BY max(l.created_at) DESC LIMIT $2"
        ))
        .bind(name_hash as i64)
        .bind(limit)
        .bind(MAX_OTHERS)
        .fetch_all(&self.pool)
        .await
    }

    /// The devices `name_hash` has logged in with, most recent first: one row
    /// per client install and what it said about itself.
    pub async fn login_devices(
        &self,
        name_hash: u64,
        limit: i64,
    ) -> Result<Vec<DeviceRecord>, Error> {
        sqlx::query_as(concat!(
            "WITH me AS (SELECT id FROM players WHERE name_hash = $1), ",
            "seen AS (SELECT web, user_agent, platform, language, touch, uid, fingerprint, ",
            "engine, js_platform, gpu, ",
            "ARRAY(SELECT DISTINCT t FROM unnest(string_to_array(string_agg(array_to_string(tamper, ','), ','), ',')) t ",
            "WHERE t <> '' ORDER BY 1) AS tamper, ",
            "count(*) AS logins, count(DISTINCT ip) AS addresses, ",
            "(array_agg(DISTINCT host(ip)))[1:20] AS ips, ",
            ms!("min(created_at)"),
            " AS first_ms, ",
            ms!("max(created_at)"),
            " AS last_ms FROM login_logs WHERE player_id = (SELECT id FROM me) ",
            "GROUP BY web, user_agent, platform, language, touch, uid, fingerprint, engine, js_platform, gpu) ",
            "SELECT web, user_agent, platform, language, touch, uid, fingerprint, engine, js_platform, gpu, ",
            "tamper, logins, addresses, ips, ",
            "first_ms, last_ms, ",
            "CASE WHEN uid IS NULL THEN 0 ELSE (SELECT count(*) FROM (SELECT o.player_id ",
            "FROM login_uids o WHERE o.uid = seen.uid AND o.player_id <> (SELECT id FROM me) ",
            "LIMIT $3 + 1) a) END AS others, ",
            "CASE WHEN fingerprint IS NULL THEN 0 ELSE (SELECT count(*) FROM (SELECT o.player_id ",
            "FROM login_fingerprints o WHERE o.fingerprint = seen.fingerprint ",
            "AND o.player_id <> (SELECT id FROM me) LIMIT $3 + 1) a) END AS fingerprint_others ",
            "FROM seen ORDER BY last_ms DESC LIMIT $2"
        ))
        .bind(name_hash as i64)
        .bind(limit)
        .bind(MAX_OTHERS)
        .fetch_all(&self.pool)
        .await
    }

    /// The other accounts linked to `name_hash` by an address or a client
    /// install (see [`LinkedAccount`]), those sharing the most first.
    /// Crowded ones ([`CROWDED_ADDRESS`], [`CROWDED_BROWSER`],
    /// [`CROWDED_FINGERPRINT`]) link nobody.
    pub async fn linked_accounts(
        &self,
        name_hash: u64,
        limit: i64,
    ) -> Result<Vec<LinkedAccount>, Error> {
        sqlx::query_as(concat!(
            "WITH me AS (SELECT id, registration_ip FROM players WHERE name_hash = $1), ",
            "mine AS (SELECT a.ip FROM (SELECT ip FROM login_ips WHERE player_id = (SELECT id FROM me) ",
            "UNION SELECT registration_ip FROM me WHERE registration_ip IS NOT NULL) a WHERE ",
            uncrowded!("ip", "a.ip", "$3"),
            "), my_uids AS (SELECT u.uid FROM login_uids u WHERE u.player_id = (SELECT id FROM me) AND ",
            uncrowded!("uid", "u.uid", "$4"),
            "), my_fps AS (SELECT f.fingerprint FROM login_fingerprints f ",
            "WHERE f.player_id = (SELECT id FROM me) AND ",
            uncrowded!("fingerprint", "f.fingerprint", "$5"),
            "), ",
            // What each shares, and their latest login (or registration) with it.
            "seen AS (SELECT l.player_id, l.ip, NULL::int AS uid, NULL::bigint AS fingerprint, l.last_at ",
            "FROM login_ips l JOIN mine m ON m.ip = l.ip WHERE l.player_id <> (SELECT id FROM me) ",
            "UNION ALL SELECT l.player_id, NULL, l.uid, NULL, l.last_at FROM login_uids l ",
            "JOIN my_uids u ON u.uid = l.uid WHERE l.player_id <> (SELECT id FROM me) ",
            "UNION ALL SELECT l.player_id, NULL, NULL, l.fingerprint, l.last_at FROM login_fingerprints l ",
            "JOIN my_fps f ON f.fingerprint = l.fingerprint WHERE l.player_id <> (SELECT id FROM me) ",
            "UNION ALL SELECT p.id, p.registration_ip, NULL, NULL, p.created_at FROM players p ",
            "JOIN mine m ON m.ip = p.registration_ip WHERE p.id <> (SELECT id FROM me)), ",
            "top AS (SELECT player_id, array_agg(DISTINCT ip) FILTER (WHERE ip IS NOT NULL) AS ips, ",
            "array_agg(DISTINCT uid) FILTER (WHERE uid IS NOT NULL) AS uids, ",
            "array_agg(DISTINCT fingerprint) FILTER (WHERE fingerprint IS NOT NULL) AS fps, ",
            "count(DISTINCT ip) + count(DISTINCT uid) + count(DISTINCT fingerprint) AS shares, ",
            "max(last_at) AS last_at ",
            "FROM seen GROUP BY player_id ORDER BY shares DESC, last_at DESC LIMIT $2) ",
            "SELECT p.display_name, p.staff_mod_level, ",
            "ARRAY(SELECT host(i) FROM unnest(coalesce(t.ips, '{}')) i ORDER BY 1) AS shared, ",
            "coalesce(cardinality(t.uids), 0)::bigint AS shared_clients, ",
            "coalesce(cardinality(t.fps), 0)::bigint AS shared_fingerprints, ",
            "(SELECT count(*) FROM login_logs l WHERE l.player_id = t.player_id ",
            "AND (l.ip = ANY(t.ips) OR l.uid = ANY(t.uids) OR l.fingerprint = ANY(t.fps))) AS logins, ",
            ms!("t.last_at"),
            " AS last_ms, CASE WHEN p.muted_until > now() THEN ",
            ms!("p.muted_until"),
            " END AS muted_until_ms, CASE WHEN p.banned_until > now() THEN ",
            ms!("p.banned_until"),
            " END AS banned_until_ms FROM top t JOIN players p ON p.id = t.player_id ",
            "ORDER BY t.shares DESC, t.last_at DESC"
        ))
        .bind(name_hash as i64)
        .bind(limit)
        .bind(CROWDED_ADDRESS)
        .bind(CROWDED_BROWSER)
        .bind(CROWDED_FINGERPRINT)
        .fetch_all(&self.pool)
        .await
    }

    /// `name_hash`'s timeline: rows of each of `kinds` (see [`TimelineRow`]),
    /// newest first, older than `before` when paging. Unknown kinds are
    /// skipped.
    pub async fn timeline(
        &self,
        name_hash: u64,
        kinds: &[String],
        before: Option<&TimelineCursor>,
        limit: i64,
    ) -> Result<Vec<TimelineRow>, Error> {
        let selects: Vec<&str> = kinds.iter().filter_map(|k| timeline_select(k)).collect();
        if selects.is_empty() {
            return Ok(Vec::new());
        }
        let player_id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM players WHERE name_hash = $1")
                .bind(name_hash as i64)
                .fetch_optional(&self.pool)
                .await?;
        let Some(player_id) = player_id else {
            return Ok(Vec::new());
        };
        // Each branch reads no further back than the page can reach; rows in
        // the cursor's own millisecond are split by its kind and id below.
        let newest = match before {
            Some(c) => format!("to_timestamp({}::double precision / 1000)", c.at_ms + 1),
            None => "now()".to_owned(),
        };
        let mut query = QueryBuilder::<Postgres>::new("SELECT * FROM (");
        for (i, select) in selects.iter().enumerate() {
            if i > 0 {
                query.push(" UNION ALL ");
            }
            // Only numbers are spliced in.
            let select = select
                .replace("$P", &player_id.to_string())
                .replace("$B", &newest)
                .replace("$L", &limit.to_string());
            query.push("(").push(select).push(")");
        }
        query.push(
            ") AS t (kind, id, at_ms, node_id, x, z, y, event_type, text, other, other_hash, \
             count, value, hours, detail)",
        );
        if let Some(c) = before {
            query
                .push(" WHERE (at_ms, kind, id) < (")
                .push_bind(c.at_ms)
                .push(", ")
                .push_bind(c.kind.clone())
                .push(", ")
                .push_bind(c.id)
                .push(")");
        }
        query
            .push(" ORDER BY at_ms DESC, kind DESC, id DESC LIMIT ")
            .push_bind(limit);
        query.build_query_as().fetch_all(&self.pool).await
    }

    /// The player with base37 name `name_hash`, if registered.
    pub async fn player_record(&self, name_hash: u64) -> Result<Option<PlayerRecord>, Error> {
        sqlx::query_as(concat!(player_record!(), " WHERE name_hash = $1"))
            .bind(name_hash as i64)
            .fetch_optional(&self.pool)
            .await
    }

    /// Up to `limit` registered players whose names start with the one
    /// `prefix_hash` is the base37 of: that name first, then shortest
    /// first, then in base37 order.
    pub async fn players_by_prefix(
        &self,
        prefix_hash: u64,
        limit: i64,
    ) -> Result<Vec<PlayerRecord>, Error> {
        let (lo, hi): (Vec<i64>, Vec<i64>) = prefix_ranges(prefix_hash).unzip();
        sqlx::query_as(concat!(
            player_record!(),
            " JOIN unnest($1::bigint[], $2::bigint[]) AS r (lo, hi)
             ON name_hash BETWEEN r.lo AND r.hi ORDER BY name_hash LIMIT $3"
        ))
        .bind(lo)
        .bind(hi)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// One of a player's wealth events, by id.
    pub async fn wealth_event(
        &self,
        name_hash: u64,
        id: i64,
    ) -> Result<Option<WealthRecord>, Error> {
        sqlx::query_as(concat!(
            "SELECT w.id, w.node_id, w.x, w.z, w.y, w.event_type, w.item, w.count, w.value, ",
            ms!("w.created_at"),
            " AS at_ms FROM wealth_events w JOIN players p ON p.id = w.player_id
             WHERE p.name_hash = $1 AND w.id = $2"
        ))
        .bind(name_hash as i64)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Moderation commands newest first: all of them, or only those on the
    /// player `target` and those by `moderator`, older than the entry
    /// `before` when paging.
    pub async fn moderation_history(
        &self,
        target: Option<u64>,
        moderator: Option<u64>,
        before: Option<i64>,
        limit: i64,
    ) -> Result<Vec<ModerationRecord>, Error> {
        let mut query = QueryBuilder::<Postgres>::new(concat!(
            "SELECT m.id, mp.display_name AS moderator, tp.display_name AS target, m.action, ",
            "m.hours, m.node_id, ",
            ms!("m.created_at"),
            " AS at_ms, m.reason FROM moderation_logs m ",
            "JOIN players mp ON mp.id = m.moderator_id ",
            "JOIN players tp ON tp.id = m.target_id WHERE true"
        ));
        if let Some(target) = target {
            query
                .push(" AND m.target_id = (SELECT id FROM players WHERE name_hash = ")
                .push_bind(target as i64)
                .push(")");
        }
        if let Some(moderator) = moderator {
            query
                .push(" AND mp.name_hash = ")
                .push_bind(moderator as i64);
        }
        if let Some(before) = before {
            query.push(" AND m.id < ").push_bind(before);
        }
        query.push(" ORDER BY m.id DESC LIMIT ").push_bind(limit);
        query.build_query_as().fetch_all(&self.pool).await
    }

    /// Everyone muted or banned now, most recently sanctioned first.
    pub async fn sanctioned_players(&self, limit: i64) -> Result<Vec<SanctionRecord>, Error> {
        sqlx::query_as(concat!(
            "SELECT p.display_name, CASE WHEN p.muted_until > now() THEN ",
            ms!("p.muted_until"),
            " END AS muted_until_ms, CASE WHEN p.banned_until > now() THEN ",
            ms!("p.banned_until"),
            " END AS banned_until_ms, last.action AS last_action, last.moderator, last.at_ms ",
            "FROM players p LEFT JOIN LATERAL (",
            "SELECT m.action, mp.display_name AS moderator, ",
            ms!("m.created_at"),
            " AS at_ms FROM moderation_logs m JOIN players mp ON mp.id = m.moderator_id ",
            "WHERE m.target_id = p.id AND m.action IN ('mute', 'ban') ",
            "ORDER BY m.id DESC LIMIT 1) last ON true ",
            "WHERE p.muted_until > now() OR p.banned_until > now() ",
            "ORDER BY last.at_ms DESC NULLS LAST, p.display_name LIMIT $1"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Chat messages newest first: everyone's, or only `sender`'s, on any
    /// world or only world `node_id`, older than the message `before` when
    /// paging. With `after`, only those newer than that message, oldest
    /// first. With `max_staff_level`, none from or to staff above it.
    pub async fn chat_history(
        &self,
        sender: Option<u64>,
        node_id: Option<i16>,
        before: Option<i64>,
        after: Option<i64>,
        max_staff_level: Option<i16>,
        limit: i64,
    ) -> Result<Vec<ChatRecord>, Error> {
        let mut query = QueryBuilder::<Postgres>::new(concat!(
            "SELECT c.id, p.display_name AS sender, c.to_name_hash, c.node_id, c.x, c.z, c.y, ",
            "c.message, ",
            ms!("c.created_at"),
            " AS at_ms FROM chat_logs c JOIN players p ON p.id = c.player_id WHERE true"
        ));
        if let Some(sender) = sender {
            query
                .push(" AND c.player_id = (SELECT id FROM players WHERE name_hash = ")
                .push_bind(sender as i64)
                .push(")");
        }
        if let Some(node_id) = node_id {
            query.push(" AND c.node_id = ").push_bind(node_id);
        }
        if let Some(before) = before {
            query.push(" AND c.id < ").push_bind(before);
        }
        if let Some(after) = after {
            query.push(" AND c.id > ").push_bind(after);
        }
        // Neither from nor to staff above that level.
        if let Some(max) = max_staff_level {
            query
                .push(" AND p.staff_mod_level <= ")
                .push_bind(max)
                .push(" AND NOT EXISTS (SELECT 1 FROM players t WHERE t.name_hash = c.to_name_hash AND t.staff_mod_level > ")
                .push_bind(max)
                .push(")");
        }
        query
            .push(if after.is_some() {
                " ORDER BY c.id LIMIT "
            } else {
                " ORDER BY c.id DESC LIMIT "
            })
            .push_bind(limit);
        query.build_query_as().fetch_all(&self.pool).await
    }

    /// Abuse reports newest first: all of them, or only those against the
    /// name `offender`, older than the report `before` when paging.
    pub async fn abuse_reports(
        &self,
        offender: Option<u64>,
        before: Option<i64>,
        limit: i64,
    ) -> Result<Vec<ReportRecord>, Error> {
        let mut query = QueryBuilder::<Postgres>::new(concat!(
            "SELECT s.id, r.display_name AS reporter, s.offender_name_hash, ",
            "o.display_name AS offender, s.reason, s.node_id, s.x, s.z, s.y, s.muted, ",
            ms!("s.created_at"),
            " AS at_ms FROM snapshots s JOIN players r ON r.id = s.reporter_id ",
            "LEFT JOIN players o ON o.name_hash = s.offender_name_hash WHERE true"
        ));
        if let Some(offender) = offender {
            query
                .push(" AND s.offender_name_hash = ")
                .push_bind(offender as i64);
        }
        if let Some(before) = before {
            query.push(" AND s.id < ").push_bind(before);
        }
        query.push(" ORDER BY s.id DESC LIMIT ").push_bind(limit);
        query.build_query_as().fetch_all(&self.pool).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_ranges_hold_every_longer_name() {
        // "ab" = 1 * 37 + 2; "abc" = 39 * 37 + 3.
        let ranges: Vec<_> = prefix_ranges(39).collect();
        assert_eq!(ranges.len(), 11);
        assert_eq!(ranges[0], (39, 39));
        assert_eq!(ranges[1], (39 * 37, 39 * 37 + 36));
        assert!(
            ranges
                .iter()
                .any(|&(lo, hi)| (lo..=hi).contains(&(39 * 37 + 3)))
        );
        assert!(ranges.windows(2).all(|w| w[0].1 < w[1].0));
        // A full-length name has only itself.
        let full = 37u64.pow(12) - 1;
        assert_eq!(
            prefix_ranges(full).collect::<Vec<_>>(),
            [(full as i64, full as i64)]
        );
    }
}
