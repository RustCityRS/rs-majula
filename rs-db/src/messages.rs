use crate::{Db, Error};
use sqlx::AssertSqlSafe;

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// A timestamp column as the website shows it: `20th October 2026, 18:04`
/// (UTC).
macro_rules! shown {
    ($column:literal) => {
        concat!(
            "to_char(",
            $column,
            " AT TIME ZONE 'UTC', 'FMDDth FMMonth YYYY, HH24:MI')"
        )
    };
}

/// A message from staff, as its player sees it: never who sent it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PlayerMessage {
    pub id: i64,
    pub subject: String,
    pub body: String,
    pub sent_date: String,
    pub read: bool,
}

/// A message from staff, as the console sees it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct StaffMessageRecord {
    pub id: i64,
    pub player: String,
    pub subject: String,
    pub body: String,
    pub staff_name: String,
    pub sent_ms: i64,
    pub read_ms: Option<i64>,
}

const PLAYER_MESSAGE: &str = concat!(
    "SELECT m.id, m.subject, m.body, ",
    shown!("m.created_at"),
    " AS sent_date, m.read_at IS NOT NULL AS read ",
    "FROM staff_messages m"
);

const STAFF_MESSAGE: &str = concat!(
    "SELECT m.id, p.display_name AS player, m.subject, m.body, m.staff_name, ",
    ms!("m.created_at"),
    " AS sent_ms, ",
    ms!("m.read_at"),
    " AS read_ms FROM staff_messages m JOIN players p ON p.id = m.player_id"
);

impl Db {
    /// The `limit` newest messages from staff to the player with row
    /// `player_id`, newest first.
    pub async fn messages_of(
        &self,
        player_id: i64,
        limit: i64,
    ) -> Result<Vec<PlayerMessage>, Error> {
        sqlx::query_as(AssertSqlSafe(format!(
            "{PLAYER_MESSAGE} WHERE m.player_id = $1 ORDER BY m.id DESC LIMIT $2"
        )))
        .bind(player_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Message `id` to the player with row `player_id`, marking it read.
    /// `None` if it isn't theirs.
    pub async fn open_message(
        &self,
        player_id: i64,
        id: i64,
    ) -> Result<Option<PlayerMessage>, Error> {
        let message: Option<PlayerMessage> = sqlx::query_as(AssertSqlSafe(format!(
            "{PLAYER_MESSAGE} WHERE m.id = $1 AND m.player_id = $2"
        )))
        .bind(id)
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await?;
        if message.as_ref().is_some_and(|m| !m.read) {
            sqlx::query(
                "UPDATE staff_messages SET read_at = now() WHERE id = $1 AND read_at IS NULL",
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        Ok(message)
    }

    /// Up to `limit` messages staff sent, newest first, before message
    /// `before` when paging, and only to the player `to_hash` if given.
    pub async fn staff_messages(
        &self,
        to_hash: Option<u64>,
        before: Option<i64>,
        limit: i64,
    ) -> Result<Vec<StaffMessageRecord>, Error> {
        sqlx::query_as(AssertSqlSafe(format!(
            "{STAFF_MESSAGE} WHERE ($1::bigint IS NULL OR p.name_hash = $1)
               AND ($2::bigint IS NULL OR m.id < $2)
             ORDER BY m.id DESC LIMIT $3"
        )))
        .bind(to_hash.map(|h| h as i64))
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Message `id`, as staff see it.
    pub async fn staff_message(&self, id: i64) -> Result<Option<StaffMessageRecord>, Error> {
        sqlx::query_as(AssertSqlSafe(format!("{STAFF_MESSAGE} WHERE m.id = $1")))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Sends the player `name_hash` a message from `staff_name`. `None` if
    /// there's no such player.
    pub async fn send_staff_message(
        &self,
        name_hash: u64,
        staff_name: &str,
        subject: &str,
        body: &str,
    ) -> Result<Option<i64>, Error> {
        sqlx::query_scalar(
            "INSERT INTO staff_messages (player_id, subject, body, staff_name)
             SELECT id, $2, $3, $4 FROM players WHERE name_hash = $1 RETURNING id",
        )
        .bind(name_hash as i64)
        .bind(subject)
        .bind(body)
        .bind(staff_name)
        .fetch_optional(&self.pool)
        .await
    }
}
