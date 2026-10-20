//! Session recordings: what a world sent a player and what their client sent
//! back, stored a chunk at a time while they play, for moderators to replay
//! in the staff console (see migration 0011).

use crate::{Db, Error};
use sqlx::{Postgres, QueryBuilder};

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// A session a world started recording.
#[derive(Debug)]
pub struct NewReplay {
    pub id: i64,
    pub name_hash: u64,
    pub node_id: i16,
    pub revision: String,
}

/// The anti-cheat checks' verdict on a session so far.
#[derive(Clone, Debug, Default)]
pub struct ReplayReport {
    pub score: i16,
    pub flags: Vec<String>,
    pub stats: String,
}

/// A chunk of a recording, deflated.
#[derive(Debug)]
pub struct ReplayChunk {
    pub seq: i32,
    pub start_ms: i32,
    pub end_ms: i32,
    pub data: Vec<u8>,
}

/// A recorded session, as staff see it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ReplayRecord {
    pub id: i64,
    pub player: String,
    pub name_hash: i64,
    pub node_id: i16,
    pub revision: String,
    pub started_ms: i64,
    pub ended_ms: Option<i64>,
    pub updated_ms: i64,
    pub duration_ms: i32,
    pub bytes: i64,
    pub complete: bool,
    pub score: i16,
    pub flags: Vec<String>,
    pub stats: String,
}

const RECORD_COLUMNS: &str = concat!(
    "SELECT r.id, p.display_name AS player, p.name_hash, r.node_id, r.revision, ",
    ms!("r.started_at"),
    " AS started_ms, ",
    ms!("r.ended_at"),
    " AS ended_ms, ",
    ms!("r.updated_at"),
    " AS updated_ms, r.duration_ms, r.bytes, r.complete, r.score, r.flags, r.stats ",
    "FROM replays r JOIN players p ON p.id = r.player_id"
);

impl Db {
    /// Starts a recording. Returns `false` if the player isn't registered,
    /// and nothing is recorded.
    pub async fn start_replay(&self, replay: &NewReplay) -> Result<bool, Error> {
        let result = sqlx::query(
            "INSERT INTO replays (id, player_id, node_id, revision)
             SELECT $1, id, $3, $4 FROM players WHERE name_hash = $2
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(replay.id)
        .bind(replay.name_hash as i64)
        .bind(replay.node_id)
        .bind(&replay.revision)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Adds `chunk` (if any) to recording `id` with the checks' verdict so
    /// far. `ended` marks the session over; `complete: false` that a chunk
    /// before this one was lost.
    pub async fn store_replay(
        &self,
        id: i64,
        chunk: Option<&ReplayChunk>,
        duration_ms: i32,
        report: &ReplayReport,
        ended: bool,
        complete: bool,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        let mut bytes = 0i64;
        if let Some(chunk) = chunk {
            sqlx::query(
                "INSERT INTO replay_chunks (replay_id, seq, start_ms, end_ms, data)
                 SELECT $1, $2, $3, $4, $5 WHERE EXISTS (SELECT 1 FROM replays WHERE id = $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(chunk.seq)
            .bind(chunk.start_ms)
            .bind(chunk.end_ms)
            .bind(&chunk.data)
            .execute(&mut *tx)
            .await?;
            bytes = chunk.data.len() as i64;
        }
        sqlx::query(
            "UPDATE replays SET duration_ms = GREATEST(duration_ms, $2), bytes = bytes + $3,
                 score = $4, flags = $5, stats = $6, complete = complete AND $7,
                 ended_at = CASE WHEN $8 THEN now() ELSE ended_at END, updated_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(duration_ms)
        .bind(bytes)
        .bind(report.score)
        .bind(&report.flags)
        .bind(&report.stats)
        .bind(complete)
        .bind(ended)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// Deletes recordings started more than `days` days ago. Returns how many.
    pub async fn prune_replays(&self, days: i32) -> Result<u64, Error> {
        let result =
            sqlx::query("DELETE FROM replays WHERE started_at < now() - make_interval(days => $1)")
                .bind(days)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected())
    }

    /// Recordings newest first: everyone's or only `player`'s, only flagged
    /// ones (most suspicious first) if `flagged`, started before `before`
    /// (Unix ms) when paging; with `max_staff_level`, none of staff above it.
    pub async fn replays(
        &self,
        player: Option<u64>,
        flagged: bool,
        before: Option<i64>,
        max_staff_level: Option<i16>,
        limit: i64,
    ) -> Result<Vec<ReplayRecord>, Error> {
        let mut query = QueryBuilder::<Postgres>::new(RECORD_COLUMNS);
        query.push(" WHERE true");
        if let Some(max) = max_staff_level {
            query.push(" AND p.staff_mod_level <= ").push_bind(max);
        }
        if let Some(player) = player {
            query
                .push(" AND r.player_id = (SELECT id FROM players WHERE name_hash = ")
                .push_bind(player as i64)
                .push(")");
        }
        if flagged {
            query.push(" AND r.score > 0");
        }
        if let Some(before) = before {
            query
                .push(" AND r.started_at < to_timestamp(")
                .push_bind(before as f64 / 1000.0)
                .push(")");
        }
        query
            .push(" ORDER BY r.started_at DESC LIMIT ")
            .push_bind(limit);
        query.build_query_as().fetch_all(&self.pool).await
    }

    /// Recording `id`, if it's still kept.
    pub async fn replay(&self, id: i64) -> Result<Option<ReplayRecord>, Error> {
        let mut query = QueryBuilder::<Postgres>::new(RECORD_COLUMNS);
        query.push(" WHERE r.id = ").push_bind(id);
        query.build_query_as().fetch_optional(&self.pool).await
    }

    /// Recording `id`'s chunks in order, deflated.
    pub async fn replay_chunks(&self, id: i64) -> Result<Vec<ReplayChunk>, Error> {
        self.replay_chunks_from(id, 0).await
    }

    /// Recording `id`'s chunks from number `seq` on, in order. A world
    /// stores a recording's chunks one after another, so once one is here
    /// every one before it is (or was lost).
    pub async fn replay_chunks_from(&self, id: i64, seq: i32) -> Result<Vec<ReplayChunk>, Error> {
        let rows: Vec<(i32, i32, i32, Vec<u8>)> = sqlx::query_as(
            "SELECT seq, start_ms, end_ms, data FROM replay_chunks WHERE replay_id = $1 AND seq >= $2 ORDER BY seq",
        )
        .bind(id)
        .bind(seq)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(seq, start_ms, end_ms, data)| ReplayChunk {
                seq,
                start_ms,
                end_ms,
                data,
            })
            .collect())
    }
}
