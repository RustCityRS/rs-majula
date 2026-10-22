use crate::counters::{self, COUNTERS, mod_action_index};
use crate::phases::login::{LOGIN_FAILURE_WINDOW_SECS, MAX_LOGIN_FAILURES};
use crate::player_save::{
    PlayerProfile, PlayerProfileInv, delete_save_file, load_binary, load_from_file,
    orphan_save_file, save_orphan, save_to_file,
};
use rs_db::{
    Claim, Db, DbConfig, LogBatch, LoginDevice, ModAction, NewPlayer, NewSnapshot, Passwords,
    PlayedSession, PlayerSave, SaveResult, check_new_password,
};
use rs_util::base37::{to_raw_username, to_screen_name, to_userhash};
use std::net::IpAddr;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep, timeout};
use tracing::{error, info, warn};

/// Seconds without a heartbeat after which a world counts as dead and
/// another world may take over the sessions it owns. A dead world's saves
/// are refused once its sessions are taken, so its stale copies can never
/// overwrite the new owner's.
pub const TAKEOVER_AFTER_SECS: i32 = 60;

/// How often the lease task renews this world's heartbeat, on a connection
/// of its own so a backlog of saves and logins can't delay it.
const LEASE_INTERVAL: Duration = Duration::from_secs(5);

/// How long the engine plays on without a confirmed heartbeat before it
/// fences itself (removes every player). Half the takeover time: a world
/// cut off from the database always stops serving its players well before
/// another world may take them over, so a player can never be live in two
/// worlds at once.
pub const DB_FENCE: Duration = Duration::from_secs(30);

const _: () = assert!(DB_FENCE.as_secs() * 2 <= TAKEOVER_AFTER_SECS as u64);

/// Settings for [`db_client_task`].
pub struct DbClientConfig {
    pub db: DbConfig,
    pub node_id: u8,
    pub pepper: String,
    pub auto_register: bool,
    pub auto_migrate: bool,
}

/// A request sent from the engine to the database client task.
pub enum DbRequest {
    Authenticate {
        user37: u64,
        password: Box<str>,
        ip: IpAddr,
        uid: Option<i32>,
        device: Box<LoginDevice>,
    },
    Save {
        user37: u64,
        username: String,
        profile: Box<PlayerProfile>,
        binary: Vec<u8>,
        logout: bool,
    },
    Load {
        user37: u64,
    },
    Release {
        user37: u64,
    },
    WorldStatus {
        players: u32,
        maintenance: bool,
        online: Vec<u64>,
        held: Vec<u64>,
    },
    Snapshot {
        reporter37: u64,
        offender37: u64,
        reason: u8,
        x: u16,
        z: u16,
        y: u8,
        mute_by: Option<u8>,
    },
    Ban {
        user37: u64,
        hours: i32,
    },
    Moderate {
        moderator37: u64,
        by_level: u8,
        target37: u64,
        action: ModAction,
        reason: Option<String>,
        reply: Option<oneshot::Sender<ModOutcome>>,
    },
    Logs {
        logs: LogBatch,
    },
    EditLoad {
        user37: u64,
        reply: oneshot::Sender<LoadResult>,
    },
    EditStore {
        user37: u64,
        profile: Box<PlayerProfile>,
        /// Who changed the save, and how, for its history.
        note: String,
        reply: oneshot::Sender<SaveOutcome>,
    },
    /// Stores the player's local save file now, as a reconnect would.
    SyncLocal {
        user37: u64,
        reply: oneshot::Sender<SyncOutcome>,
    },
}

/// How a [`DbRequest::SyncLocal`] went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncOutcome {
    /// Stored, and the file removed.
    Synced,
    /// There's no local file for them.
    NoFile,
    /// The session isn't this world's, so the file is stale: moved aside.
    Orphaned,
    /// Not stored; the file is kept.
    Failed,
}

/// How long a moderator's in-game mute lasts, as the client's abuse report
/// promises ("Mute player for 48 hours").
pub const MUTE_HOURS: i32 = 48;

/// How long the automated ban lasts for sending what only a modified client
/// can.
pub const AUTO_BAN_HOURS: i32 = 48;

/// How long logins (and the addresses they came from) are kept, for staff
/// to link accounts by.
const LOGIN_LOG_DAYS: i32 = 180;

/// How long earlier saves are kept to roll players back to, and how many of
/// each player's newest are kept however old, and how often they're pruned.
const SAVE_HISTORY_DAYS: i32 = 30;
const SAVE_HISTORY_KEEP: i64 = 5;
const SAVE_HISTORY_PRUNE: Duration = Duration::from_secs(3600);

/// The outcome of a [`DbRequest::Authenticate`].
pub enum AuthResult {
    Accepted {
        staff_mod_level: u8,
        muted_for: Option<Duration>,
        left_other_world: Option<Duration>,
        /// How long ago the account was registered.
        account_age: Duration,
        /// The sessions it played over the last week, for the anti-cheat's
        /// schedule check.
        played: Vec<PlayedSession>,
    },
    InvalidCredentials,
    Banned,
    TooManyAttempts,
}

/// Where a [`DbRequest::Save`] ended up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Stored,
    Unregistered,
    NotOwner,
    Failed,
}

/// The outcome of a [`DbRequest::Load`].
pub enum LoadResult {
    Loaded(Option<Box<PlayerProfile>>),
    Busy,
    Unregistered,
}

/// A response sent from the database client task back to the engine.
pub enum DbResponse {
    DbReady,
    DbDisconnected,
    AuthResponse {
        user37: u64,
        result: AuthResult,
    },
    SaveAck {
        user37: u64,
        username: String,
        outcome: SaveOutcome,
        /// Echoes [`DbRequest::Save::logout`].
        logout: bool,
    },
    LoadResponse {
        user37: u64,
        result: LoadResult,
    },
    SessionsLost {
        users37: Vec<u64>,
    },
    Lease {
        at: Instant,
    },
    /// A [`DbRequest::Mute`] took: the player is muted for `remaining`.
    Muted {
        user37: u64,
        remaining: Duration,
    },
    /// The mute and ban of each online player, as of a
    /// [`DbRequest::WorldStatus`]: how long is left on a mute (`None` while not
    /// muted), and whether they are banned. Carries what was done elsewhere
    /// (another world, or by hand in the database).
    Sanctions {
        players: Vec<(u64, Option<Duration>, bool)>,
    },
    /// How a [`DbRequest::Moderate`] went.
    Moderated {
        moderator37: u64,
        target37: u64,
        action: ModAction,
        outcome: ModOutcome,
    },
}

