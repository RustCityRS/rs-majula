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

/// The wealth event types the engine logs itself, as items pass between
/// players or through the floor. The content's `^wealth_*` constants (a
/// script's `wealth_event`) are all below these.
pub mod transfer {
    /// Handed over in a trade: to the event's counterpart.
    pub const TRADE: i16 = 100;
    /// Lost as a duel stake, to the winner.
    pub const STAKE: i16 = 101;
    /// Handed straight to another player some other way.
    pub const GIVE: i16 = 102;
    /// Dropped on the floor (death drops too).
    pub const DROP: i16 = 103;
    /// Picked up from the floor.
    pub const TAKE: i16 = 104;
    /// The types that hand items to a counterpart.
    pub const HANDOVERS: [i16; 3] = [TRADE, STAKE, GIVE];
}

/// A script's `wealth_event`, or one of the engine's [`transfer`]s, to log.
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
    /// Who a handover went to.
    pub counterpart: Option<u64>,
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

    /// How many `session_log` events a player's scripts logged in each hour
    /// (as Unix ms, UTC) of the last `days`, for the hours with any.
    pub async fn activity_hours(
        &self,
        name_hash: u64,
        days: i32,
    ) -> Result<Vec<(i64, i64)>, Error> {
        sqlx::query_as(
            "SELECT (extract(epoch FROM date_trunc('hour', l.created_at AT TIME ZONE 'UTC')) * 1000)::bigint,
                    count(*)
             FROM session_logs l JOIN players p ON p.id = l.player_id
             WHERE p.name_hash = $1 AND l.created_at > now() - make_interval(days => $2)
             GROUP BY 1 ORDER BY 1",
        )
        .bind(name_hash as i64)
        .bind(days)
        .fetch_all(&self.pool)
        .await
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
    let counterparts: Vec<Option<i64>> = logs
        .iter()
        .map(|l| l.counterpart.map(|h| h as i64))
        .collect();
    sqlx::query(
        "INSERT INTO wealth_events
             (player_id, node_id, x, z, y, event_type, item, count, value, counterpart_id)
         SELECT p.id, $1, w.x, w.z, w.y, w.event_type, w.item, w.count, w.value, c.id
         FROM unnest($2::bigint[], $3::smallint[], $4::smallint[], $5::smallint[],
                     $6::smallint[], $7::text[], $8::int[], $9::int[], $10::bigint[])
              WITH ORDINALITY AS w (name_hash, x, z, y, event_type, item, count, value,
                                    counterpart, ord)
         JOIN players p ON p.name_hash = w.name_hash
         LEFT JOIN players c ON c.name_hash = w.counterpart
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
    .bind(&counterparts)
    .execute(conn)
    .await?;
    Ok(())
}
