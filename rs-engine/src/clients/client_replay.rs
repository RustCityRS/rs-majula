//! The recording task: stores session recordings ([`crate::watch`]) on a
//! connection of its own, so chunks never hold up saves and logins, and
//! prunes recordings past their keeping time.

use crate::counters::{self, COUNTERS};
use crate::watch::ReplayRequest;
use flate2::Compression;
use flate2::write::DeflateEncoder;
use rs_db::{Db, DbConfig, NewReplay, ReplayChunk};
use rustc_hash::FxHashSet;
use std::io::Write;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::{Instant, MissedTickBehavior, interval_at};
use tracing::{info, warn};

/// How often old recordings are pruned.
const PRUNE_EVERY: Duration = Duration::from_secs(60 * 60);
/// The first prune waits until the world is up (and its migrations ran).
const FIRST_PRUNE: Duration = Duration::from_secs(5 * 60);

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
                data: deflate(&raw.data),
            });
            let result = db
                .store_replay(
                    id,
                    chunk.as_ref(),
                    duration_ms as i32,
                    &report,
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

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::with_capacity(data.len() / 4), Compression::fast());
    // Writing to a Vec can't fail.
    let _ = encoder.write_all(data);
    encoder.finish().unwrap_or_default()
}
