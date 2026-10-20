use crate::{ConnectionGuard, REVISION};
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rs_admin_api::{
    Broadcast, CancelReboot, ChatLookup, ChatRecord, ControlsSet, Health, InvSheet, ItemLine, Kick,
    LinkedAccount, Links, LinksLookup, LoginAddress, Maintenance, ModOutcome, Moderate,
    ModerationLookup, ModerationRecord, ObjLine, ObjSearch, OnlinePlayer, Outcome, PlayerEdit,
    PlayerEditRequest, PlayerLookup, PlayerRecord, PlayerSheet, Reboot, ReplayGet, ReplayLookup,
    ReplaySummary, ReportLookup, ReportRecord, Sanction, SanctionsLookup, SetControls, SheetLookup,
    StatLine, TimelineEntry, TimelineLookup, Verifier, WorldControls, WorldStatus, path,
};
use rs_db::{Db, DbConfig, ModAction, TimelineCursor};
use rs_engine::controls::{self, Controls};
use rs_engine::edit::{self, Change};
use rs_engine::watch::playback;
use rs_engine::{DbRequest, Engine, LoadResult, SaveOutcome, TickStats, profile_from_save};
use rs_util::base37::{is_valid_userhash, to_raw_username, to_screen_name, to_userhash};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{oneshot, watch};
use tokio::time::timeout;
use tracing::{info, warn};

/// Work for the engine task to run between ticks, with the engine to itself.
pub type EngineJob = Box<dyn FnOnce(&mut Engine) + Send>;

/// How long a request waits on the engine (or its database) before it's
/// answered as unavailable.
const ENGINE_TIMEOUT: Duration = Duration::from_secs(10);

/// Request bodies are a few small JSON fields.
const MAX_BODY: usize = 16 * 1024;

/// The longest a reboot may be scheduled ahead: ten hours, inside the
/// client's countdown (a u16 of ticks, about 10.9 hours).
const MAX_REBOOT_SECS: u32 = 36_000;

/// The longest broadcast, in characters.
const MAX_BROADCAST_CHARS: usize = 400;

/// The longest mute or ban: 10 years, as in game.
const MAX_MOD_HOURS: i32 = 87_600;

/// The longest reboot reason, in characters.
const MAX_REASON_CHARS: usize = 80;

/// The longest reason logged with a moderation command, in characters.
const MAX_MOD_REASON_CHARS: usize = 300;

/// The longest kill switch pattern, in characters.
const MAX_PATTERN_CHARS: usize = 100;

/// The most scripts one kill switch may turn off: more is a pattern too
/// broad to be meant (`*` would stop every player script, logins included).
const MAX_SWITCH_SCRIPTS: usize = 300;

/// The longest login lockdown: a year.
const MAX_LOCKDOWN_HOURS: u32 = 8760;

/// The most accounts and addresses a links lookup returns.
const MAX_LINKS: u32 = 100;

/// The most rows a lookup returns.
const MAX_LOOKUP_ROWS: u32 = 200;

/// Database connections kept for lookups, apart from the game's own.
const LOOKUP_CONNECTIONS: u32 = 2;

/// Ticks the health summary covers: a minute's worth.
const HEALTH_TICKS: usize = 100;

/// The game tick, and the length a tick may take before it overruns.
const TICK_BUDGET_MS: f32 = 600.0;

struct AdminState {
    verifier: Verifier,
    jobs: UnboundedSender<EngineJob>,
    node_id: u8,
    members: bool,
    started: Instant,
    db: Db,
    ticks: Mutex<VecDeque<f32>>,
    connections: ConnectionGuard,
    sys: Mutex<System>,
    pid: Option<Pid>,
    reboot_reason: Mutex<Option<String>>,
}

impl AdminState {
    fn health(&self) -> Health {
        let ticks = self.ticks.lock().unwrap_or_else(|e| e.into_inner());
        let count = ticks.len().max(1) as f32;
        Health {
            tick_ms: ticks.iter().sum::<f32>() / count,
            tick_max_ms: ticks.iter().copied().fold(0.0, f32::max),
            overruns: ticks.iter().filter(|&&ms| ms > TICK_BUDGET_MS).count() as u32,
            memory_bytes: self.resident_bytes(),
            connections: self.connections.active() as u32,
        }
    }

    fn resident_bytes(&self) -> Option<u64> {
        let pid = self.pid?;
        let mut sys = self.sys.lock().ok()?;
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        sys.process(pid).map(|p| p.memory())
    }
}

