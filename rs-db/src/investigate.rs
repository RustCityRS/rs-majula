use crate::{Db, Error};

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// A player whose sessions the anti-cheat checks flagged.
#[derive(Clone, Debug, sqlx::FromRow)]
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
    pub online_node: Option<i16>,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
}

/// What an account did over a window, for spotting bot farms.
#[derive(Clone, Debug, sqlx::FromRow)]
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
    pub x: Option<i16>,
    pub z: Option<i16>,
    pub y: Option<i16>,
    pub online_node: Option<i16>,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
}

/// The mute and ban in force on `p`.
macro_rules! sanctions {
    () => {
        concat!(
            "CASE WHEN p.muted_until > now() THEN ",
            ms!("p.muted_until"),
            " END AS muted_until_ms, CASE WHEN p.banned_until > now() THEN ",
            ms!("p.banned_until"),
            " END AS banned_until_ms"
        )
    };
}

impl Db {
    /// The players with sessions started since `since_ms` (Unix ms) that the
    /// checks flagged, most flagged sessions first (then the highest scores
    /// summed), with each one's highest score in each of `buckets` equal
    /// stretches from then to now. With `max_staff_level`, no staff above it.
    pub async fn anticheat_leaders(
        &self,
        since_ms: i64,
        buckets: i32,
        max_staff_level: Option<i16>,
        limit: i64,
    ) -> Result<Vec<AnticheatLeader>, Error> {
        let since = since_ms as f64 / 1000.0;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(since, |d| d.as_secs_f64());
        let bucket_secs = ((now - since) / f64::from(buckets.max(1))).max(1.0);
        let sql = concat!(
            "WITH r AS (SELECT player_id, score, flags, duration_ms, started_at, ",
            "least($3 - 1, greatest(0, floor((extract(epoch FROM started_at)::float8 - $1) / $2)))::int AS b ",
            "FROM replays WHERE started_at >= to_timestamp($1) AND ($5::smallint IS NULL ",
            "OR player_id IN (SELECT id FROM players WHERE staff_mod_level <= $5))), ",
            "top AS (SELECT player_id, count(*) AS sessions, ",
            "count(*) FILTER (WHERE score > 0) AS flagged, max(score) AS max_score, ",
            "sum(score)::bigint AS total_score, sum(duration_ms)::bigint AS play_ms, ",
            "max(started_at) FILTER (WHERE score > 0) AS last_flagged ",
            "FROM r GROUP BY player_id HAVING max(score) > 0 ",
            "ORDER BY count(*) FILTER (WHERE score > 0) DESC, sum(score) DESC LIMIT $4) ",
            "SELECT p.display_name AS name, t.sessions, t.flagged, t.max_score, t.total_score, ",
            "t.play_ms, ",
            ms!("t.last_flagged"),
            " AS last_flagged_ms, ",
            "ARRAY(SELECT DISTINCT f FROM r, unnest(r.flags) f WHERE r.player_id = t.player_id ORDER BY f) AS flags, ",
            "ARRAY(SELECT coalesce(max(r.score), 0)::smallint FROM generate_series(0, $3 - 1) g ",
            "LEFT JOIN r ON r.player_id = t.player_id AND r.b = g GROUP BY g ORDER BY g) AS trend, ",
            "p.online_node, ",
            sanctions!(),
            " FROM top t JOIN players p ON p.id = t.player_id ",
            "ORDER BY t.flagged DESC, t.total_score DESC"
        );
        sqlx::query_as(sql)
            .bind(since)
            .bind(bucket_secs)
            .bind(buckets.max(1))
            .bind(limit)
            .bind(max_staff_level)
            .fetch_all(&self.pool)
            .await
    }

    /// Accounts that aren't staff, registered or logged in since `since_ms`
    /// (Unix ms), the most recently active first, with what they did since.
    pub async fn account_activity(
        &self,
        since_ms: i64,
        limit: i64,
    ) -> Result<Vec<AccountActivity>, Error> {
        let sql = concat!(
            "WITH recent AS (SELECT player_id, max(created_at) AS at FROM login_logs ",
            "WHERE created_at >= to_timestamp($1) GROUP BY player_id ",
            "UNION ALL SELECT id, created_at FROM players WHERE created_at >= to_timestamp($1)), ",
            "a AS (SELECT p.id, max(recent.at) AS active FROM recent ",
            "JOIN players p ON p.id = recent.player_id WHERE p.staff_mod_level = 0 ",
            "GROUP BY p.id ORDER BY active DESC LIMIT $2) ",
            "SELECT p.display_name AS name, ",
            ms!("p.created_at"),
            " AS registered_ms, host(p.registration_ip) AS registration_ip, ",
            "ARRAY(SELECT DISTINCT host(l.ip) FROM login_logs l ",
            "WHERE l.player_id = a.id AND l.created_at >= to_timestamp($1)) AS login_ips, ",
            "s.sessions, coalesce(s.play_ms, 0) AS play_ms, s.flagged, ",
            "coalesce(s.max_score, 0)::smallint AS max_score, s.rhythm, ",
            "ARRAY(SELECT count(r.id)::int FROM generate_series(0, 23) g LEFT JOIN replays r ",
            "ON r.player_id = a.id AND r.started_at >= to_timestamp($1) ",
            "AND extract(hour FROM r.started_at AT TIME ZONE 'UTC') = g ",
            "GROUP BY g ORDER BY g) AS hours, ",
            "ps.x, ps.z, ps.y, p.online_node, ",
            sanctions!(),
            " FROM a JOIN players p ON p.id = a.id ",
            "LEFT JOIN LATERAL (SELECT count(*) AS sessions, sum(duration_ms)::bigint AS play_ms, ",
            "count(*) FILTER (WHERE score > 0) AS flagged, max(score) AS max_score, ",
            "avg((stats::jsonb ->> 'clickRhythm')::float8) AS rhythm FROM replays ",
            "WHERE player_id = a.id AND started_at >= to_timestamp($1)) s ON true ",
            "LEFT JOIN player_saves ps ON ps.player_id = a.id ORDER BY a.active DESC"
        );
        sqlx::query_as(sql)
            .bind(since_ms as f64 / 1000.0)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }
}