/// How a [`DbRequest::Moderate`] went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModOutcome {
    Done,
    /// No such player, or they are staff of the moderator's level or above.
    Refused,
    /// The database was unreachable.
    Offline,
}

/// Aborts a spawned task when dropped, so it can't outlive its owner.
struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Renews this world's heartbeat every [`LEASE_INTERVAL`] on a connection of
/// its own, reporting each renewal to the engine as a [`DbResponse::Lease`].
/// Each attempt is bounded by a timeout, and a failure drops the connection
/// for a fresh one next time.
async fn lease_task(config: DbConfig, node_id: u8, response_tx: UnboundedSender<DbResponse>) {
    let mut db: Option<Db> = None;
    loop {
        sleep(LEASE_INTERVAL).await;
        if db.is_none() {
            db = timeout(LEASE_INTERVAL, Db::connect(&config, 1))
                .await
                .ok()
                .and_then(Result::ok);
        }
        let Some(conn) = &db else {
            continue;
        };
        let at = Instant::now();
        match timeout(LEASE_INTERVAL, conn.touch_world(node_id.into())).await {
            Ok(Ok(())) => {
                if response_tx.send(DbResponse::Lease { at }).is_err() {
                    return;
                }
            }
            Ok(Err(e)) => {
                warn!("World heartbeat failed: {}", e);
                db = None;
            }
            Err(_) => {
                warn!("World heartbeat timed out");
                db = None;
            }
        }
    }
}

/// Handles a request while the database is unreachable. Saves go straight to
/// local files (synced on reconnect) and are acked as failed, so a crash
/// during the outage loses nothing that reached this task; the rest is
/// dropped: the engine rejected its logins when the connection went, and the
/// heartbeat's reconcile cleans up any claim a dropped release would have
/// freed.
fn handle_offline(req: DbRequest, response_tx: &UnboundedSender<DbResponse>) {
    if let DbRequest::Logs { logs } = &req {
        drop_logs(logs);
    }
    if let DbRequest::Moderate {
        moderator37,
        target37,
        action,
        reply,
        ..
    } = req
    {
        if let Some(reply) = reply {
            let _ = reply.send(ModOutcome::Offline);
        }
        let _ = response_tx.send(DbResponse::Moderated {
            moderator37,
            target37,
            action,
            outcome: ModOutcome::Offline,
        });
        return;
    }
    if let DbRequest::Save {
        user37,
        username,
        binary,
        logout,
        ..
    } = req
    {
        save_to_file(&username, &binary);
        let _ = response_tx.send(DbResponse::SaveAck {
            user37,
            username,
            outcome: SaveOutcome::Failed,
            logout,
        });
    }
}

/// Counts a log batch the database never got.
fn drop_logs(logs: &LogBatch) {
    counters::add(&COUNTERS.log_batches_dropped, 1);
    let rows = logs.chat.len() + logs.sessions.len() + logs.wealth.len();
    counters::add(&COUNTERS.log_rows_dropped, rows as u64);
}

/// Waits out `delay` before the next connect attempt, handling requests
/// offline meanwhile. Returns `false` once the engine has closed the channel
/// and every request is handled: nothing is left to do.
async fn wait_offline(
    delay: Duration,
    request_rx: &mut UnboundedReceiver<DbRequest>,
    response_tx: &UnboundedSender<DbResponse>,
) -> bool {
    let deadline = sleep(delay);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            _ = &mut deadline => return true,
            req = request_rx.recv() => match req {
                Some(req) => handle_offline(req, response_tx),
                None => return false,
            },
        }
    }
}

/// Brings the schema up to date, or with `auto_migrate` off, checks that it
/// already is.
async fn prepare_schema(db: &Db, auto_migrate: bool) -> Result<(), String> {
    if auto_migrate {
        return db
            .migrate()
            .await
            .map_err(|e| format!("DB migration failed: {e}"));
    }
    match db.pending_migrations().await {
        Ok(pending) if pending.is_empty() => Ok(()),
        Ok(pending) => Err(format!(
            "DB schema is missing migrations {pending:?}; run `rs-server migrate`"
        )),
        Err(e) => Err(format!("DB schema check failed: {e}")),
    }
}

/// Long-running async task that owns the database connection for player
/// operations (authentication, save, load).
///
/// Connects with exponential backoff, applies (or checks for) pending
/// migrations, syncs any locally saved player files to the database, and on
/// the first connect ends every session a previous run of this world left
/// claimed. It then signals readiness via [`DbResponse::DbReady`] and serves
/// requests in order. Requests are handled one at a time so a player's save
/// always lands before a later load or claim.
///
/// If a query fails, a [`DbResponse::DbDisconnected`] is sent and the task
/// reconnects.
///
/// # Call Stack
/// **Calls:** [`sync_local_saves`], [`run_requests`]
pub async fn db_client_task(
    config: DbClientConfig,
    mut request_rx: UnboundedReceiver<DbRequest>,
    response_tx: UnboundedSender<DbResponse>,
) {
    let passwords = Passwords::new(&config.pepper);
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);
    let mut first_connect = true;
    let mut lease: Option<AbortOnDrop> = None;

    loop {
        let delay = match Db::connect(&config.db, 2).await {
            Ok(db) => match serve(
                &db,
                &passwords,
                &config,
                &mut first_connect,
                &mut lease,
                &mut request_rx,
                &response_tx,
            )
            .await
            {
                Served::Closed => {
                    // Stop renewing first, or the lease would re-list the
                    // world right after it retires.
                    drop(lease.take());
                    retire_node(&db, config.node_id).await;
                    db.close().await;
                    info!("DB request channel closed -- stopping DB client task");
                    return;
                }
                Served::Retry(delay) => {
                    db.close().await;
                    delay
                }
                Served::Lost(e) => {
                    db.close().await;
                    warn!("DB connection lost: {}", e);
                    let _ = response_tx.send(DbResponse::DbDisconnected);
                    backoff = Duration::from_secs(1);
                    backoff
                }
            },
            Err(e) => {
                warn!("DB connect failed: {} (retry in {:?})", e, backoff);
                let delay = backoff;
                backoff = (backoff * 2).min(max_backoff);
                delay
            }
        };

        if !wait_offline(delay, &mut request_rx, &response_tx).await {
            drop(lease);
            info!("DB request channel closed while offline -- saves kept as local files");
            return;
        }
    }
}