/// Keeps the last [`HEALTH_TICKS`] tick times for the health summary.
async fn record_ticks(mut stats: watch::Receiver<TickStats>, state: Arc<AdminState>) {
    while stats.changed().await.is_ok() {
        let ms = stats.borrow_and_update().total_ms as f32;
        let mut ticks = state.ticks.lock().unwrap_or_else(|e| e.into_inner());
        if ticks.len() == HEALTH_TICKS {
            ticks.pop_front();
        }
        ticks.push_back(ms);
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn serve(
    host: String,
    port: u16,
    secret: String,
    jobs: UnboundedSender<EngineJob>,
    node_id: u8,
    members: bool,
    db: DbConfig,
    stats: watch::Receiver<TickStats>,
    connections: ConnectionGuard,
) {
    let listener = match TcpListener::bind((host.as_str(), port)).await {
        Ok(l) => l,
        Err(e) => {
            warn!("Failed to bind the admin API on: {}:{} - {}", host, port, e);
            return;
        }
    };
    info!("Admin API available at: http://{}:{}", host, port);
    let state = Arc::new(AdminState {
        verifier: Verifier::new(&secret),
        jobs,
        node_id,
        members,
        started: Instant::now(),
        db: Db::connect_lazy(&db, LOOKUP_CONNECTIONS),
        ticks: Mutex::new(VecDeque::with_capacity(HEALTH_TICKS)),
        connections,
        sys: Mutex::new(System::new()),
        pid: sysinfo::get_current_pid().ok(),
        reboot_reason: Mutex::default(),
    });
    tokio::spawn(record_ticks(stats, state.clone()));
    let router = Router::new()
        .route(path::STATUS, get(status))
        .route(path::PLAYERS, get(players))
        .route(path::REBOOT, post(reboot))
        .route(path::CANCEL_REBOOT, post(cancel_reboot))
        .route(path::BROADCAST, post(broadcast))
        .route(path::KICK, post(kick))
        .route(path::MODERATE, post(moderate))
        .route(path::MAINTENANCE, post(maintenance))
        .route(path::CONTROLS, post(set_controls))
        .route(path::PLAYER, post(player_lookup))
        .route(path::MODERATION, post(moderation_lookup))
        .route(path::SANCTIONS, post(sanctions_lookup))
        .route(path::CHAT, post(chat_lookup))
        .route(path::REPORTS, post(reports_lookup))
        .route(path::TIMELINE, post(timeline_lookup))
        .route(path::LINKS, post(links_lookup))
        .route(path::REPLAYS, post(replays_lookup))
        .route(path::REPLAY, post(replay_lookup))
        .route(path::REPLAY_DATA, post(replay_data))
        .route(path::PLAYER_SHEET, post(player_sheet))
        .route(path::PLAYER_EDIT, post(player_edit))
        .route(path::OBJ_SEARCH, post(obj_search))
        .layer(middleware::from_fn_with_state(state.clone(), authenticate))
        .with_state(state);
    if let Err(e) = axum::serve(listener, router).await {
        warn!("Admin API stopped: {}", e);
    }
}

/// Refuses any request that isn't signed with the shared secret, is stale,
/// or replays a nonce already used.
async fn authenticate(State(state): State<Arc<AdminState>>, req: Request, next: Next) -> Response {
    let (parts, body) = req.into_parts();
    let Ok(body) = axum::body::to_bytes(body, MAX_BODY).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let path = parts
        .uri
        .path_and_query()
        .map_or(parts.uri.path(), |p| p.as_str());
    let header = |name: &str| parts.headers.get(name).and_then(|v| v.to_str().ok());
    if let Err(reason) = state
        .verifier
        .check(header, parts.method.as_str(), path, &body)
    {
        warn!("Admin API refused {} {}: {}", parts.method, path, reason);
        return StatusCode::UNAUTHORIZED.into_response();
    }
    next.run(Request::from_parts(parts, Body::from(body))).await
}

/// Runs `f` on the engine between ticks and returns what it does.
async fn on_engine<T: Send + 'static>(
    state: &AdminState,
    f: impl FnOnce(&mut Engine) -> T + Send + 'static,
) -> Result<T, StatusCode> {
    let (tx, rx) = oneshot::channel();
    state
        .jobs
        .send(Box::new(move |engine| {
            let _ = tx.send(f(engine));
        }))
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    match timeout(ENGINE_TIMEOUT, rx).await {
        Ok(Ok(value)) => Ok(value),
        _ => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// `300` -> `5:00`, `3725` -> `1:02:05`.
fn clock(secs: u32) -> String {
    let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Seconds a game tick lasts.
const TICK_SECS: f64 = 0.6;

/// A staff-supplied line with control characters dropped, or `None` if
/// nothing is left.
fn clean_line(text: &str) -> Option<String> {
    let text: String = text.trim().chars().filter(|c| !c.is_control()).collect();
    (!text.is_empty()).then_some(text)
}

fn world_status(state: &AdminState, engine: &Engine) -> WorldStatus {
    let players = engine
        .player_list
        .players
        .iter()
        .flatten()
        .filter(|active| !active.player.bot)
        .count() as u32;
    WorldStatus {
        node_id: state.node_id,
        revision: REVISION.to_owned(),
        members: state.members,
        players,
        uptime_secs: state.started.elapsed().as_secs(),
        reboot_in_secs: engine
            .is_pending_shutdown()
            .then(|| (engine.shutdown_ticks_remaining().max(0) as f64 * TICK_SECS) as u32),
        shutting_down: engine.is_shutdown(),
        maintenance: engine.maintenance_mode,
        reboot_reason: engine
            .is_pending_shutdown()
            .then(|| state.reboot_reason.lock().ok()?.clone())
            .flatten(),
        health: Some(state.health()),
        controls: Some(WorldControls {
            switches: engine.controls.patterns().to_vec(),
            min_account_age_hours: engine.controls.min_account_age_hours,
        }),
    }
}

async fn status(State(state): State<Arc<AdminState>>) -> Result<Json<WorldStatus>, StatusCode> {
    let s = state.clone();
    on_engine(&state, move |engine| world_status(&s, engine))
        .await
        .map(Json)
}

async fn players(
    State(state): State<Arc<AdminState>>,
) -> Result<Json<Vec<OnlinePlayer>>, StatusCode> {
    on_engine(&state, |engine| {
        engine
            .player_list
            .players
            .iter()
            .flatten()
            .filter(|active| !active.player.bot)
            .map(|active| {
                let coord = active.player.pathing.coord;
                OnlinePlayer {
                    name: to_screen_name(&active.player.uid.username()),
                    staff_level: active.player.staff_mod_level as u8,
                    x: coord.x(),
                    z: coord.z(),
                    level: coord.y(),
                    muted: active.is_muted(),
                }
            })
            .collect()
    })
    .await
    .map(Json)
}

async fn reboot(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<Reboot>,
) -> Result<Json<Outcome>, StatusCode> {
    if req.seconds > MAX_REBOOT_SECS {
        return Ok(Json(Outcome::refused(format!(
            "A reboot can be at most {MAX_REBOOT_SECS} seconds away."
        ))));
    }
    let reason = req.reason.as_deref().and_then(clean_line);
    if reason
        .as_ref()
        .is_some_and(|r| r.chars().count() > MAX_REASON_CHARS)
    {
        return Ok(Json(Outcome::refused(format!(
            "A reboot reason can be at most {MAX_REASON_CHARS} characters."
        ))));
    }
    let s = state.clone();
    on_engine(&state, move |engine| {
        if engine.is_shutdown() {
            return Outcome::refused("The world is already shutting down.");
        }
        let clocks = (u64::from(req.seconds) * 1000).div_ceil(600);
        engine.reboot_timer(clocks);
        let message = match &reason {
            Some(reason) => format!("Rebooting in {}: {reason}", clock(req.seconds)),
            None => format!("Rebooting in {}.", clock(req.seconds)),
        };
        // Players see the countdown either way; a reason also goes out as a
        // message beside it.
        if reason.is_some() {
            engine.broadcast(&message);
        }
        *s.reboot_reason.lock().unwrap_or_else(|e| e.into_inner()) = reason;
        info!(
            "{} scheduled a reboot in {}s from the admin console",
            req.actor.name, req.seconds
        );
        Outcome::ok(message)
    })
    .await
    .map(Json)
}

async fn cancel_reboot(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<CancelReboot>,
) -> Result<Json<Outcome>, StatusCode> {
    let s = state.clone();
    on_engine(&state, move |engine| {
        if engine.is_shutdown() {
            Outcome::refused("The reboot has already begun.")
        } else if engine.cancel_reboot() {
            *s.reboot_reason.lock().unwrap_or_else(|e| e.into_inner()) = None;
            info!(
                "{} cancelled the reboot from the admin console",
                req.actor.name
            );
            Outcome::ok("Reboot cancelled.")
        } else {
            Outcome::refused("No reboot is scheduled.")
        }
    })
    .await
    .map(Json)
}

async fn broadcast(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<Broadcast>,
) -> Result<Json<Outcome>, StatusCode> {
    let message: String = req
        .message
        .trim()
        .chars()
        .filter(|&c| c == '\n' || !c.is_control())
        .collect();
    if message.is_empty() || message.chars().count() > MAX_BROADCAST_CHARS {
        return Ok(Json(Outcome::refused(format!(
            "A broadcast must be 1 to {MAX_BROADCAST_CHARS} characters."
        ))));
    }
    on_engine(&state, move |engine| {
        engine.broadcast(&message);
        info!(
            "{} broadcast from the admin console: {}",
            req.actor.name, message
        );
        Outcome::ok("Sent.")
    })
    .await
    .map(Json)
}

async fn kick(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<Kick>,
) -> Result<Json<Outcome>, StatusCode> {
    let target37 = to_userhash(&req.name);
    if !is_valid_userhash(target37) {
        return Ok(Json(Outcome::refused("That isn't a valid name.")));
    }
    on_engine(&state, move |engine| {
        let name = to_screen_name(&req.name);
        let Some(target) = engine
            .find_pid_by_user37(target37)
            .and_then(|pid| engine.get_player_mut(pid))
        else {
            return Outcome::refused(format!("{name} isn't on this world."));
        };
        if target.player.staff_mod_level as u8 >= req.actor.staff_level {
            return Outcome::refused(format!("{name} is staff of your level or above."));
        }
        if !target.player.logout_sent {
            target.logout();
        }
        info!("{} kicked {} from the admin console", req.actor.name, name);
        Outcome::ok(format!("Kicked {name}."))
    })
    .await
    .map(Json)
}

async fn moderate(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<Moderate>,
) -> Result<Json<ModOutcome>, StatusCode> {
    let target37 = to_userhash(&req.name);
    let hours = req.hours.unwrap_or(0);
    let action = match req.action {
        rs_admin_api::ModAction::Mute => ModAction::Mute { hours },
        rs_admin_api::ModAction::Unmute => ModAction::Unmute,
        rs_admin_api::ModAction::Ban => ModAction::Ban { hours },
        rs_admin_api::ModAction::Unban => ModAction::Unban,
    };
    if !is_valid_userhash(target37)
        || (action.hours().is_some() && !(1..=MAX_MOD_HOURS).contains(&hours))
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let reason = req
        .reason
        .as_deref()
        .and_then(clean_line)
        .map(|r| r.chars().take(MAX_MOD_REASON_CHARS).collect());
    let moderator37 = to_userhash(&req.actor.name);
    let by_level = req.actor.staff_level;
    let (reply, outcome) = oneshot::channel();
    let sent = on_engine(&state, move |engine| match &engine.db_tx {
        Some(tx) => tx
            .send(DbRequest::Moderate {
                moderator37,
                by_level,
                target37,
                action,
                reason,
                reply: Some(reply),
            })
            .is_ok(),
        None => false,
    })
    .await?;
    if !sent {
        return Ok(Json(ModOutcome::Offline));
    }
    let outcome = match timeout(ENGINE_TIMEOUT, outcome).await {
        Ok(Ok(rs_engine::ModOutcome::Done)) => ModOutcome::Done,
        Ok(Ok(rs_engine::ModOutcome::Refused)) => ModOutcome::Refused,
        Ok(Ok(rs_engine::ModOutcome::Offline)) | Ok(Err(_)) => ModOutcome::Offline,
        Err(_) => ModOutcome::Pending,
    };
    info!(
        "{} ran {} on {} from the admin console: {:?}",
        req.actor.name,
        action.name(),
        req.name,
        outcome
    );
    Ok(Json(outcome))
}

async fn maintenance(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<Maintenance>,
) -> Result<Json<Outcome>, StatusCode> {
    on_engine(&state, move |engine| {
        if engine.maintenance_mode == req.enabled {
            return Outcome::ok(if req.enabled {
                "Logins were already closed."
            } else {
                "Logins were already open."
            });
        }
        engine.maintenance_mode = req.enabled;
        info!(
            "{} {} logins from the admin console",
            req.actor.name,
            if req.enabled { "closed" } else { "reopened" }
        );
        Outcome::ok(if req.enabled {
            "Logins closed. Players already on stay, and staff can still log in."
        } else {
            "Logins open again."
        })
    })
    .await
    .map(Json)
}

/// Replaces the kill switches and login lockdown. Refuses (changing nothing)
/// a pattern that is empty, too long, or matches too many scripts.
async fn set_controls(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<SetControls>,
) -> Result<Json<Result<ControlsSet, String>>, StatusCode> {
    let controls = req.controls;
    if controls.switches.len() > controls::MAX_SWITCHES {
        return Ok(Json(Err(format!(
            "A world takes at most {} kill switches.",
            controls::MAX_SWITCHES
        ))));
    }
    if let Some(bad) = controls
        .switches
        .iter()
        .find(|p| p.trim().is_empty() || p.chars().count() > MAX_PATTERN_CHARS)
    {
        return Ok(Json(Err(format!(
            "\"{bad}\" isn't a pattern: they're 1 to {MAX_PATTERN_CHARS} characters."
        ))));
    }
    if controls.min_account_age_hours > MAX_LOCKDOWN_HOURS {
        return Ok(Json(Err(format!(
            "A lockdown can turn away accounts at most {MAX_LOCKDOWN_HOURS} hours old."
        ))));
    }
    let actor = req.actor.name;
    on_engine(&state, move |engine| {
        let counts: Vec<usize> = controls
            .switches
            .iter()
            .map(|p| Controls::matches(p, &engine.scripts))
            .collect();
        if let Some((p, n)) = controls
            .switches
            .iter()
            .zip(&counts)
            .find(|&(_, &n)| n > MAX_SWITCH_SCRIPTS)
        {
            return Err(format!(
                "\"{p}\" matches {n} scripts, more than the {MAX_SWITCH_SCRIPTS} a switch may turn off."
            ));
        }
        let changed = engine.controls.patterns() != controls.switches.as_slice()
            || engine.controls.min_account_age_hours != controls.min_account_age_hours;
        engine.controls.min_account_age_hours = controls.min_account_age_hours;
        let matched = engine
            .controls
            .set_switches(controls.switches.clone(), &engine.scripts);
        if changed {
            info!(
                "{} set the kill switches to {:?} and the login lockdown to {}h from the admin console",
                actor, controls.switches, controls.min_account_age_hours
            );
        }
        Ok(ControlsSet {
            matched: matched.into_iter().map(|n| n as u32).collect(),
        })
    })
    .await
    .map(Json)
}

/// A lookup's database failure: logged, and answered as the database being
/// out of reach.
fn db_failed(e: rs_db::Error) -> StatusCode {
    warn!("Admin lookup failed: {}", e);
    StatusCode::BAD_GATEWAY
}

/// `name` as a base37 name hash, if it could be a player's.
fn name_hash(name: &str) -> Result<u64, StatusCode> {
    let hash = to_userhash(name);
    if is_valid_userhash(hash) {
        Ok(hash)
    } else {
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    }
}

fn node(id: Option<i16>) -> Option<u8> {
    id.and_then(|id| u8::try_from(id).ok())
}

async fn player_lookup(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<PlayerLookup>,
) -> Result<Json<Option<PlayerRecord>>, StatusCode> {
    let record = state
        .db
        .player_record(name_hash(&req.name)?)
        .await
        .map_err(db_failed)?;
    Ok(Json(record.map(|p| PlayerRecord {
        name: p.display_name,
        staff_level: p.staff_mod_level.clamp(0, 255) as u8,
        registered_ms: p.registered_ms,
        online_node: node(p.online_node),
        online_since_ms: p.online_since_ms,
        last_node: node(p.last_logout_node),
        last_seen_ms: p.last_logout_ms,
        muted_until_ms: p.muted_until_ms,
        banned_until_ms: p.banned_until_ms,
    })))
}

async fn moderation_lookup(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<ModerationLookup>,
) -> Result<Json<Vec<ModerationRecord>>, StatusCode> {
    let target = req.name.as_deref().map(name_hash).transpose()?;
    let rows = state
        .db
        .moderation_history(
            target,
            req.before,
            i64::from(req.limit.clamp(1, MAX_LOOKUP_ROWS)),
        )
        .await
        .map_err(db_failed)?;
    Ok(Json(
        rows.into_iter()
            .map(|m| ModerationRecord {
                id: m.id,
                moderator: m.moderator,
                target: m.target,
                action: m.action,
                hours: m.hours,
                node: node(Some(m.node_id)).unwrap_or_default(),
                at_ms: m.at_ms,
                reason: m.reason,
            })
            .collect(),
    ))
}

async fn sanctions_lookup(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<SanctionsLookup>,
) -> Result<Json<Vec<Sanction>>, StatusCode> {
    let rows = state
        .db
        .sanctioned_players(i64::from(req.limit.clamp(1, MAX_LOOKUP_ROWS)))
        .await
        .map_err(db_failed)?;
    Ok(Json(
        rows.into_iter()
            .map(|s| Sanction {
                name: s.display_name,
                muted_until_ms: s.muted_until_ms,
                banned_until_ms: s.banned_until_ms,
                last_action: s.last_action,
                moderator: s.moderator,
                at_ms: s.at_ms,
            })
            .collect(),
    ))
}

async fn chat_lookup(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<ChatLookup>,
) -> Result<Json<Vec<ChatRecord>>, StatusCode> {
    let sender = req.name.as_deref().map(name_hash).transpose()?;
    let rows = state
        .db
        .chat_history(
            sender,
            req.node.map(i16::from),
            req.before,
            i64::from(req.limit.clamp(1, MAX_LOOKUP_ROWS)),
        )
        .await
        .map_err(db_failed)?;
    Ok(Json(
        rows.into_iter()
            .map(|c| ChatRecord {
                id: c.id,
                from: c.sender,
                to: c
                    .to_name_hash
                    .map(|h| to_screen_name(&to_raw_username(h as u64))),
                node: node(Some(c.node_id)).unwrap_or_default(),
                x: c.x as u16,
                z: c.z as u16,
                level: c.y as u8,
                message: c.message,
                at_ms: c.at_ms,
            })
            .collect(),
    ))
}

async fn reports_lookup(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<ReportLookup>,
) -> Result<Json<Vec<ReportRecord>>, StatusCode> {
    let offender = req.offender.as_deref().map(name_hash).transpose()?;
    let rows = state
        .db
        .abuse_reports(
            offender,
            req.before,
            i64::from(req.limit.clamp(1, MAX_LOOKUP_ROWS)),
        )
        .await
        .map_err(db_failed)?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ReportRecord {
                id: r.id,
                reporter: r.reporter,
                offender: r.offender.unwrap_or_else(|| {
                    to_screen_name(&to_raw_username(r.offender_name_hash as u64))
                }),
                reason: r.reason.clamp(0, 255) as u8,
                node: node(Some(r.node_id)).unwrap_or_default(),
                x: r.x as u16,
                z: r.z as u16,
                level: r.y as u8,
                muted: r.muted,
                at_ms: r.at_ms,
            })
            .collect(),
    ))
}

/// The most timeline rows a page returns.
const MAX_TIMELINE_ROWS: u32 = 200;

async fn timeline_lookup(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<TimelineLookup>,
) -> Result<Json<Vec<TimelineEntry>>, StatusCode> {
    let player = name_hash(&req.name)?;
    let before = req.before.map(|c| TimelineCursor {
        at_ms: c.at_ms,
        kind: c.kind,
        id: c.id,
    });
    let rows = state
        .db
        .timeline(
            player,
            &req.kinds,
            before.as_ref(),
            i64::from(req.limit.clamp(1, MAX_TIMELINE_ROWS)),
        )
        .await
        .map_err(db_failed)?;
    Ok(Json(
        rows.into_iter()
            .map(|r| TimelineEntry {
                kind: r.kind,
                id: r.id.to_string(),
                at_ms: r.at_ms,
                node: node(r.node_id),
                x: r.x.map(|x| x as u16),
                z: r.z.map(|z| z as u16),
                level: r.y.map(|y| y as u8),
                event_type: r.event_type,
                text: r.text,
                other: r.other.or_else(|| {
                    r.other_hash
                        .map(|h| to_screen_name(&to_raw_username(h as u64)))
                }),
                count: r.count,
                value: r.value,
                hours: r.hours,
                detail: r.detail,
            })
            .collect(),
    ))
}

async fn links_lookup(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<LinksLookup>,
) -> Result<Json<Links>, StatusCode> {
    let player = name_hash(&req.name)?;
    let limit = i64::from(req.limit.clamp(1, MAX_LINKS));
    let addresses = state
        .db
        .login_addresses(player, limit)
        .await
        .map_err(db_failed)?;
    let accounts = state
        .db
        .linked_accounts(player, limit)
        .await
        .map_err(db_failed)?;
    Ok(Json(Links {
        addresses: addresses
            .into_iter()
            .map(|a| LoginAddress {
                ip: a.ip,
                logins: a.logins,
                first_ms: a.first_ms,
                last_ms: a.last_ms,
                others: a.others,
            })
            .collect(),
        accounts: accounts
            .into_iter()
            .map(|a| LinkedAccount {
                name: a.display_name,
                staff_level: a.staff_mod_level.clamp(0, 255) as u8,
                shared: a.shared,
                logins: a.logins,
                last_ms: a.last_ms,
                muted_until_ms: a.muted_until_ms,
                banned_until_ms: a.banned_until_ms,
            })
            .collect(),
    }))
}

fn replay_summary(r: rs_db::ReplayRecord) -> ReplaySummary {
    ReplaySummary {
        id: r.id.to_string(),
        player: r.player,
        node: node(Some(r.node_id)).unwrap_or_default(),
        revision: r.revision,
        started_ms: r.started_ms,
        ended_ms: r.ended_ms,
        updated_ms: r.updated_ms,
        duration_ms: r.duration_ms.max(0) as u32,
        bytes: r.bytes.max(0) as u64,
        complete: r.complete,
        score: r.score,
        flags: r.flags,
        stats: r.stats,
    }
}

fn replay_id(id: &str) -> Result<i64, StatusCode> {
    id.parse().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
}

async fn replays_lookup(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<ReplayLookup>,
) -> Result<Json<Vec<ReplaySummary>>, StatusCode> {
    let player = req.name.as_deref().map(name_hash).transpose()?;
    let rows = state
        .db
        .replays(
            player,
            req.flagged,
            req.before,
            i64::from(req.limit.clamp(1, MAX_LOOKUP_ROWS)),
        )
        .await
        .map_err(db_failed)?;
    Ok(Json(rows.into_iter().map(replay_summary).collect()))
}

async fn replay_lookup(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<ReplayGet>,
) -> Result<Json<Option<ReplaySummary>>, StatusCode> {
    let id = replay_id(&req.id)?;
    let record = state.db.replay(id).await.map_err(db_failed)?;
    Ok(Json(record.map(replay_summary)))
}

/// The replay file for a recording ([`playback`]). Only a world of the
/// revision it was recorded at can read it.
async fn replay_data(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<ReplayGet>,
) -> Result<Response, StatusCode> {
    let id = replay_id(&req.id)?;
    let Some(record) = state.db.replay(id).await.map_err(db_failed)? else {
        return Err(StatusCode::NOT_FOUND);
    };
    if record.revision != REVISION {
        return Err(StatusCode::CONFLICT);
    }
    let chunks = state.db.replay_chunks(id).await.map_err(db_failed)?;
    let mut extra = serde_json::Map::new();
    extra.insert("id".into(), record.id.to_string().into());
    extra.insert("player".into(), record.player.into());
    extra.insert("node".into(), record.node_id.into());
    extra.insert("revision".into(), record.revision.into());
    extra.insert("startedMs".into(), record.started_ms.into());
    extra.insert("complete".into(), record.complete.into());
    let file =
        tokio::task::spawn_blocking(move || playback::build(&playback::inflate(&chunks), extra))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        file,
    )
        .into_response())
}

