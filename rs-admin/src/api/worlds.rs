use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::worlds::{Outcome, Reply, WorldConfig, WorldView};
use axum::Json;
use axum::extract::{Path, State};
use rs_admin_api::{
    Broadcast, CancelReboot, Maintenance, OnlinePlayer, Reboot, ReloadScripts, ScriptsReloaded,
    path,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// The furthest ahead a reboot may land on any world, stagger included:
/// what a world accepts (ten hours).
const MAX_REBOOT_SECS: u32 = 36_000;
const MAX_STAGGER_SECS: u32 = 3_600;
const MAX_BROADCAST_CHARS: usize = 400;
const MAX_REASON_CHARS: usize = 80;

#[derive(Serialize)]
pub struct WorldsPage {
    worlds: Vec<WorldView>,
    can_command: bool,
}

/// `GET /api/worlds`.
pub async fn list(State(app): State<Arc<App>>, staff: Staff) -> Json<WorldsPage> {
    Json(WorldsPage {
        worlds: app.worlds.views(),
        can_command: staff.require_admin().is_ok(),
    })
}

/// `GET /api/worlds/{node}/players`.
pub async fn players(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Path(node): Path<u8>,
) -> Result<Json<Vec<OnlinePlayer>>, ApiError> {
    let world = app
        .worlds
        .get(node)
        .ok_or_else(|| ApiError::bad_request("No such world."))?;
    let mut players = app
        .worlds
        .players(world)
        .await
        .map_err(|e| ApiError::new(axum::http::StatusCode::BAD_GATEWAY, e))?;
    players.sort_by_key(|p| p.name.to_lowercase());
    Ok(Json(players))
}

/// The worlds named by `nodes`, in that order, each once.
fn targets(app: &App, nodes: &[u8]) -> Result<Vec<WorldConfig>, ApiError> {
    if nodes.is_empty() {
        return Err(ApiError::bad_request("Pick at least one world."));
    }
    let mut targets: Vec<WorldConfig> = Vec::with_capacity(nodes.len());
    for &node in nodes {
        let world = app
            .worlds
            .get(node)
            .ok_or_else(|| ApiError::bad_request(format!("There is no world with node {node}.")))?;
        if !targets.iter().any(|t| t.node == node) {
            targets.push(world.clone());
        }
    }
    Ok(targets)
}

fn world_list(targets: &[WorldConfig]) -> String {
    let ids: Vec<String> = targets.iter().map(|w| w.id.to_string()).collect();
    format!("worlds {}", ids.join(","))
}

/// How a command went everywhere, for the audit trail: `3/4 ok`.
fn tally(replies: &[Reply<rs_admin_api::Outcome>]) -> (bool, String) {
    let ok = replies
        .iter()
        .filter(|r| matches!(&r.result, Outcome::Answered { answer } if answer.ok))
        .count();
    (ok == replies.len(), format!("{ok}/{} ok", replies.len()))
}

#[derive(Serialize)]
pub struct CommandResult {
    replies: Vec<Reply<rs_admin_api::Outcome>>,
    worlds: Vec<WorldView>,
}

async fn finish(
    app: &App,
    targets: &[WorldConfig],
    replies: Vec<Reply<rs_admin_api::Outcome>>,
) -> Json<CommandResult> {
    let nodes: Vec<u8> = targets.iter().map(|w| w.node).collect();
    app.worlds.refresh(Some(&nodes)).await;
    Json(CommandResult {
        replies,
        worlds: app.worlds.views(),
    })
}

#[derive(Deserialize)]
pub struct RebootRequest {
    nodes: Vec<u8>,
    seconds: u32,
    /// Added to the delay of each world after the first, so they don't all
    /// go down (and come back to a login rush) at once.
    #[serde(default)]
    stagger_secs: u32,
    /// Shown to players with the countdown.
    #[serde(default)]
    reason: Option<String>,
}

/// `POST /api/worlds/reboot`.
pub async fn reboot(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<RebootRequest>,
) -> Result<Json<CommandResult>, ApiError> {
    staff.require_admin()?;
    let targets = targets(&app, &req.nodes)?;
    if req.stagger_secs > MAX_STAGGER_SECS {
        return Err(ApiError::bad_request(format!(
            "The stagger can be at most {MAX_STAGGER_SECS} seconds."
        )));
    }
    let last = u64::from(req.seconds) + u64::from(req.stagger_secs) * (targets.len() as u64 - 1);
    if last > u64::from(MAX_REBOOT_SECS) {
        return Err(ApiError::bad_request(
            "The last world would reboot more than 10 hours from now.",
        ));
    }
    let reason = req
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .map(str::to_owned);
    if reason
        .as_ref()
        .is_some_and(|r| r.chars().count() > MAX_REASON_CHARS)
    {
        return Err(ApiError::bad_request(format!(
            "A reason can be at most {MAX_REASON_CHARS} characters."
        )));
    }
    let actor = staff.actor();
    let replies = app
        .worlds
        .command(&targets, path::REBOOT, |i| Reboot {
            actor: actor.clone(),
            seconds: req.seconds + req.stagger_secs * i as u32,
            reason: reason.clone(),
        })
        .await;
    let (ok, summary) = tally(&replies);
    let why = reason.map(|r| format!(" (\"{r}\")")).unwrap_or_default();
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "reboot")
            .target(world_list(&targets))
            .detail(format!(
                "in {}s, stagger {}s{why}: {summary}",
                req.seconds, req.stagger_secs
            ))
            .ok(ok),
    );
    Ok(finish(&app, &targets, replies).await)
}