/// How one connection's service ended.
enum Served {
    /// The engine closed the request channel: shut down.
    Closed,
    /// The connection couldn't be brought into service; retry after a delay.
    Retry(Duration),
    /// The connection failed mid-service.
    Lost(rs_db::Error),
}

/// Brings a fresh connection into service and serves requests on it.
///
/// Checks the schema, then on the first connect takes over this node's
/// leftover sessions and starts the lease task; on a reconnect it syncs the
/// local saves written while offline. It renews the heartbeat before
/// reporting [`DbResponse::DbReady`], so the engine holds a lease from the
/// moment it accepts logins.
async fn serve(
    db: &Db,
    passwords: &Passwords,
    config: &DbClientConfig,
    first_connect: &mut bool,
    lease: &mut Option<AbortOnDrop>,
    request_rx: &mut UnboundedReceiver<DbRequest>,
    response_tx: &UnboundedSender<DbResponse>,
) -> Served {
    info!(
        "DB connected to {}:{}/{}",
        config.db.host, config.db.port, config.db.name
    );

    if let Err(e) = prepare_schema(db, config.auto_migrate).await {
        error!("{}", e);
        return Served::Retry(Duration::from_secs(5));
    }

    if *first_connect {
        match take_over_node(db, config.node_id).await {
            Ok(true) => *first_connect = false,
            Ok(false) => return Served::Retry(Duration::from_secs(5)),
            Err(e) => {
                warn!("Failed to take over leftover sessions: {}", e);
                return Served::Retry(Duration::from_secs(5));
            }
        }
    } else {
        sync_local_saves(db, config.node_id).await;
    }

    let at = Instant::now();
    if let Err(e) = db.touch_world(config.node_id.into()).await {
        return Served::Lost(e);
    }
    let _ = response_tx.send(DbResponse::Lease { at });
    if lease.is_none() {
        *lease = Some(AbortOnDrop(tokio::spawn(lease_task(
            config.db.clone(),
            config.node_id,
            response_tx.clone(),
        ))));
    }
    let _ = response_tx.send(DbResponse::DbReady);

    match run_requests(db, passwords, config, request_rx, response_tx).await {
        Ok(()) => Served::Closed,
        Err(e) => Served::Lost(e),
    }
}

/// Prepares this world's sessions on its first connect: syncs the local save
/// files a previous run left, then ends the sessions that run still owned.
///
/// If this node's heartbeat is still fresh, another process may be running
/// as the same node, and ending "its" sessions would strip a live world's
/// players. A release build then waits (returns `false`) until the heartbeat
/// goes stale; a clean shutdown retires the heartbeat, so only a crash makes
/// a restart wait. Debug builds only warn, to keep dev restarts quick.
///
/// # Call Stack
/// **Called by:** [`db_client_task`]
async fn take_over_node(db: &Db, node_id: u8) -> Result<bool, rs_db::Error> {
    let live = db
        .live_worlds(TAKEOVER_AFTER_SECS)
        .await?
        .iter()
        .any(|world| world.node_id == i16::from(node_id));
    if live {
        if cfg!(debug_assertions) {
            warn!(
                "World {} has a fresh heartbeat: if another process runs as this node, \
                 its sessions are about to be ended",
                node_id
            );
        } else {
            warn!(
                "World {} has a fresh heartbeat: another process may be running as this \
                 node (or it just crashed); waiting for it to go stale",
                node_id
            );
            return Ok(false);
        }
    }

    // Local files first: they hold the newest copies of the sessions a
    // previous run still owns, and releasing those sessions before syncing
    // would get the files refused. Files that still can't sync keep their
    // sessions held.
    sync_local_saves(db, node_id).await;
    let ended = db
        .release_node_sessions(node_id.into(), &local_save_users())
        .await?;
    if ended > 0 {
        warn!("Ended {} session(s) left over from a previous run", ended);
    }

    let orphans = std::fs::read_dir(Path::new("data").join("players"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().contains(".sav.orphaned-"))
                .count()
        })
        .unwrap_or(0);
    if orphans > 0 {
        warn!(
            "{} orphaned save file(s) in data/players await manual review (refused as stale)",
            orphans
        );
    }
    Ok(true)
}

/// Takes this world offline at a clean shutdown: ends any session it still
/// owns (every player has been logged out by now) and retires its heartbeat.
/// Best effort; a failure here is covered by the next start.
async fn retire_node(db: &Db, node_id: u8) {
    if let Err(e) = db
        .release_node_sessions(node_id.into(), &local_save_users())
        .await
    {
        warn!("Failed to end sessions at shutdown: {}", e);
    }
    if let Err(e) = db.retire_world(node_id.into()).await {
        warn!("Failed to retire world status at shutdown: {}", e);
    }
}

/// The players with a local `.sav` file waiting to be synced.
fn local_save_users() -> Vec<u64> {
    let Ok(entries) = std::fs::read_dir(Path::new("data").join("players")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != "sav" {
                return None;
            }
            Some(to_userhash(path.file_stem()?.to_str()?))
        })
        .collect()
}

/// Scans the `data/players/` directory for locally saved `.sav` files and
/// uploads each one to the database.
///
/// A file is only accepted for a player whose session this world still
/// owns; then it is the newest copy, and is deleted once stored. A file for
/// a session the world lost (another world took it over, or it already
/// ended) is stale and is moved aside as an orphan rather than overwriting
/// newer data. Files whose player is not registered are kept. This handles
/// the case where the database was unavailable when a save was attempted.
///
/// # Call Stack
/// **Called by:** [`db_client_task`]
async fn sync_local_saves(db: &Db, node_id: u8) {
    let mut synced = 0;
    for user37 in local_save_users() {
        let username = to_raw_username(user37);
        match sync_local_save(db, node_id, user37, &username).await {
            Ok(Some(SaveResult::Stored)) => synced += 1,
            Ok(Some(SaveResult::NotOwner)) => {
                error!(
                    "Local save for '{}' is stale (session not owned by this world) - orphaned",
                    username
                );
            }
            Ok(Some(SaveResult::Unregistered)) => {
                warn!(
                    "No registered player '{}' - keeping local save file",
                    username
                );
            }
            Ok(None) => {}
            Err(e) => {
                warn!("Failed to sync local save '{}' to DB: {}", username, e);
            }
        }
    }

    if synced > 0 {
        info!("Synced {} local save(s) to database", synced);
    }
}

