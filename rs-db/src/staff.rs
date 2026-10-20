use crate::{Db, Error};
use sqlx::{Postgres, QueryBuilder};

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

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

impl Db {
    /// The player with base37 name `name_hash`, if registered.
    pub async fn player_record(&self, name_hash: u64) -> Result<Option<PlayerRecord>, Error> {
        sqlx::query_as(concat!(
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
            " AS last_logout_ms FROM players WHERE name_hash = $1"
        ))
        .bind(name_hash as i64)
        .fetch_optional(&self.pool)
        .await
    }

    /// Moderation commands newest first, all of them or only those on the
    /// player `target`, older than the entry `before` when paging.
    pub async fn moderation_history(
        &self,
        target: Option<u64>,
        before: Option<i64>,
        limit: i64,
    ) -> Result<Vec<ModerationRecord>, Error> {
        let mut query = QueryBuilder::<Postgres>::new(concat!(
            "SELECT m.id, mp.display_name AS moderator, tp.display_name AS target, m.action, ",
            "m.hours, m.node_id, ",
            ms!("m.created_at"),
            " AS at_ms FROM moderation_logs m ",
            "JOIN players mp ON mp.id = m.moderator_id ",
            "JOIN players tp ON tp.id = m.target_id WHERE true"
        ));
        if let Some(target) = target {
            query
                .push(" AND m.target_id = (SELECT id FROM players WHERE name_hash = ")
                .push_bind(target as i64)
                .push(")");
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
    /// paging.
    pub async fn chat_history(
        &self,
        sender: Option<u64>,
        node_id: Option<i16>,
        before: Option<i64>,
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
        query.push(" ORDER BY c.id DESC LIMIT ").push_bind(limit);
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
