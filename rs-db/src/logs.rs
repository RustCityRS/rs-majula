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

/// The `session_log` types the engine logs itself. The content's `^log_*`
/// constants (a script's `session_log`) are all below these.
pub mod log_type {
    /// A command a jagex moderator or maintainer typed (`::give`, `::tele`, ...),
    /// as they typed it.
    pub const STAFF_COMMAND: i16 = 100;
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
    /// Made out of nothing by a staff command (`::give` and the like).
    pub const SPAWNED: i16 = 105;
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

/// A script error a world gathered over a minute or so: how many times it
/// happened to one player, one kind of NPC or the world, and where the
/// first one was.
#[derive(Debug, Clone)]
pub struct ScriptErrorLog {
    /// When the first and last of them were (Unix ms).
    pub first_ms: i64,
    pub last_ms: i64,
    pub count: i32,
    /// The script, as `[trigger,subject]`, and where in it.
    pub script: String,
    pub file: String,
    pub line: i32,
    pub message: String,
    /// The scripts that called it, innermost first, a line each.
    pub trace: String,
    /// The player it ran on, if any.
    pub name_hash: Option<u64>,
    /// The NPC it ran on, if any: its debug name.
    pub npc: Option<String>,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    /// The player's recording, and how far into it the first one was.
    pub replay_id: Option<i64>,
    pub replay_ms: Option<i32>,
}

/// Logs gathered on a world, each kind in order.
#[derive(Debug, Default)]
pub struct LogBatch {
    pub chat: Vec<ChatLog>,
    pub sessions: Vec<SessionLog>,
    pub wealth: Vec<WealthEvent>,
    pub script_errors: Vec<ScriptErrorLog>,
}

impl LogBatch {
    pub fn is_empty(&self) -> bool {
        self.chat.is_empty()
            && self.sessions.is_empty()
            && self.wealth.is_empty()
            && self.script_errors.is_empty()
    }
}

/// Deletes up to `$2` of a table's rows whose `$at` is older than `$1`
/// days, oldest first, skipping the ones another world is deleting.
macro_rules! prune_oldest {
    ($table:literal, $at:literal, $keep:literal) => {
        concat!(
            "DELETE FROM ",
            $table,
            " WHERE id IN (SELECT id FROM ",
            $table,
            " WHERE ",
            $at,
            " < now() - make_interval(days => $1)",
            $keep,
            " ORDER BY ",
            $at,
            " LIMIT $2 FOR UPDATE SKIP LOCKED)"
        )
    };
}
pub(crate) use prune_oldest;

/// Deletes up to `$2` of a login pair table's rows (`login_ips`,
/// `login_uids` or `login_fingerprints`, keyed by `$key` and player) whose
/// latest login is older than `$1` days, like [`prune_oldest!`].
macro_rules! prune_pairs {
    ($table:literal, $key:literal) => {
        concat!(
            "DELETE FROM ",
            $table,
            " d USING (SELECT ",
            $key,
            ", player_id FROM ",
            $table,
            " WHERE last_at < now() - make_interval(days => $1) ORDER BY last_at",
            " LIMIT $2 FOR UPDATE SKIP LOCKED) o WHERE d.",
            $key,
            " = o.",
            $key,
            " AND d.player_id = o.player_id"
        )
    };
}

/// The logs pruned by age. Staff commands are kept: they're what staff did.
const PRUNED_LOGS: [&str; 3] = [
    prune_oldest!("chat_logs", "created_at", ""),
    prune_oldest!("session_logs", "created_at", " AND event_type <> 100"),
    prune_oldest!("wealth_events", "created_at", ""),
];
const PRUNED_LOGINS: &str = prune_oldest!("login_logs", "created_at", "");
const PRUNED_LOGIN_PAIRS: [&str; 3] = [
    prune_pairs!("login_ips", "ip"),
    prune_pairs!("login_uids", "uid"),
    prune_pairs!("login_fingerprints", "fingerprint"),
];
const PRUNED_SCRIPT_ERRORS: &str = prune_oldest!("script_errors", "last_at", "");
const _: () = assert!(log_type::STAFF_COMMAND == 100);

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
        if !logs.script_errors.is_empty() {
            insert_script_errors(&mut tx, node_id, &logs.script_errors).await?;
        }
        tx.commit().await
    }

    /// Deletes the chat, session logs and wealth events older than `days`,
    /// `batch` rows at a time so no one statement runs long. Returns how
    /// many.
    pub async fn prune_logs(&self, days: i32, batch: i64) -> Result<u64, Error> {
        let mut pruned = 0;
        for sql in PRUNED_LOGS {
            pruned += self.prune_oldest(sql, days, batch).await?;
        }
        Ok(pruned)
    }

    /// Forgets logins older than `days`, `batch` at a time, and the
    /// addresses and installs no login kept was from. Returns how many
    /// logins.
    pub async fn prune_login_logs(&self, days: i32, batch: i64) -> Result<u64, Error> {
        let pruned = self.prune_oldest(PRUNED_LOGINS, days, batch).await?;
        for sql in PRUNED_LOGIN_PAIRS {
            self.prune_oldest(sql, days, batch).await?;
        }
        Ok(pruned)
    }

    /// Deletes script errors last seen over `days` ago, `batch` at a time.
    /// Returns how many.
    pub async fn prune_script_errors(&self, days: i32, batch: i64) -> Result<u64, Error> {
        self.prune_oldest(PRUNED_SCRIPT_ERRORS, days, batch).await
    }

    /// Runs [`prune_oldest!`] statement `sql` until it deletes less than a
    /// whole batch.
    pub(crate) async fn prune_oldest(
        &self,
        sql: &'static str,
        days: i32,
        batch: i64,
    ) -> Result<u64, Error> {
        let mut pruned = 0;
        loop {
            let deleted = sqlx::query(sql)
                .bind(days)
                .bind(batch)
                .execute(&self.pool)
                .await?
                .rows_affected();
            pruned += deleted;
            if deleted < batch as u64 {
                return Ok(pruned);
            }
        }
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

async fn insert_script_errors(
    conn: &mut PgConnection,
    node_id: i16,
    logs: &[ScriptErrorLog],
) -> Result<(), Error> {
    let firsts: Vec<i64> = logs.iter().map(|l| l.first_ms).collect();
    let lasts: Vec<i64> = logs.iter().map(|l| l.last_ms).collect();
    let counts: Vec<i32> = logs.iter().map(|l| l.count).collect();
    let scripts: Vec<&str> = logs.iter().map(|l| l.script.as_str()).collect();
    let files: Vec<&str> = logs.iter().map(|l| l.file.as_str()).collect();
    let lines: Vec<i32> = logs.iter().map(|l| l.line).collect();
    let messages: Vec<&str> = logs.iter().map(|l| l.message.as_str()).collect();
    let traces: Vec<&str> = logs.iter().map(|l| l.trace.as_str()).collect();
    let name_hashes: Vec<Option<i64>> =
        logs.iter().map(|l| l.name_hash.map(|h| h as i64)).collect();
    let npcs: Vec<Option<&str>> = logs.iter().map(|l| l.npc.as_deref()).collect();
    let xs: Vec<i16> = logs.iter().map(|l| l.x).collect();
    let zs: Vec<i16> = logs.iter().map(|l| l.z).collect();
    let ys: Vec<i16> = logs.iter().map(|l| l.y).collect();
    let replay_ids: Vec<Option<i64>> = logs.iter().map(|l| l.replay_id).collect();
    let replay_ms: Vec<Option<i32>> = logs.iter().map(|l| l.replay_ms).collect();
    sqlx::query(
        "INSERT INTO script_errors (node_id, first_at, last_at, count, script, file, line,
                                    message, trace, player_id, npc, x, z, y, replay_id, replay_ms)
         SELECT $1, to_timestamp(s.first_ms / 1000.0), to_timestamp(s.last_ms / 1000.0), s.count,
                s.script, s.file, s.line, s.message, s.trace, p.id, s.npc, s.x, s.z, s.y,
                s.replay_id, s.replay_ms
         FROM unnest($2::bigint[], $3::bigint[], $4::int[], $5::text[], $6::text[], $7::int[],
                     $8::text[], $9::text[], $10::bigint[], $11::text[], $12::smallint[],
                     $13::smallint[], $14::smallint[], $15::bigint[], $16::int[])
              WITH ORDINALITY AS s (first_ms, last_ms, count, script, file, line, message, trace,
                                    name_hash, npc, x, z, y, replay_id, replay_ms, ord)
         LEFT JOIN players p ON p.name_hash = s.name_hash
         ORDER BY s.ord",
    )
    .bind(node_id)
    .bind(&firsts)
    .bind(&lasts)
    .bind(&counts)
    .bind(&scripts)
    .bind(&files)
    .bind(&lines)
    .bind(&messages)
    .bind(&traces)
    .bind(&name_hashes)
    .bind(&npcs)
    .bind(&xs)
    .bind(&zs)
    .bind(&ys)
    .bind(&replay_ids)
    .bind(&replay_ms)
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