/// Uploads one local save file (see [`sync_local_saves`]). Returns `None` if
/// there is no usable file; an unreadable one is moved aside as an orphan.
async fn sync_local_save(
    db: &Db,
    node_id: u8,
    user37: u64,
    username: &str,
) -> Result<Option<SaveResult>, rs_db::Error> {
    let Some(data) = load_from_file(username) else {
        return Ok(None);
    };
    let profile = match load_binary(&data) {
        Ok(profile) => profile,
        Err(e) => {
            error!("Unreadable local save for '{}' - orphaned: {}", username, e);
            orphan_save_file(username);
            return Ok(None);
        }
    };
    let result = db
        .store_save(user37, node_id.into(), &to_save(&profile))
        .await?;
    match result {
        SaveResult::Stored => delete_save_file(username),
        SaveResult::NotOwner => orphan_save_file(username),
        SaveResult::Unregistered => {}
    }
    Ok(Some(result))
}

/// Serves requests until the channel closes or a query fails.
///
/// On save failure, falls back to writing a local `.sav` file.
///
/// # Returns
/// `Ok(())` when the channel closes, or the database error that caused the
/// connection to be considered lost. A request that hits an error gets no
/// response; the engine rejects pending logins on [`DbResponse::DbDisconnected`].
///
/// # Call Stack
/// **Called by:** [`db_client_task`]
/// **Calls:** [`authenticate`], [`claim`]
async fn run_requests(
    db: &Db,
    passwords: &Passwords,
    config: &DbClientConfig,
    request_rx: &mut UnboundedReceiver<DbRequest>,
    response_tx: &UnboundedSender<DbResponse>,
) -> Result<(), rs_db::Error> {
    let node_id = i16::from(config.node_id);
    let mut history_pruned: Option<Instant> = None;
    while let Some(req) = request_rx.recv().await {
        match req {
            DbRequest::Authenticate {
                user37,
                password,
                ip,
                uid,
                device,
            } => {
                let result = authenticate(
                    db,
                    passwords,
                    config.auto_register,
                    config.node_id,
                    user37,
                    &password,
                    ip,
                    uid,
                    &device,
                )
                .await?;
                let _ = response_tx.send(DbResponse::AuthResponse { user37, result });
            }
            DbRequest::Save {
                user37,
                username,
                profile,
                binary,
                logout,
            } => {
                let save = to_save(&profile);
                let result = if logout {
                    db.store_logout_save(user37, node_id, &save).await
                } else {
                    db.store_save(user37, node_id, &save).await
                };
                let outcome = match &result {
                    Ok(SaveResult::Stored) => SaveOutcome::Stored,
                    Ok(SaveResult::Unregistered) => {
                        save_to_file(&username, &binary);
                        SaveOutcome::Unregistered
                    }
                    Ok(SaveResult::NotOwner) => {
                        error!(
                            "Save for '{}' refused: session not owned by this world",
                            username
                        );
                        save_orphan(&username, &binary);
                        SaveOutcome::NotOwner
                    }
                    Err(e) => {
                        error!("DB save failed for '{}': {}", username, e);
                        save_to_file(&username, &binary);
                        SaveOutcome::Failed
                    }
                };
                let _ = response_tx.send(DbResponse::SaveAck {
                    user37,
                    username,
                    outcome,
                    logout,
                });
                result?;
            }
            DbRequest::Load { user37 } => {
                let result = claim(db, config.node_id, user37).await?;
                let _ = response_tx.send(DbResponse::LoadResponse { user37, result });
            }
            DbRequest::EditLoad { user37, reply } => {
                let _ = reply.send(claim(db, config.node_id, user37).await?);
            }
            DbRequest::EditStore {
                user37,
                profile,
                note,
                reply,
            } => {
                let outcome = match db
                    .store_edit_save(user37, node_id, &to_save(&profile), &note)
                    .await?
                {
                    SaveResult::Stored => SaveOutcome::Stored,
                    SaveResult::NotOwner => SaveOutcome::NotOwner,
                    SaveResult::Unregistered => SaveOutcome::Unregistered,
                };
                let _ = reply.send(outcome);
            }
            DbRequest::SyncLocal { user37, reply } => {
                let username = to_raw_username(user37);
                let outcome = match sync_local_save(db, config.node_id, user37, &username).await {
                    Ok(None) => SyncOutcome::NoFile,
                    Ok(Some(SaveResult::Stored)) => SyncOutcome::Synced,
                    Ok(Some(SaveResult::NotOwner)) => SyncOutcome::Orphaned,
                    Ok(Some(SaveResult::Unregistered)) => SyncOutcome::Failed,
                    Err(e) => {
                        warn!("Failed to sync local save '{}' to DB: {}", username, e);
                        SyncOutcome::Failed
                    }
                };
                let _ = reply.send(outcome);
            }
            DbRequest::Release { user37 } => {
                // Unsynced local data means the database copy is stale:
                // keep the claim so no other world loads it.
                if local_save_users().contains(&user37) {
                    warn!(
                        "Kept the session of '{}': it has an unsynced local save",
                        to_raw_username(user37)
                    );
                } else {
                    db.release_session(user37, node_id).await?;
                }
            }
            DbRequest::WorldStatus {
                players,
                maintenance,
                online,
                mut held,
            } => {
                let at = Instant::now();
                db.report_world_status(node_id, players as i32, maintenance)
                    .await?;
                let _ = response_tx.send(DbResponse::Lease { at });
                held.extend(local_save_users());
                let fixed = db.reconcile_sessions(node_id, &online, &held).await?;
                for user37 in &fixed.released {
                    warn!(
                        "Released a leaked session claim on '{}'",
                        to_raw_username(*user37)
                    );
                }
                if !fixed.lost.is_empty() {
                    let _ = response_tx.send(DbResponse::SessionsLost {
                        users37: fixed.lost,
                    });
                }
                db.prune_login_failures(LOGIN_FAILURE_WINDOW_SECS).await?;
                db.prune_login_logs(LOGIN_LOG_DAYS).await?;
                if history_pruned.is_none_or(|at| at.elapsed() > SAVE_HISTORY_PRUNE) {
                    history_pruned = Some(Instant::now());
                    let pruned = db
                        .prune_save_history(SAVE_HISTORY_DAYS, SAVE_HISTORY_KEEP)
                        .await?;
                    if pruned > 0 {
                        info!(
                            "Pruned {} save history rows past {} days",
                            pruned, SAVE_HISTORY_DAYS
                        );
                    }
                }
                let sanctions = db.sanctions(&online).await?;
                let _ = response_tx.send(DbResponse::Sanctions {
                    players: sanctions
                        .into_iter()
                        .map(|s| (s.name_hash, s.muted_ms.map(millis), s.banned))
                        .collect(),
                });
            }
            DbRequest::Snapshot {
                reporter37,
                offender37,
                reason,
                x,
                z,
                y,
                mute_by,
            } => {
                let muted = match mute_by {
                    Some(level) => {
                        db.mute_player(offender37, MUTE_HOURS, i16::from(level))
                            .await?
                    }
                    None => None,
                };
                if let Some(ms) = muted {
                    counters::add(&COUNTERS.mutes, 1);
                    info!(
                        "'{}' muted '{}' for {} hours",
                        to_raw_username(reporter37),
                        to_raw_username(offender37),
                        MUTE_HOURS
                    );
                    let _ = response_tx.send(DbResponse::Muted {
                        user37: offender37,
                        remaining: millis(ms),
                    });
                }
                db.record_snapshot(&NewSnapshot {
                    reporter_name_hash: reporter37,
                    offender_name_hash: offender37,
                    reason: i16::from(reason),
                    node_id,
                    x: x as i16,
                    z: z as i16,
                    y: i16::from(y),
                    muted: muted.is_some(),
                })
                .await?;
            }
            DbRequest::Logs { logs } => {
                if let Err(e) = db.record_logs(node_id, &logs).await {
                    drop_logs(&logs);
                    return Err(e);
                }
                counters::add(&COUNTERS.log_batches, 1);
                let rows = [logs.chat.len(), logs.sessions.len(), logs.wealth.len()];
                for (total, n) in COUNTERS.log_rows.iter().zip(rows) {
                    counters::add(total, n as u64);
                }
            }
            DbRequest::Moderate {
                moderator37,
                by_level,
                target37,
                action,
                reason,
                reply,
            } => {
                let done = db
                    .moderate(
                        moderator37,
                        i16::from(by_level),
                        target37,
                        action,
                        reason.as_deref(),
                        node_id,
                    )
                    .await?;
                if done {
                    counters::add(&COUNTERS.mod_actions[mod_action_index(action)], 1);
                    info!(
                        "'{}' ran {} on '{}'",
                        to_raw_username(moderator37),
                        action.name(),
                        to_raw_username(target37)
                    );
                }
                let outcome = if done {
                    ModOutcome::Done
                } else {
                    ModOutcome::Refused
                };
                if let Some(reply) = reply {
                    let _ = reply.send(outcome);
                }
                let _ = response_tx.send(DbResponse::Moderated {
                    moderator37,
                    target37,
                    action,
                    outcome,
                });
            }
            DbRequest::Ban { user37, hours } => {
                if db.ban_player(user37, hours).await? {
                    warn!("Banned '{}' for {} hours", to_raw_username(user37), hours);
                }
            }
        }
    }
    Ok(())
}

