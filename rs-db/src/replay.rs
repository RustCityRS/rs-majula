use crate::logs::prune_oldest;
use crate::{Db, Error};
use sqlx::{Postgres, QueryBuilder};
use std::slice;

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

/// A chunk of a recording, compressed (zstd; deflate before 2026-10-21). A
/// finished recording is compacted into one chunk, number 0.
#[derive(Clone, Debug)]
pub struct ReplayChunk {
    pub seq: i32,
    pub start_ms: i32,
    pub end_ms: i32,
    pub data: Vec<u8>,
}

/// What a world stores to one of its recordings at once (see
/// [`Db::store_replays`]).
#[derive(Debug, Default)]
pub struct ReplayStore {
    pub id: i64,
    /// Its new chunks.
    pub chunks: Vec<ReplayChunk>,
    pub duration_ms: i32,
    /// The checks' verdict so far, if one was due (else the last stays).
    pub report: Option<ReplayReport>,
    /// The session is over.
    pub ended: bool,
    /// `false`: a chunk before these was lost.
    pub complete: bool,
}

/// Between a report's flags in one text: no flag has it.
const FLAG_SEPARATOR: &str = "\u{1f}";

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

/// A session the player played, for judging when they play (see the
/// world's anti-cheat schedule check).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PlayedSession {
    pub started_ms: i64,
    pub duration_ms: i32,
    /// How it ended (`logout`, `idle`, `disconnect`, `kick`, `reboot`), as
    /// the world noted in its stats.
    pub end_reason: Option<String>,
    /// How long after the session before it was ended for it (not by the
    /// player) this one began.
    pub return_ms: Option<i32>,
}

/// How much the recordings take up.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ReplayStorage {
    pub recordings: i64,
    pub bytes: i64,
    pub disk_bytes: i64,
    pub oldest_ms: Option<i64>,
    pub play_ms: i64,
    pub day_recordings: i64,
    pub day_bytes: i64,
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

const PRUNED_REPLAYS: &str = prune_oldest!("replays", "started_at", "");

impl Db {
    /// Starts a recording. Returns `false` if the player isn't registered,
    /// and nothing is recorded.
    pub async fn start_replay(&self, replay: &NewReplay) -> Result<bool, Error> {
        Ok(self.start_replays(slice::from_ref(replay)).await? == 1)
    }