/// Player editing needs staff level 3 (developers), whatever the console
/// allowed.
const DEVELOPER: u8 = 3;

/// The most items a search returns.
const MAX_OBJ_RESULTS: usize = 30;

fn sheet(name: String, sheet: edit::Sheet) -> PlayerSheet {
    PlayerSheet {
        name,
        online: sheet.online,
        x: sheet.x,
        z: sheet.z,
        level: sheet.level,
        stats: sheet
            .stats
            .into_iter()
            .map(|s| StatLine {
                id: s.id,
                name: s.name,
                level: s.level,
                base: s.base,
                xp: s.xp,
            })
            .collect(),
        invs: sheet
            .invs
            .into_iter()
            .map(|inv| InvSheet {
                name: inv.name,
                size: inv.size,
                items: inv
                    .items
                    .into_iter()
                    .map(|i| ItemLine {
                        slot: i.slot,
                        id: i.id,
                        name: i.name,
                        count: i.count,
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// A player's stats, items and position: live if they're on this world,
/// otherwise from their save.
async fn player_sheet(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<SheetLookup>,
) -> Result<Json<Option<PlayerSheet>>, StatusCode> {
    let user37 = name_hash(&req.name)?;
    if let Some(live) = on_engine(&state, move |engine| engine.player_sheet(user37)).await? {
        return Ok(Json(Some(sheet(to_screen_name(&req.name), live))));
    }
    let Some(record) = state.db.player_record(user37).await.map_err(db_failed)? else {
        return Ok(Json(None));
    };
    let save = state.db.load_save(user37).await.map_err(db_failed)?;
    let profile = profile_from_save(save.unwrap_or_default());
    let saved = on_engine(&state, move |engine| engine.profile_sheet(&profile)).await?;
    Ok(Json(Some(sheet(record.display_name, saved))))
}

/// Items by name, config name or id.
async fn obj_search(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<ObjSearch>,
) -> Result<Json<Vec<ObjLine>>, StatusCode> {
    let found = on_engine(&state, move |engine| {
        engine.find_objs(&req.query, MAX_OBJ_RESULTS)
    })
    .await?;
    Ok(Json(
        found
            .into_iter()
            .map(|o| ObjLine {
                id: o.id,
                name: o.name,
                debugname: o.debugname,
                stackable: o.stackable,
            })
            .collect(),
    ))
}

fn change(edit: PlayerEdit) -> Change {
    match edit {
        PlayerEdit::SetStat { stat, level } => Change::SetStat { stat, level },
        PlayerEdit::Give { inv, obj, count } => Change::Give { inv, obj, count },
        PlayerEdit::Take { inv, obj, count } => Change::Take { inv, obj, count },
        PlayerEdit::Move { x, z, level } => Change::Move { x, z, level },
    }
}

/// Changes a player: live if they're on this world, otherwise their save,
/// under this world's claim on their session (see [`rs_engine::dev`]).
async fn player_edit(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<PlayerEditRequest>,
) -> Result<Json<Outcome>, StatusCode> {
    let user37 = name_hash(&req.name)?;
    if req.actor.staff_level < DEVELOPER {
        return Ok(Json(Outcome::refused(
            "Only developers can change players.",
        )));
    }
    // Its own task, so a dropped request can't stop an edit half way and
    // leave the claim held.
    tokio::spawn(edit_player(state, user37, req))
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Ends an offline edit however it finishes: releases the claim unless the
/// edited save was stored (which ended it).
struct EditHold {
    jobs: UnboundedSender<EngineJob>,
    user37: u64,
    stored: bool,
}

impl Drop for EditHold {
    fn drop(&mut self) {
        let (user37, stored) = (self.user37, self.stored);
        let _ = self.jobs.send(Box::new(move |engine| {
            engine.end_offline_edit(user37, stored);
        }));
    }
}

async fn edit_player(state: Arc<AdminState>, user37: u64, req: PlayerEditRequest) -> Outcome {
    let name = to_screen_name(&req.name);
    let outcome = edit_player_with(&state, user37, &name, Arc::new(change(req.edit))).await;
    if outcome.ok {
        info!(
            "{} changed {} from the admin console: {}",
            req.actor.name, name, outcome.message
        );
    }
    outcome
}

/// What [`Engine::edit_player`] or [`Engine::begin_offline_edit`] made of an edit.
enum Begun {
    Live(Result<String, String>),
    Offline(oneshot::Receiver<LoadResult>),
}

async fn edit_player_with(
    state: &AdminState,
    user37: u64,
    name: &str,
    change: Arc<Change>,
) -> Outcome {
    const UNANSWERED: &str = "The world didn't answer in time; nothing was changed.";
    let live = change.clone();
    let begun = on_engine(state, move |engine| {
        match engine.edit_player(user37, &live) {
            Some(done) => Ok(Begun::Live(done)),
            None => engine.begin_offline_edit(user37).map(Begun::Offline),
        }
    })
    .await;
    let loading = match begun {
        Err(_) => return Outcome::refused(UNANSWERED),
        Ok(Err(refused)) | Ok(Ok(Begun::Live(Err(refused)))) => return Outcome::refused(refused),
        Ok(Ok(Begun::Live(Ok(done)))) => return Outcome::ok(done),
        Ok(Ok(Begun::Offline(loading))) => loading,
    };
    let mut hold = EditHold {
        jobs: state.jobs.clone(),
        user37,
        stored: false,
    };

    let profile = match timeout(ENGINE_TIMEOUT, loading).await {
        Ok(Ok(LoadResult::Loaded(Some(profile)))) => profile,
        Ok(Ok(LoadResult::Loaded(None))) => {
            return Outcome::refused(format!(
                "{name} has never logged in, so there's no save to change yet."
            ));
        }
        Ok(Ok(LoadResult::Busy)) => {
            return Outcome::refused(format!("{name} is logged in on another world; try again."));
        }
        Ok(Ok(LoadResult::Unregistered)) => {
            return Outcome::refused(format!("No player named {name} is registered."));
        }
        _ => return Outcome::refused("The game database didn't answer; nothing was changed."),
    };

    let edited = on_engine(state, move |engine| {
        let mut profile = *profile;
        let message = engine.edit_profile(&mut profile, &change)?;
        let stored = engine
            .store_offline_edit(user37, profile)
            .ok_or_else(|| "This world's database is down; nothing was changed.".to_owned())?;
        Ok::<_, String>((message, stored))
    })
    .await;
    let (message, stored) = match edited {
        Err(_) => return Outcome::refused(UNANSWERED),
        Ok(Err(refused)) => return Outcome::refused(refused),
        Ok(Ok(edited)) => edited,
    };
    match timeout(ENGINE_TIMEOUT, stored).await {
        Ok(Ok(SaveOutcome::Stored)) => {
            hold.stored = true;
            Outcome::ok(message)
        }
        Ok(Ok(_)) => Outcome::refused("Another world took their session; nothing was changed."),
        _ => Outcome::refused(
            "The game database didn't answer; the change may not have been saved. Look them up again.",
        ),
    }
}