/// Claims a player's session for this world and loads their profile.
///
/// A local save file for the player (data this world failed to store) is
/// newer than the database copy: once the claim makes this world the owner
/// it is stored and used instead.
///
/// # Call Stack
/// **Called by:** [`run_requests`]
async fn claim(db: &Db, node_id: u8, user37: u64) -> Result<LoadResult, rs_db::Error> {
    let username = to_raw_username(user37);
    let save = match db
        .claim_session(user37, node_id.into(), TAKEOVER_AFTER_SECS)
        .await?
    {
        Claim::Claimed { save, took_over } => {
            if let Some(dead) = took_over {
                warn!(
                    "Took over the session of '{}' from unresponsive world {}",
                    username, dead
                );
            }
            save
        }
        Claim::Busy { node } => {
            info!("Session of '{}' is owned by world {}", username, node);
            return Ok(LoadResult::Busy);
        }
        Claim::Unregistered => return Ok(LoadResult::Unregistered),
    };

    if let Some(data) = load_from_file(&username) {
        match load_binary(&data) {
            Ok(local) => {
                if db
                    .store_save(user37, node_id.into(), &to_save(&local))
                    .await?
                    == SaveResult::Stored
                {
                    delete_save_file(&username);
                    warn!("Loaded '{}' from an unsynced local save", username);
                    return Ok(LoadResult::Loaded(Some(Box::new(local))));
                }
            }
            Err(e) => {
                error!("Unreadable local save for '{}' - orphaned: {}", username, e);
                orphan_save_file(&username);
            }
        }
    }
    Ok(LoadResult::Loaded(
        save.map(|save| Box::new(profile_from_save(*save))),
    ))
}