    /// Starts recordings, in one statement. Those of players who aren't
    /// registered aren't. Returns how many were.
    pub async fn start_replays(&self, replays: &[NewReplay]) -> Result<u64, Error> {
        let ids: Vec<i64> = replays.iter().map(|r| r.id).collect();
        let hashes: Vec<i64> = replays.iter().map(|r| r.name_hash as i64).collect();
        let nodes: Vec<i16> = replays.iter().map(|r| r.node_id).collect();
        let revisions: Vec<&str> = replays.iter().map(|r| r.revision.as_str()).collect();
        let result = sqlx::query(
            "INSERT INTO replays (id, player_id, node_id, revision)
             SELECT r.id, p.id, r.node_id, r.revision
             FROM unnest($1::bigint[], $2::bigint[], $3::smallint[], $4::text[])
                 r (id, name_hash, node_id, revision)
             JOIN players p ON p.name_hash = r.name_hash
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&ids)
        .bind(&hashes)
        .bind(&nodes)
        .bind(&revisions)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// The newest `limit` sessions player `name_hash` started since
    /// `since_ms` (Unix ms), oldest first.
    pub async fn played_since(
        &self,
        name_hash: u64,
        since_ms: i64,
        limit: i64,
    ) -> Result<Vec<PlayedSession>, Error> {
        let sql = concat!(
            "SELECT * FROM (SELECT ",
            ms!("r.started_at"),
            " AS started_ms, r.duration_ms, s.stats ->> 'endReason' AS end_reason, ",
            "(s.stats ->> 'returnMs')::int AS return_ms FROM replays r ",
            // Parsed once a row (`OFFSET 0` keeps it from being inlined).
            "CROSS JOIN LATERAL (SELECT r.stats::jsonb AS stats OFFSET 0) s ",
            "WHERE r.player_id = (SELECT id FROM players WHERE name_hash = $1) ",
            "AND r.started_at >= to_timestamp($2) ORDER BY r.started_at DESC LIMIT $3) p ",
            "ORDER BY started_ms"
        );
        sqlx::query_as(sql)
            .bind(name_hash as i64)
            .bind(since_ms as f64 / 1000.0)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// Adds `chunk` (if any) to recording `id`, with the checks' verdict so
    /// far if given (else the last stays). `ended` marks the session over;
    /// `complete: false` that a chunk before this one was lost.
    pub async fn store_replay(
        &self,
        id: i64,
        chunk: Option<&ReplayChunk>,
        duration_ms: i32,
        report: Option<&ReplayReport>,
        ended: bool,
        complete: bool,
    ) -> Result<(), Error> {
        self.store_replays(&[ReplayStore {
            id,
            chunks: chunk.into_iter().cloned().collect(),
            duration_ms,
            report: report.cloned(),
            ended,
            complete,
        }])
        .await
    }

    /// Stores to recordings, one [`ReplayStore`] each, in one transaction
    /// of two statements. A chunk stored after its recording was compacted
    /// gets it compacted again; one for a recording that wasn't started is
    /// dropped.
    pub async fn store_replays(&self, stores: &[ReplayStore]) -> Result<(), Error> {
        let chunks = || {
            stores
                .iter()
                .flat_map(|s| s.chunks.iter().map(move |c| (s.id, c)))
        };
        let chunk_ids: Vec<i64> = chunks().map(|(id, _)| id).collect();
        let seqs: Vec<i32> = chunks().map(|(_, c)| c.seq).collect();
        let starts: Vec<i32> = chunks().map(|(_, c)| c.start_ms).collect();
        let ends: Vec<i32> = chunks().map(|(_, c)| c.end_ms).collect();
        let data: Vec<&[u8]> = chunks().map(|(_, c)| c.data.as_slice()).collect();

        let ids: Vec<i64> = stores.iter().map(|s| s.id).collect();
        let durations: Vec<i32> = stores.iter().map(|s| s.duration_ms).collect();
        let bytes: Vec<i64> = stores
            .iter()
            .map(|s| s.chunks.iter().map(|c| c.data.len() as i64).sum())
            .collect();
        let scores: Vec<Option<i16>> = stores
            .iter()
            .map(|s| s.report.as_ref().map(|r| r.score))
            .collect();
        let flags: Vec<Option<String>> = stores
            .iter()
            .map(|s| s.report.as_ref().map(|r| r.flags.join(FLAG_SEPARATOR)))
            .collect();
        let stats: Vec<Option<&str>> = stores
            .iter()
            .map(|s| s.report.as_ref().map(|r| r.stats.as_str()))
            .collect();
        let complete: Vec<bool> = stores.iter().map(|s| s.complete).collect();
        let ended: Vec<bool> = stores.iter().map(|s| s.ended).collect();
        let chunked: Vec<bool> = stores.iter().map(|s| !s.chunks.is_empty()).collect();

        let mut tx = self.pool.begin().await?;
        if !chunk_ids.is_empty() {
            sqlx::query(
                "INSERT INTO replay_chunks (replay_id, seq, start_ms, end_ms, data)
                 SELECT c.* FROM unnest($1::bigint[], $2::int[], $3::int[], $4::int[], $5::bytea[])
                     c (replay_id, seq, start_ms, end_ms, data)
                 WHERE EXISTS (SELECT 1 FROM replays WHERE id = c.replay_id)
                 ON CONFLICT DO NOTHING",
            )
            .bind(&chunk_ids)
            .bind(&seqs)
            .bind(&starts)
            .bind(&ends)
            .bind(&data)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "UPDATE replays r SET duration_ms = GREATEST(r.duration_ms, u.duration_ms),
                 bytes = r.bytes + u.bytes, score = coalesce(u.score, r.score),
                 flags = coalesce(string_to_array(u.flags, chr(31)), r.flags),
                 stats = coalesce(u.stats, r.stats), complete = r.complete AND u.complete,
                 ended_at = CASE WHEN u.ended THEN now() ELSE r.ended_at END, updated_at = now(),
                 compacted = r.compacted AND NOT u.chunked
             FROM unnest($1::bigint[], $2::int[], $3::bigint[], $4::smallint[], $5::text[],
                     $6::text[], $7::bool[], $8::bool[], $9::bool[])
                 u (id, duration_ms, bytes, score, flags, stats, complete, ended, chunked)
             WHERE r.id = u.id",
        )
        .bind(&ids)
        .bind(&durations)
        .bind(&bytes)
        .bind(&scores)
        .bind(&flags)
        .bind(&stats)
        .bind(&complete)
        .bind(&ended)
        .bind(&chunked)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// Up to `limit` of world `node_id`'s recordings to compact: ended more
    /// than `settle_secs` ago (or not stored to in an hour, the world having
    /// stopped without ending them) and not compacted yet.
    pub async fn replays_to_compact(
        &self,
        node_id: i16,
        settle_secs: i32,
        limit: i64,
    ) -> Result<Vec<i64>, Error> {
        sqlx::query_scalar(
            "SELECT r.id FROM replays r
             WHERE r.node_id = $1 AND NOT r.compacted
               AND (r.ended_at < now() - make_interval(secs => $2)
                    OR (r.ended_at IS NULL AND r.updated_at < now() - interval '1 hour'))
             ORDER BY r.id LIMIT $3",
        )
        .bind(node_id)
        .bind(f64::from(settle_secs))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Replaces recording `id`'s chunks up to number `last_seq` with `chunk`
    /// (number 0): the whole recording, compressed in one go.
    pub async fn compact_replay(
        &self,
        id: i64,
        last_seq: i32,
        chunk: &ReplayChunk,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        let freed: Option<i64> = sqlx::query_scalar(
            "WITH gone AS (DELETE FROM replay_chunks WHERE replay_id = $1 AND seq <= $2 RETURNING data)
             SELECT sum(length(data))::bigint FROM gone",
        )
        .bind(id)
        .bind(last_seq)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO replay_chunks (replay_id, seq, start_ms, end_ms, data)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(chunk.seq)
        .bind(chunk.start_ms)
        .bind(chunk.end_ms)
        .bind(&chunk.data)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE replays SET bytes = GREATEST(bytes - $2 + $3, 0), compacted = true WHERE id = $1",
        )
            .bind(id)
            .bind(freed.unwrap_or(0))
            .bind(chunk.data.len() as i64)
            .execute(&mut *tx)
            .await?;
        tx.commit().await
    }

    /// Marks recording `id` compacted as it is: it's in one chunk already
    /// (or none, every one lost).
    pub async fn skip_compacting(&self, id: i64) -> Result<(), Error> {
        sqlx::query("UPDATE replays SET compacted = true WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// How much the recordings take up.
    pub async fn replay_storage(&self) -> Result<ReplayStorage, Error> {
        sqlx::query_as(concat!(
            "SELECT count(*) AS recordings, coalesce(sum(bytes), 0)::bigint AS bytes, ",
            "(pg_total_relation_size('replays') + pg_total_relation_size('replay_chunks'))::bigint AS disk_bytes, ",
            ms!("min(started_at)"),
            " AS oldest_ms, coalesce(sum(duration_ms), 0)::bigint AS play_ms, count(*) FILTER (WHERE started_at > now() - interval '1 day') AS day_recordings, ",
            "coalesce(sum(bytes) FILTER (WHERE started_at > now() - interval '1 day'), 0)::bigint AS day_bytes ",
            "FROM replays"
        ))
        .fetch_one(&self.pool)
        .await
    }

    /// Deletes recordings started more than `days` days ago, `batch` at a
    /// time (their chunks go with them). Returns how many.
    pub async fn prune_replays(&self, days: i32, batch: i64) -> Result<u64, Error> {
        self.prune_oldest(PRUNED_REPLAYS, days, batch).await
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

    /// Recording `id`'s chunks in order, compressed.
    pub async fn replay_chunks(&self, id: i64) -> Result<Vec<ReplayChunk>, Error> {
        self.replay_chunks_from(id, 0).await
    }

    /// Recording `id`'s chunks from number `seq` on, in order. A world
    /// stores a recording's chunks one after another, so once one is here
    /// every one before it is (or was lost).
    pub async fn replay_chunks_from(&self, id: i64, seq: i32) -> Result<Vec<ReplayChunk>, Error> {
        self.replay_chunks_page(id, seq, i64::MAX).await
    }

    /// Like [`Self::replay_chunks_from`], the first `limit` of them.
    pub async fn replay_chunks_page(
        &self,
        id: i64,
        seq: i32,
        limit: i64,
    ) -> Result<Vec<ReplayChunk>, Error> {
        let rows: Vec<(i32, i32, i32, Vec<u8>)> = sqlx::query_as(
            "SELECT seq, start_ms, end_ms, data FROM replay_chunks WHERE replay_id = $1 AND seq >= $2 ORDER BY seq LIMIT $3",
        )
        .bind(id)
        .bind(seq)
        .bind(limit)
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
