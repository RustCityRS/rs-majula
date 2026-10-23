use crate::counters::{self, COUNTERS};
use crate::watch::playback::decompress_chunk;
use crate::watch::{ReplayRequest, Report};
use rs_db::{Db, DbConfig, NewReplay, ReplayChunk};
use rustc_hash::FxHashSet;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::{Instant, MissedTickBehavior, interval_at};
use tracing::{debug, info, warn};

/// How often old recordings are pruned.
const PRUNE_EVERY: Duration = Duration::from_secs(60 * 60);
/// The first prune waits until the world is up (and its migrations ran).
const FIRST_PRUNE: Duration = Duration::from_secs(5 * 60);
/// The zstd level chunks are stored at.
const LEVEL: i32 = 3;
/// How often finished recordings are compacted: recompressed whole, into
/// one chunk, which takes about 40% off (chunks repeat each other, keyframes
/// most).
const COMPACT_EVERY: Duration = Duration::from_secs(5 * 60);
/// How long a recording has been over before it's compacted: staff watching
/// it live have read its last chunks by then.
const COMPACT_SETTLE_SECS: i32 = 5 * 60;
/// How many recordings to compact are looked up at a time.
const COMPACT_BATCH: i64 = 64;
/// The zstd level a finished recording is recompressed at.
const COMPACT_LEVEL: i32 = 19;

/// Stores what `rx` sends until every sender is gone, keeping recordings for
/// `days` days. Recorded at protocol revision `revision`.
pub async fn replay_task(
    config: DbConfig,
    node_id: u8,
    revision: &'static str,
    days: u32,
    mut rx: Receiver<ReplayRequest>,
) {
    let db = Db::connect_lazy(&config, 2);
    let compacting = tokio::spawn(compact_task(db.clone(), node_id));
    // Recordings that lost a chunk: they stop there.
    let mut broken = FxHashSet::default();
    let mut prune = interval_at(Instant::now() + FIRST_PRUNE, PRUNE_EVERY);
    prune.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            req = rx.recv() => {
                let Some(req) = req else { break };
                store(&db, node_id, revision, req, &mut broken).await;
            }
            _ = prune.tick() => match db.prune_replays(days as i32).await {
                Ok(0) => {}
                Ok(n) => info!("Pruned {n} session recordings older than {days} days"),
                Err(e) => warn!("Pruning session recordings failed: {e}"),
            },
        }
    }
    compacting.abort();
}

/// Compacts this world's finished recordings every [`COMPACT_EVERY`].
async fn compact_task(db: Db, node_id: u8) {
    let mut every = interval_at(Instant::now() + FIRST_PRUNE, COMPACT_EVERY);
    every.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        every.tick().await;
        let (mut count, mut saved) = (0, 0);
        'pass: loop {
            let ids = match db
                .replays_to_compact(node_id.into(), COMPACT_SETTLE_SECS, COMPACT_BATCH)
                .await
            {
                Ok(ids) => ids,
                Err(e) => {
                    warn!("Looking up session recordings to compact failed: {e}");
                    break;
                }
            };
            for &id in &ids {
                match compact(&db, id).await {
                    Ok(bytes) => {
                        count += 1;
                        saved += bytes;
                    }
                    Err(e) => {
                        warn!("Compacting session recording {id} failed: {e}");
                        break 'pass;
                    }
                }
            }
            if (ids.len() as i64) < COMPACT_BATCH {
                break;
            }
        }
        if count > 0 {
            debug!("Compacted {count} session recordings, {saved} bytes smaller");
        }
    }
}

/// Recompresses recording `id` whole, into one chunk. Returns how many
/// bytes that saved.
async fn compact(db: &Db, id: i64) -> Result<i64, rs_db::Error> {
    let chunks = db.replay_chunks(id).await?;
    let [first, .., last] = chunks.as_slice() else {
        db.skip_compacting(id).await?;
        return Ok(0);
    };
    let (start_ms, end_ms, last_seq) = (first.start_ms, last.end_ms, last.seq);
    let before: i64 = chunks.iter().map(|c| c.data.len() as i64).sum();
    let (data, whole) = tokio::task::spawn_blocking(move || {
        let mut raw = Vec::new();
        let mut whole = true;
        for chunk in &chunks {
            // Nothing after a chunk that doesn't decompress can play.
            let Some(mut data) = decompress_chunk(&chunk.data) else {
                whole = false;
                break;
            };
            raw.append(&mut data);
        }
        (zstd::bulk::compress(&raw, COMPACT_LEVEL).ok(), whole)
    })
    .await
    .unwrap_or_default();
    // Compressing into memory only fails on running out of it: try again
    // next time.
    let Some(data) = data else {
        return Ok(0);
    };
    if !whole {
        warn!("Session recording {id} has a chunk that doesn't decompress: compacted up to it");
    }
    let after = data.len() as i64;
    let chunk = ReplayChunk {
        seq: 0,
        start_ms,
        end_ms,
        data,
    };
    db.compact_replay(id, last_seq, &chunk).await?;
    Ok(before - after)
}

async fn store(
    db: &Db,
    node_id: u8,
    revision: &str,
    req: ReplayRequest,
    broken: &mut FxHashSet<i64>,
) {
    match req {
        ReplayRequest::Start { id, user37 } => {
            let replay = NewReplay {
                id,
                name_hash: user37,
                node_id: node_id.into(),
                revision: revision.to_owned(),
            };
            if let Err(e) = db.start_replay(&replay).await {
                warn!("Starting a session recording failed: {e}");
            }
        }
        ReplayRequest::Store {
            id,
            chunk,
            duration_ms,
            report,
            ended,
            complete,
        } => {
            let lost = broken.contains(&id);
            if ended {
                broken.remove(&id);
            }
            let chunk = chunk.filter(|_| !lost).map(|raw| ReplayChunk {
                seq: raw.seq,
                start_ms: raw.start_ms as i32,
                end_ms: raw.end_ms as i32,
                data: compress(&raw.data),
            });
            let result = db
                .store_replay(
                    id,
                    chunk.as_ref(),
                    duration_ms as i32,
                    report.map(Report::stored).as_ref(),
                    ended,
                    complete && !lost,
                )
                .await;
            match (result, chunk) {
                (Ok(()), Some(chunk)) => {
                    counters::add(&COUNTERS.replay_chunks, 1);
                    counters::add(&COUNTERS.replay_bytes, chunk.data.len() as u64);
                }
                (Ok(()), None) => {}
                (Err(e), chunk) => {
                    warn!("Storing a session recording failed: {e}");
                    if chunk.is_some() {
                        counters::add(&COUNTERS.replay_chunks_dropped, 1);
                        if !ended {
                            broken.insert(id);
                        }
                    }
                }
            }
        }
    }
}

fn compress(data: &[u8]) -> Vec<u8> {
    // Compressing into memory only fails on running out of it.
    zstd::bulk::compress(data, LEVEL).unwrap_or_default()
}