/// Authenticates a login against the `players` table.
///
/// An IP with too many recent wrong passwords (across all worlds) is refused
/// before the password is checked. A registered player is accepted if the
/// password verifies and they are not banned (the password is checked first
/// so a ban is never revealed to a wrong password). An unknown name is
/// rejected, unless `auto_register` is set, in which case the player is
/// registered with this password and any local save file for the name is
/// imported. Wrong passwords and unknown names count against the IP; an
/// accepted login is logged with its IP, client uid and device.
///
/// # Call Stack
/// **Called by:** [`run_requests`]
async fn authenticate(
    db: &Db,
    passwords: &Passwords,
    auto_register: bool,
    node_id: u8,
    user37: u64,
    password: &str,
    ip: IpAddr,
    uid: Option<i32>,
    device: &LoginDevice,
) -> Result<AuthResult, rs_db::Error> {
    if db
        .recent_login_failures(ip, LOGIN_FAILURE_WINDOW_SECS)
        .await?
        >= i64::from(MAX_LOGIN_FAILURES)
    {
        return Ok(AuthResult::TooManyAttempts);
    }

    if let Some(creds) = db.find_credentials(user37, node_id.into()).await? {
        if !passwords.verify(password, &creds.password_hash) {
            db.record_login_failure(ip).await?;
            return Ok(AuthResult::InvalidCredentials);
        }
        if creds.banned {
            return Ok(AuthResult::Banned);
        }
        db.record_login(user37, node_id.into(), ip, uid, device)
            .await?;
        let week_ago = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64)
            - crate::watch::schedule::WEEK_MS;
        let played = db.played_since(user37, week_ago).await.unwrap_or_else(|e| {
            warn!(
                "Couldn't read the sessions '{}' played: {}",
                to_raw_username(user37),
                e
            );
            Vec::new()
        });
        return Ok(AuthResult::Accepted {
            staff_mod_level: creds.staff_mod_level as u8,
            muted_for: creds.muted_ms.map(millis),
            left_other_world: creds.left_other_world_ms.map(millis),
            account_age: millis(creds.age_ms),
            played,
        });
    }

    if !auto_register {
        db.record_login_failure(ip).await?;
        return Ok(AuthResult::InvalidCredentials);
    }

    // A new account's password follows the website's rule, so it can be used
    // there (appeals) as well as here.
    if let Err(e) = check_new_password(password) {
        info!("Didn't auto-register '{}': {}", to_raw_username(user37), e);
        return Ok(AuthResult::InvalidCredentials);
    }
    let Some(password_hash) = passwords.hash(password) else {
        error!("Failed to hash password for user37={}", user37);
        return Ok(AuthResult::InvalidCredentials);
    };
    // Dev builds hand auto-registered players full staff rights.
    let staff_mod_level = if cfg!(debug_assertions) { 3 } else { 0 };
    let username = to_raw_username(user37);
    let created = db
        .create_player(&NewPlayer {
            name_hash: user37,
            display_name: &to_screen_name(&username),
            password_hash: &password_hash,
            staff_mod_level,
            registration_ip: None,
        })
        .await?;
    if !created {
        // Registered concurrently (another world, or the website).
        return Ok(AuthResult::InvalidCredentials);
    }
    info!("Auto-registered player '{}'", username);

    if let Some(data) = load_from_file(&username)
        && let Ok(profile) = load_binary(&data)
        && db.import_save(user37, &to_save(&profile)).await?
    {
        delete_save_file(&username);
        info!("Imported local save for '{}'", username);
    }

    db.record_login(user37, node_id.into(), ip, uid, device)
        .await?;
    Ok(AuthResult::Accepted {
        staff_mod_level: staff_mod_level as u8,
        muted_for: None,
        left_other_world: None,
        account_age: Duration::ZERO,
        played: Vec::new(),
    })
}

/// A database millisecond count as a [`Duration`], clamping negatives to zero.
fn millis(ms: i64) -> Duration {
    Duration::from_millis(ms.max(0) as u64)
}

/// Flattens a [`PlayerProfile`] into its `player_saves` row. Inventory items
/// are spread across the index-aligned `inv_*` arrays, grouped by inventory.
fn to_save(profile: &PlayerProfile) -> PlayerSave {
    let mut save = PlayerSave {
        x: profile.x as i16,
        z: profile.z as i16,
        y: profile.y as i16,
        body: profile.body.iter().map(|&v| v as i16).collect(),
        colors: profile.colors.iter().map(|&v| v as i16).collect(),
        gender: profile.gender as i16,
        runenergy: profile.runenergy as i32,
        playtime: profile.playtime,
        stats: profile.stats.to_vec(),
        levels: profile.levels.iter().map(|&v| v as i16).collect(),
        afk_zones: profile.afk_zones.iter().map(|&v| v as i32).collect(),
        last_afk_zone: profile.last_afk_zone as i16,
        public_chat: profile.public_chat as i16,
        private_chat: profile.private_chat as i16,
        trade_chat: profile.trade_chat as i16,
        last_date: profile.last_date,
        varp_ids: profile.varps.iter().map(|&(id, _)| id as i32).collect(),
        varp_values: profile.varps.iter().map(|&(_, v)| v).collect(),
        ..Default::default()
    };
    for inv in &profile.invs {
        for &(slot, obj_id, count) in &inv.items {
            save.inv_types.push(inv.inv_type as i32);
            save.inv_slots.push(slot as i16);
            save.inv_objs.push(obj_id as i32);
            save.inv_counts.push(count as i32);
        }
    }
    save
}