#[derive(Deserialize)]
pub struct Targets {
    nodes: Vec<u8>,
}

/// `POST /api/worlds/cancel-reboot`.
pub async fn cancel_reboot(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<Targets>,
) -> Result<Json<CommandResult>, ApiError> {
    staff.require_admin()?;
    let targets = targets(&app, &req.nodes)?;
    let actor = staff.actor();
    let replies = app
        .worlds
        .command(&targets, path::CANCEL_REBOOT, |_| CancelReboot {
            actor: actor.clone(),
        })
        .await;
    let (ok, summary) = tally(&replies);
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "cancel_reboot")
            .target(world_list(&targets))
            .detail(summary)
            .ok(ok),
    );
    Ok(finish(&app, &targets, replies).await)
}

#[derive(Deserialize)]
pub struct BroadcastRequest {
    nodes: Vec<u8>,
    message: String,
}

/// `POST /api/worlds/broadcast`.
pub async fn broadcast(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<BroadcastRequest>,
) -> Result<Json<CommandResult>, ApiError> {
    staff.require_admin()?;
    let targets = targets(&app, &req.nodes)?;
    let message = req.message.trim().to_owned();
    if message.is_empty() || message.chars().count() > MAX_BROADCAST_CHARS {
        return Err(ApiError::bad_request(format!(
            "A broadcast must be 1 to {MAX_BROADCAST_CHARS} characters."
        )));
    }
    let actor = staff.actor();
    let replies = app
        .worlds
        .command(&targets, path::BROADCAST, |_| Broadcast {
            actor: actor.clone(),
            message: message.clone(),
        })
        .await;
    let (ok, summary) = tally(&replies);
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "broadcast")
            .target(world_list(&targets))
            .detail(format!("\"{message}\": {summary}"))
            .ok(ok),
    );
    Ok(finish(&app, &targets, replies).await)
}

#[derive(Deserialize)]
pub struct MaintenanceRequest {
    nodes: Vec<u8>,
    enabled: bool,
}

/// `POST /api/worlds/maintenance`: closes worlds to new logins, or opens
/// them again.
pub async fn maintenance(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<MaintenanceRequest>,
) -> Result<Json<CommandResult>, ApiError> {
    staff.require_admin()?;
    let targets = targets(&app, &req.nodes)?;
    let actor = staff.actor();
    let replies = app
        .worlds
        .command(&targets, path::MAINTENANCE, |_| Maintenance {
            actor: actor.clone(),
            enabled: req.enabled,
        })
        .await;
    let (ok, summary) = tally(&replies);
    app.audit.record(
        Event::new(
            Some(staff.name()),
            staff.client.ip,
            if req.enabled {
                "maintenance_on"
            } else {
                "maintenance_off"
            },
        )
        .target(world_list(&targets))
        .detail(summary)
        .ok(ok),
    );
    Ok(finish(&app, &targets, replies).await)
}

/// Time for a world to compile its scripts and swap them in.
const RELOAD_TIMEOUT: Duration = Duration::from_secs(180);
/// The longest reason a script reload is logged with.
const MAX_RELOAD_REASON_CHARS: usize = 300;

#[derive(Deserialize)]
pub struct ReloadRequest {
    #[serde(default)]
    check_only: bool,
    #[serde(default)]
    reason: String,
}

/// `POST /api/worlds/{node}/reload-scripts`: recompiles one world's scripts
/// and, unless only checking, swaps them in: an emergency fix without a
/// restart. Maintainers only, and audited either way.
pub async fn reload_scripts(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(node): Path<u8>,
    Json(req): Json<ReloadRequest>,
) -> Result<Json<ScriptsReloaded>, ApiError> {
    staff.require_maintainer()?;
    let world = app
        .worlds
        .get(node)
        .ok_or_else(|| ApiError::bad_request("No such world."))?
        .clone();
    let reason: String = req
        .reason
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_owned();
    if !req.check_only && reason.is_empty() {
        return Err(ApiError::bad_request("Say what the fix is for."));
    }
    if reason.chars().count() > MAX_RELOAD_REASON_CHARS {
        return Err(ApiError::bad_request(format!(
            "Keep the reason to {MAX_RELOAD_REASON_CHARS} characters."
        )));
    }
    let reply: Result<ScriptsReloaded, String> = app
        .worlds
        .ask_node(
            node,
            path::RELOAD_SCRIPTS,
            &ReloadScripts {
                actor: staff.actor(),
                check_only: req.check_only,
                reason: reason.clone(),
            },
            RELOAD_TIMEOUT,
        )
        .await;
    let (ok, message) = match &reply {
        Ok(r) => (r.ok, r.message.clone()),
        Err(e) => (false, format!("{e}. It may still have happened.")),
    };
    let detail = if reason.is_empty() {
        message
    } else {
        format!("{message} Reason: {reason}")
    };
    app.audit.record(
        Event::new(
            Some(staff.name()),
            staff.client.ip,
            if req.check_only {
                "scripts_checked"
            } else {
                "scripts_reloaded"
            },
        )
        .target(format!("World {}", world.id))
        .detail(detail)
        .ok(ok),
    );
    reply
        .map(Json)
        .map_err(|e| ApiError::new(axum::http::StatusCode::BAD_GATEWAY, format!("{e}.")))
}
