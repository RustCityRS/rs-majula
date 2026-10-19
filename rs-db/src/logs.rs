use crate::{Db, Error};
use sqlx::PgConnection;

/// A public or private chat message to log.
#[derive(Debug)]
pub struct ChatLog {
    pub name_hash: u64,
    /// The recipient of a private message, `None` for public chat.
    pub to_name_hash: Option<u64>,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub message: String,
}

/// A script's `session_log` event to log.
#[derive(Debug)]
pub struct SessionLog {
    pub name_hash: u64,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub event_type: i16,
    pub event: String,
}

/// A script's `wealth_event` to log.
#[derive(Debug)]
pub struct WealthEvent {
    pub name_hash: u64,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub event_type: i16,
    pub item: String,
    pub count: i32,
    pub value: i32,
}

/// Logs gathered on a world, each kind in order.
#[derive(Debug, Default)]
pub struct LogBatch {
    pub chat: Vec<ChatLog>,
    pub sessions: Vec<SessionLog>,
    pub wealth: Vec<WealthEvent>,
}

impl LogBatch {
    pub fn is_empty(&self) -> bool {
        self.chat.is_empty() && self.sessions.is_empty() && self.wealth.is_empty()
    }
}

impl Db {
    /// Records logs from world `node_id` in one transaction, keeping each
    /// kind's order. Logs of players who aren't registered are skipped.
    pub async fn record_logs(&self, node_id: i16, logs: &LogBatch) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        if !logs.chat.is_empty() {
            insert_chat(&mut tx, node_id, &logs.chat).await?;
        }
        if !logs.sessions.is_empty() {
            insert_sessions(&mut tx, node_id, &logs.sessions).await?;
        }
        if !logs.wealth.is_empty() {
            insert_wealth(&mut tx, node_id, &logs.wealth).await?;
        }
        tx.commit().await
    }
}

async fn insert_chat(conn: &mut PgConnection, node_id: i16, logs: &[ChatLog]) -> Result<(), Error> {
    let name_hashes: Vec<i64> = logs.iter().map(|l| l.name_hash as i64).collect();
    let to_name_hashes: Vec<Option<i64>> = logs
        .iter()
        .map(|l| l.to_name_hash.map(|h| h as i64))
        .collect();
    let xs: Vec<i16> = logs.iter().map(|l| l.x).collect();
    let zs: Vec<i16> = logs.iter().map(|l| l.z).collect();
    let ys: Vec<i16> = logs.iter().map(|l| l.y).collect();
    let messages: Vec<&str> = logs.iter().map(|l| l.message.as_str()).collect();
    sqlx::query(
        "INSERT INTO chat_logs (player_id, to_name_hash, node_id, x, z, y, message)
         SELECT p.id, c.to_name_hash, $1, c.x, c.z, c.y, c.message
         FROM unnest($2::bigint[], $3::bigint[], $4::smallint[], $5::smallint[],
                     $6::smallint[], $7::text[])
              WITH ORDINALITY AS c (name_hash, to_name_hash, x, z, y, message, ord)
         JOIN players p ON p.name_hash = c.name_hash
         ORDER BY c.ord",
    )
    .bind(node_id)
    .bind(&name_hashes)
    .bind(&to_name_hashes)
    .bind(&xs)
    .bind(&zs)
    .bind(&ys)
    .bind(&messages)
    .execute(conn)
    .await?;
    Ok(())
}

async fn insert_sessions(
    conn: &mut PgConnection,
    node_id: i16,
    logs: &[SessionLog],
) -> Result<(), Error> {
    let name_hashes: Vec<i64> = logs.iter().map(|l| l.name_hash as i64).collect();
    let xs: Vec<i16> = logs.iter().map(|l| l.x).collect();
    let zs: Vec<i16> = logs.iter().map(|l| l.z).collect();
    let ys: Vec<i16> = logs.iter().map(|l| l.y).collect();
    let event_types: Vec<i16> = logs.iter().map(|l| l.event_type).collect();
    let events: Vec<&str> = logs.iter().map(|l| l.event.as_str()).collect();
    sqlx::query(
        "INSERT INTO session_logs (player_id, node_id, x, z, y, event_type, event)
         SELECT p.id, $1, s.x, s.z, s.y, s.event_type, s.event
         FROM unnest($2::bigint[], $3::smallint[], $4::smallint[], $5::smallint[],
                     $6::smallint[], $7::text[])
              WITH ORDINALITY AS s (name_hash, x, z, y, event_type, event, ord)
         JOIN players p ON p.name_hash = s.name_hash
         ORDER BY s.ord",
    )
    .bind(node_id)
    .bind(&name_hashes)
    .bind(&xs)
    .bind(&zs)
    .bind(&ys)
    .bind(&event_types)
    .bind(&events)
    .execute(conn)
    .await?;
    Ok(())
}

async fn insert_wealth(
    conn: &mut PgConnection,
    node_id: i16,
    logs: &[WealthEvent],
) -> Result<(), Error> {
    let name_hashes: Vec<i64> = logs.iter().map(|l| l.name_hash as i64).collect();
    let xs: Vec<i16> = logs.iter().map(|l| l.x).collect();
    let zs: Vec<i16> = logs.iter().map(|l| l.z).collect();
    let ys: Vec<i16> = logs.iter().map(|l| l.y).collect();
    let event_types: Vec<i16> = logs.iter().map(|l| l.event_type).collect();
    let items: Vec<&str> = logs.iter().map(|l| l.item.as_str()).collect();
    let counts: Vec<i32> = logs.iter().map(|l| l.count).collect();
    let values: Vec<i32> = logs.iter().map(|l| l.value).collect();
    sqlx::query(
        "INSERT INTO wealth_events (player_id, node_id, x, z, y, event_type, item, count, value)
         SELECT p.id, $1, w.x, w.z, w.y, w.event_type, w.item, w.count, w.value
         FROM unnest($2::bigint[], $3::smallint[], $4::smallint[], $5::smallint[],
                     $6::smallint[], $7::text[], $8::int[], $9::int[])
              WITH ORDINALITY AS w (name_hash, x, z, y, event_type, item, count, value, ord)
         JOIN players p ON p.name_hash = w.name_hash
         ORDER BY w.ord",
    )
    .bind(node_id)
    .bind(&name_hashes)
    .bind(&xs)
    .bind(&zs)
    .bind(&ys)
    .bind(&event_types)
    .bind(&items)
    .bind(&counts)
    .bind(&values)
    .execute(conn)
    .await?;
    Ok(())
}