/// Rebuilds a [`PlayerProfile`] from its `player_saves` row. Short arrays are
/// padded with defaults; the flattened `inv_*` arrays are regrouped per
/// inventory (in any order).
pub fn profile_from_save(save: PlayerSave) -> PlayerProfile {
    fn fill<T: Copy, const N: usize>(default: T, values: impl Iterator<Item = T>) -> [T; N] {
        let mut out = [default; N];
        for (slot, v) in out.iter_mut().zip(values) {
            *slot = v;
        }
        out
    }

    let varps = save
        .varp_ids
        .iter()
        .zip(&save.varp_values)
        .map(|(&id, &val)| (id as u16, val))
        .collect();

    let mut invs: Vec<PlayerProfileInv> = Vec::new();
    for (i, inv_type) in save.inv_types.iter().map(|&x| x as u16).enumerate() {
        let item = (
            save.inv_slots.get(i).copied().unwrap_or(0) as u16,
            save.inv_objs.get(i).copied().unwrap_or(0) as u16,
            save.inv_counts.get(i).copied().unwrap_or(0) as u32,
        );
        if let Some(inv) = invs.iter_mut().find(|inv| inv.inv_type == inv_type) {
            inv.items.push(item);
        } else {
            invs.push(PlayerProfileInv {
                inv_type,
                items: vec![item],
            });
        }
    }

    PlayerProfile {
        x: save.x as u16,
        z: save.z as u16,
        y: save.y as u8,
        body: fill(0, save.body.iter().map(|&v| v as i32)),
        colors: fill(0, save.colors.iter().map(|&v| v as u8)),
        gender: save.gender as u8,
        runenergy: save.runenergy as u16,
        playtime: save.playtime,
        stats: fill(0, save.stats.iter().copied()),
        levels: fill(1, save.levels.iter().map(|&v| v as u8)),
        varps,
        invs,
        afk_zones: fill(0, save.afk_zones.iter().map(|&v| v as u32)),
        last_afk_zone: save.last_afk_zone as u16,
        public_chat: save.public_chat as u8,
        private_chat: save.private_chat as u8,
        trade_chat: save.trade_chat as u8,
        last_date: save.last_date,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_round_trips_through_save_row() {
        let profile = PlayerProfile {
            x: 3222,
            z: 3218,
            y: 1,
            body: [0, 10, 18, 26, 33, 36, 42],
            colors: [1, 2, 3, 4, 5],
            gender: 1,
            runenergy: 9000,
            playtime: 600,
            stats: [1154; 21],
            levels: [10; 21],
            varps: vec![(281, 1000), (1000, -1)],
            invs: vec![
                PlayerProfileInv {
                    inv_type: 93,
                    items: vec![(0, 995, u32::MAX >> 1), (27, 1351, 1)],
                },
                PlayerProfileInv {
                    inv_type: 94,
                    items: vec![(3, 1277, 1)],
                },
            ],
            afk_zones: [7, 8],
            last_afk_zone: 9,
            public_chat: 1,
            private_chat: 2,
            trade_chat: 3,
            last_date: 1_700_000_000,
        };

        let back = profile_from_save(to_save(&profile));
        assert_eq!((back.x, back.z, back.y), (3222, 3218, 1));
        assert_eq!(back.body, profile.body);
        assert_eq!(back.colors, profile.colors);
        assert_eq!(back.stats, profile.stats);
        assert_eq!(back.levels, profile.levels);
        assert_eq!(back.varps, profile.varps);
        assert_eq!(back.invs.len(), 2);
        for (a, b) in back.invs.iter().zip(&profile.invs) {
            assert_eq!(a.inv_type, b.inv_type);
            assert_eq!(a.items, b.items);
        }
        assert_eq!(back.afk_zones, profile.afk_zones);
        assert_eq!(
            (back.public_chat, back.private_chat, back.trade_chat),
            (1, 2, 3)
        );
        assert_eq!(back.last_date, profile.last_date);
    }

    const AUTH_PEPPER: &str = "pepper";

    async fn auth(db: &Db, auto_register: bool, name: &str, password: &str) -> AuthResult {
        let passwords = Passwords::new(AUTH_PEPPER);
        authenticate(
            db,
            &passwords,
            auto_register,
            1,
            to_userhash(name),
            password,
            [203, 0, 113, 7].into(),
            None,
            &LoginDevice::default(),
        )
        .await
        .unwrap()
    }

    /// Needs a reachable Postgres:
    /// `DATABASE_URL=postgres://postgres:password@localhost/postgres cargo test -p rs-engine client_db -- --ignored`
    #[sqlx::test(migrator = "rs_db::MIGRATOR")]
    #[ignore = "needs DATABASE_URL"]
    async fn login_policy(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool.clone());

        // Unknown names are rejected unless auto-registering.
        assert!(matches!(
            auth(&db, false, "bob", "secret").await,
            AuthResult::InvalidCredentials
        ));
        assert!(
            db.find_credentials(to_userhash("bob"), 1)
                .await
                .unwrap()
                .is_none()
        );

        // Auto-register creates the player with this password.
        let staff = if cfg!(debug_assertions) { 3 } else { 0 };
        assert!(matches!(
            auth(&db, true, "bob", "secret").await,
            AuthResult::Accepted { staff_mod_level, .. } if staff_mod_level == staff
        ));

        // From then on the password is enforced, auto-register or not, and
        // the name matches case- and space/underscore-insensitively.
        assert!(matches!(
            auth(&db, true, "bob", "wrong").await,
            AuthResult::InvalidCredentials
        ));
        assert!(matches!(
            auth(&db, false, "BOB", "secret").await,
            AuthResult::Accepted { .. }
        ));

        sqlx::query("UPDATE players SET banned_until = now() + interval '1 day'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(matches!(
            auth(&db, false, "bob", "secret").await,
            AuthResult::Banned
        ));
        // A wrong password never learns about the ban.
        assert!(matches!(
            auth(&db, false, "bob", "wrong").await,
            AuthResult::InvalidCredentials
        ));

        // That was the IP's third failure (the unknown name, then two wrong
        // passwords); two more lock it out, even with the right password.
        sqlx::query("UPDATE players SET banned_until = NULL")
            .execute(&pool)
            .await
            .unwrap();
        for _ in 0..2 {
            assert!(matches!(
                auth(&db, false, "bob", "wrong").await,
                AuthResult::InvalidCredentials
            ));
        }
        assert!(matches!(
            auth(&db, false, "bob", "secret").await,
            AuthResult::TooManyAttempts
        ));
    }

    /// The settings to reach a `#[sqlx::test]` database: its name from the
    /// pool, the rest from `DATABASE_URL`.
    fn test_db_config(pool: &sqlx::PgPool) -> DbConfig {
        let url = std::env::var("DATABASE_URL").unwrap();
        let rest = url.split_once("://").unwrap().1;
        let (auth, host) = rest.split_once('@').unwrap();
        let (user, pass) = auth.split_once(':').unwrap_or((auth, ""));
        let host = host.split('/').next().unwrap();
        let (host, port) = host.split_once(':').unwrap_or((host, "5432"));
        DbConfig {
            host: host.to_owned(),
            port: port.parse().unwrap(),
            name: pool.connect_options().get_database().unwrap().to_owned(),
            user: user.to_owned(),
            pass: pass.to_owned(),
        }
    }

    /// One world's database client task, driven like the engine drives it.
    struct World {
        tx: UnboundedSender<DbRequest>,
        rx: UnboundedReceiver<DbResponse>,
        leases: u32,
    }

    const HOPPER: &str = "hopper";

    impl World {
        async fn start(pool: &sqlx::PgPool, node_id: u8) -> Self {
            let (tx, req_rx) = tokio::sync::mpsc::unbounded_channel();
            let (resp_tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let config = DbClientConfig {
                db: test_db_config(pool),
                node_id,
                pepper: AUTH_PEPPER.to_owned(),
                auto_register: false,
                auto_migrate: false,
            };
            tokio::spawn(db_client_task(config, req_rx, resp_tx));
            let mut world = World { tx, rx, leases: 0 };
            assert!(matches!(world.next().await, DbResponse::DbReady));
            // The world holds a lease before it accepts any login.
            assert_eq!(world.leases, 1);
            world
        }

        /// The next response, skipping lease renewals (but counting them).
        async fn next(&mut self) -> DbResponse {
            loop {
                let response = tokio::time::timeout(Duration::from_secs(10), self.rx.recv())
                    .await
                    .expect("db task stalled")
                    .expect("db task stopped");
                if !matches!(response, DbResponse::Lease { .. }) {
                    return response;
                }
                self.leases += 1;
            }
        }

        async fn auth(&mut self) -> AuthResult {
            let _ = self.tx.send(DbRequest::Authenticate {
                user37: to_userhash(HOPPER),
                password: "secret".into(),
                ip: [198, 51, 100, 1].into(),
                uid: None,
                device: Box::default(),
            });
            match self.next().await {
                DbResponse::AuthResponse { result, .. } => result,
                _ => panic!("expected an auth response"),
            }
        }

        async fn load(&mut self) -> LoadResult {
            let _ = self.tx.send(DbRequest::Load {
                user37: to_userhash(HOPPER),
            });
            match self.next().await {
                DbResponse::LoadResponse { result, .. } => result,
                _ => panic!("expected a load response"),
            }
        }

        async fn save(&mut self, x: i16, logout: bool) -> SaveOutcome {
            let profile = profile_from_save(PlayerSave {
                x,
                ..PlayerSave::default()
            });
            let _ = self.tx.send(DbRequest::Save {
                user37: to_userhash(HOPPER),
                username: HOPPER.to_owned(),
                profile: Box::new(profile),
                binary: Vec::new(),
                logout,
            });
            match self.next().await {
                DbResponse::SaveAck { outcome, .. } => outcome,
                _ => panic!("expected a save ack"),
            }
        }

        /// Sends a heartbeat with the player online, then waits on an auth
        /// round trip so the heartbeat has landed. Returns the players the
        /// world was told it lost.
        async fn heartbeat_online(&mut self) -> Vec<u64> {
            let _ = self.tx.send(DbRequest::WorldStatus {
                players: 1,
                maintenance: false,
                online: vec![to_userhash(HOPPER)],
                held: Vec::new(),
            });
            let mut lost = Vec::new();
            let _ = self.tx.send(DbRequest::Authenticate {
                user37: to_userhash(HOPPER),
                password: "secret".into(),
                ip: [198, 51, 100, 1].into(),
                uid: None,
                device: Box::default(),
            });
            loop {
                match self.next().await {
                    DbResponse::SessionsLost { users37 } => lost = users37,
                    DbResponse::Sanctions { .. } => {}
                    DbResponse::AuthResponse { .. } => return lost,
                    _ => panic!("unexpected response"),
                }
            }
        }
    }

    fn loaded_x(result: LoadResult) -> Option<u16> {
        match result {
            LoadResult::Loaded(profile) => Some(profile.map_or(0, |p| p.x)),
            _ => None,
        }
    }

    /// Two worlds on one database handing a player back and forth: the
    /// session claim refuses a second owner, refuses a stale world's saves,
    /// times out a dead world, and is released on shutdown.
    #[sqlx::test(migrator = "rs_db::MIGRATOR")]
    #[ignore = "needs DATABASE_URL"]
    async fn worlds_hand_over_a_player(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool.clone());
        let hash = Passwords::new(AUTH_PEPPER).hash("secret").unwrap();
        db.create_player(&NewPlayer {
            name_hash: to_userhash(HOPPER),
            display_name: "Hopper",
            password_hash: &hash,
            staff_mod_level: 0,
            registration_ip: None,
        })
        .await
        .unwrap();

        let mut one = World::start(&pool, 1).await;
        let mut two = World::start(&pool, 2).await;

        // World 1 logs the player in and keeps a live heartbeat.
        assert!(matches!(
            one.auth().await,
            AuthResult::Accepted {
                left_other_world: None,
                ..
            }
        ));
        assert_eq!(loaded_x(one.load().await), Some(0));
        assert!(one.heartbeat_online().await.is_empty());

        // World 2 can't claim them while world 1 has them.
        assert!(matches!(two.load().await, LoadResult::Busy));
        assert_eq!(one.save(3100, false).await, SaveOutcome::Stored);

        // World 1's logout save ends the session; world 2 sees the hop and
        // the final save.
        assert_eq!(one.save(3200, true).await, SaveOutcome::Stored);
        match two.auth().await {
            AuthResult::Accepted {
                left_other_world: Some(ago),
                ..
            } => assert!(ago < Duration::from_secs(5)),
            _ => panic!("expected a recent hop"),
        }
        assert_eq!(loaded_x(two.load().await), Some(3200));
        assert!(two.heartbeat_online().await.is_empty());
        assert!(matches!(one.load().await, LoadResult::Busy));

        // A live world's lease keeps its players: world 2 renews its
        // heartbeat on its own even if a stale one is forced on it.
        sqlx::query("UPDATE world_status SET updated_at = now() - interval '2 minutes'")
            .execute(&pool)
            .await
            .unwrap();
        tokio::time::sleep(LEASE_INTERVAL + Duration::from_secs(1)).await;
        assert!(matches!(one.load().await, LoadResult::Busy));

        // A world that went silent (9) loses them: world 1 takes over, and
        // world 2, which still thinks it has them, learns it lost them.
        sqlx::query("UPDATE players SET online_node = 9")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO world_status (node_id, players, updated_at)
             VALUES (9, 1, now() - interval '2 minutes')",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(loaded_x(one.load().await), Some(3200));
        assert_eq!(two.heartbeat_online().await, vec![to_userhash(HOPPER)]);
        assert!(two.leases >= 2, "the lease task renews on its own");

        // A login that never entered releases its claim.
        let _ = one.tx.send(DbRequest::Release {
            user37: to_userhash(HOPPER),
        });
        one.auth().await;
        assert_eq!(loaded_x(two.load().await), Some(3200));

        // A clean shutdown releases what the world held and retires it.
        drop(two.tx);
        while let Some(response) = two.rx.recv().await {
            assert!(matches!(response, DbResponse::Lease { .. }));
        }
        assert!(
            !db.live_worlds(60)
                .await
                .unwrap()
                .iter()
                .any(|w| w.node_id == 2)
        );
        assert_eq!(loaded_x(one.load().await), Some(3200));
    }

    /// With the database down, the task must still finish once the engine
    /// hangs up, or a shutdown would wait on it forever.
    #[tokio::test]
    async fn offline_wait_ends_when_the_engine_hangs_up() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let (resp_tx, mut resp_rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = tx.send(DbRequest::Release { user37: 1 });
        assert!(wait_offline(Duration::from_millis(50), &mut rx, &resp_tx).await);
        assert!(rx.try_recv().is_err(), "requests are handled while offline");

        drop(tx);
        let wait = wait_offline(Duration::from_secs(60), &mut rx, &resp_tx);
        let done = tokio::time::timeout(Duration::from_secs(1), wait).await;
        assert_eq!(
            done.ok(),
            Some(false),
            "returns at once, not after the delay"
        );
        assert!(resp_rx.try_recv().is_err());
    }

    #[test]
    fn short_arrays_are_padded() {
        let back = profile_from_save(PlayerSave::default());
        assert_eq!(back.levels, [1; 21]);
        assert_eq!(back.stats, [0; 21]);
        assert!(back.invs.is_empty());
    }
}
