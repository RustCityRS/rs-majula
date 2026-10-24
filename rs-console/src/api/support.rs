use super::edit::{record, their_world, unavailable};
use super::players::{check_case, checked_name, note_on_case, reason_detail};
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::worlds::EDIT_TIMEOUT;
use axum::Json;
use axum::extract::{Query, State};
use rs_admin_api::{
    MAX_RESTORE_ITEMS, ObjLine, ObjSearch, Outcome, PasswordReset, PlayerEdit, PlayerEditRequest,
    PlayerRecord, PlayerSheet, ResetIssued, RestoreItem, Rollback, SavePointLine, SavePointLookup,
    SavePointsLookup, SessionHold, SessionLookup, SessionState, SheetLookup, Unstick, WealthLine,
    WealthLookup, path, utc_time,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// The longest reason given, in characters.
const MAX_REASON_CHARS: usize = 300;

/// What a wealth event's type means, as the content's `^wealth_*`
/// constants have them.
fn wealth_kind(event_type: i16) -> &'static str {
    match event_type {
        5 => "picked up",
        6 => "bought from a shop",
        7 => "sold to a shop",
        8 => "low alched",
        9 => "high alched",
        10 => "got from the party room",
        103 => "dropped",
        104 => "picked up from the floor",
        _ => "had",
    }
}

/// The reason as it's kept: one line, trimmed.
pub(super) fn reason(text: &str, required: bool) -> Result<String, ApiError> {
    let text: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_owned();
    if required && text.is_empty() {
        return Err(ApiError::bad_request("Say why."));
    }
    if text.chars().count() > MAX_REASON_CHARS {
        return Err(ApiError::bad_request(format!(
            "Keep the reason to {MAX_REASON_CHARS} characters."
        )));
    }
    Ok(text)
}

/// The open case a change to what a player owns is made for.
pub(super) fn open_case(app: &App, case: Option<u64>) -> Result<u64, ApiError> {
    let Some(id) = case else {
        return Err(ApiError::bad_request(
            "Pick the case this is for: restorations and rollbacks need one.",
        ));
    };
    match app.cases.get(id) {
        None => Err(ApiError::bad_request(format!("There's no case #{id}."))),
        Some(case) if !case.is_open() => Err(ApiError::bad_request(format!(
            "Case #{id} is closed; reopen it first."
        ))),
        Some(_) => Ok(id),
    }
}

/// The reason with its case, as the world keeps it in the save history.
pub(super) fn with_case(case: Option<u64>, reason: &str) -> String {
    match (case, reason.is_empty()) {
        (Some(id), true) => format!("Case #{id}"),
        (Some(id), false) => format!("Case #{id}: {reason}"),
        (None, _) => reason.to_owned(),
    }
}

pub(super) async fn registered(app: &App, name: &str) -> Result<PlayerRecord, ApiError> {
    record(app, name)
        .await?
        .ok_or_else(|| ApiError::bad_request(format!("No player named {name} is registered.")))
}

/// The world to send a change to: theirs, or any while they're logged out.
pub(super) async fn world_for(app: &App, record: &PlayerRecord) -> Result<u8, ApiError> {
    match their_world(app, record)? {
        Some(node) => Ok(node),
        None => app
            .worlds
            .online()
            .first()
            .map(|w| w.node)
            .ok_or_else(|| unavailable("No world is online to ask".to_owned())),
    }
}

/// Sends a command to one world, never retried elsewhere, so it can't
/// happen twice; an unanswered one may have happened.
pub(super) async fn send_once<B: Serialize>(app: &App, node: u8, path: &str, body: &B) -> Outcome {
    app.worlds
        .ask_node(node, path, body, EDIT_TIMEOUT)
        .await
        .unwrap_or_else(|e: String| {
            Outcome::refused(format!(
                "{e}. It may still have happened; look them up again."
            ))
        })
}

#[derive(Deserialize)]
pub struct NameQuery {
    name: String,
}

#[derive(Serialize)]
pub struct SessionView {
    /// The world whose claim on their session keeps them from logging in
    /// anywhere else.
    owner: Option<OwnerView>,
    /// How the owning world holds it, when the console could ask it.
    hold: Option<SessionHold>,
}

#[derive(Serialize)]
pub struct OwnerView {
    node: u8,
    world: Option<u16>,
    since_ms: Option<i64>,
    heartbeat_age_ms: Option<i64>,
    takeover_after_ms: i64,
    /// Whether the console can reach it.
    reachable: bool,
}

/// `GET /api/support/session?name=`: `null` when no such player is
/// registered.
pub async fn session(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<NameQuery>,
) -> Result<Json<Option<SessionView>>, ApiError> {
    let name = checked_name(&query.name)?;
    let Some(record) = record(&app, &name).await? else {
        return Ok(Json(None));
    };
    let reachable = |node: u8| app.worlds.online().iter().any(|w| w.node == node);
    let ask = SessionLookup { name };
    let owner_node = record.online_node.filter(|&n| reachable(n));
    let state: Option<SessionState> = match owner_node {
        Some(node) => {
            app.worlds
                .ask_node(node, path::SESSION, &ask, EDIT_TIMEOUT)
                .await
        }
        None => app.worlds.ask(path::SESSION, &ask).await,
    }
    .map_err(unavailable)?;
    let Some(state) = state else {
        return Ok(Json(None));
    };
    // What the world that answered holds only matters if it's the owner.
    let hold = state
        .owner
        .as_ref()
        .filter(|o| Some(o.node) == owner_node)
        .map(|_| state.here.clone());
    Ok(Json(Some(SessionView {
        owner: state.owner.map(|o| OwnerView {
            node: o.node,
            world: app.worlds.get(o.node).map(|w| w.id),
            since_ms: o.since_ms,
            heartbeat_age_ms: o.heartbeat_age_ms,
            takeover_after_ms: o.takeover_after_ms,
            reachable: reachable(o.node),
        }),
        hold,
    })))
}

#[derive(Deserialize)]
pub struct HelpRequest {
    name: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    case: Option<u64>,
}

/// `POST /api/support/unstick`: frees the session of a player who can't
/// log in, on the world that owns it, or (when that world can't be reached)
/// on any, which releases it only once the owner has stopped.
pub async fn unstick(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<HelpRequest>,
) -> Result<Json<Outcome>, ApiError> {
    let name = checked_name(&req.name)?;
    let why = reason(&req.reason, false)?;
    check_case(&app, req.case)?;
    let record = registered(&app, &name).await?;
    let body = Unstick {
        actor: staff.actor(),
        name: record.name.clone(),
    };
    let reachable = record
        .online_node
        .filter(|&n| app.worlds.online().iter().any(|w| w.node == n));
    let outcome = match reachable {
        Some(node) => send_once(&app, node, path::UNSTICK, &body).await,
        None => app
            .worlds
            .ask(path::UNSTICK, &body)
            .await
            .unwrap_or_else(|e| Outcome::refused(format!("{e}."))),
    };
    finish(
        &app,
        &staff,
        "session_freed",
        "unstick",
        &record.name,
        req.case,
        &why,
        &outcome,
    );
    Ok(Json(outcome))
}

/// Audits a support action, and notes it on the case it was for.
#[allow(clippy::too_many_arguments)]
fn finish(
    app: &App,
    staff: &Staff,
    audit: &'static str,
    action: &str,
    name: &str,
    case: Option<u64>,
    reason: &str,
    outcome: &Outcome,
) {
    let detail = reason_detail(reason, &outcome.message);
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, audit)
            .target(name)
            .detail(match case {
                Some(id) => format!("Case #{id}. {detail}"),
                None => detail,
            })
            .ok(outcome.ok),
    );
    note_on_case(
        app,
        staff,
        case,
        action,
        name,
        None,
        outcome.ok,
        &outcome.message,
        reason,
    );
}

/// `POST /api/support/send-home`: moves a player stuck where they are to
/// Lumbridge (or Tutorial Island's start, if they're on it): now if they're
/// online, otherwise where they log in. Admins only: moderators change
/// nobody.
pub async fn send_home(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<HelpRequest>,
) -> Result<Json<Outcome>, ApiError> {
    staff.require_admin()?;
    let name = checked_name(&req.name)?;
    let why = reason(&req.reason, false)?;
    check_case(&app, req.case)?;
    let record = registered(&app, &name).await?;
    let node = world_for(&app, &record).await?;
    let outcome = send_once(
        &app,
        node,
        path::PLAYER_EDIT,
        &PlayerEditRequest {
            actor: staff.actor(),
            name: record.name.clone(),
            edit: PlayerEdit::SendHome,
            reason: Some(with_case(req.case, &why)),
        },
    )
    .await;
    finish(
        &app,
        &staff,
        "sent_home",
        "send_home",
        &record.name,
        req.case,
        &why,
        &outcome,
    );
    Ok(Json(outcome))
}

#[derive(Deserialize)]
pub struct ResetRequest {
    name: String,
    /// Stop their current password working now (they're logged out).
    #[serde(default)]
    lock: bool,
    reason: String,
    #[serde(default)]
    case: Option<u64>,
}

/// `POST /api/support/password-reset`: a one-time code for the player to
/// set a new password with on the website. The code is shown once and
/// isn't audited or noted anywhere.
pub async fn password_reset(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<ResetRequest>,
) -> Result<Json<ResetIssued>, ApiError> {
    staff.require_admin()?;
    let name = checked_name(&req.name)?;
    super::access::check_conflict(&app, &staff, &name, "reset their password").await?;
    let why = reason(&req.reason, true)?;
    check_case(&app, req.case)?;
    let record = registered(&app, &name).await?;
    // The world they're on, so a lock logs them out there.
    let node = world_for(&app, &record).await?;
    let issued: ResetIssued = app
        .worlds
        .ask_node(
            node,
            path::PASSWORD_RESET,
            &PasswordReset {
                actor: staff.actor(),
                name: record.name.clone(),
                lock: req.lock,
            },
            EDIT_TIMEOUT,
        )
        .await
        .unwrap_or_else(|e| ResetIssued {
            ok: false,
            message: format!("{e}. A reset may have been issued; issue another to be sure."),
            code: None,
            until_ms: None,
        });
    let summary = Outcome {
        ok: issued.ok,
        message: if issued.ok {
            format!(
                "Issued a password reset code{}.",
                if req.lock {
                    " and locked their old password"
                } else {
                    ""
                }
            )
        } else {
            issued.message.clone()
        },
    };
    finish(
        &app,
        &staff,
        "password_reset",
        "password_reset",
        &record.name,
        req.case,
        &why,
        &summary,
    );
    Ok(Json(issued))
}

/// `GET /api/support/sheet?name=`: a player's stats and items, live or from
/// their save, for restorations.
pub async fn sheet(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<NameQuery>,
) -> Result<Json<Option<PlayerSheet>>, ApiError> {
    staff.require_admin()?;
    let name = checked_name(&query.name)?;
    let Some(record) = record(&app, &name).await? else {
        return Ok(Json(None));
    };
    let ask = SheetLookup { name };
    match their_world(&app, &record)? {
        Some(node) => {
            app.worlds
                .ask_node(node, path::PLAYER_SHEET, &ask, EDIT_TIMEOUT)
                .await
        }
        None => app.worlds.ask(path::PLAYER_SHEET, &ask).await,
    }
    .map(Json)
    .map_err(unavailable)
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    name: String,
    before: Option<i64>,
}

#[derive(Serialize)]
pub struct PointView {
    #[serde(flatten)]
    point: SavePointLine,
    world: Option<u16>,
}

/// `GET /api/support/history?name=&before=`: a player's earlier saves,
/// newest first.
pub async fn history(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<PointView>>, ApiError> {
    staff.require_admin()?;
    let name = checked_name(&query.name)?;
    let points: Vec<SavePointLine> = app
        .worlds
        .ask(
            path::SAVE_POINTS,
            &SavePointsLookup {
                name,
                before: query.before,
                saved_by_ms: None,
                limit: 50,
            },
        )
        .await
        .map_err(unavailable)?;
    Ok(Json(
        points
            .into_iter()
            .map(|point| PointView {
                world: app.worlds.get(point.node).map(|w| w.id),
                point,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct PointQuery {
    name: String,
    id: i64,
}

/// `GET /api/support/history/sheet?name=&id=`: one earlier save as a sheet,
/// `null` if it isn't theirs or was pruned.
pub async fn point_sheet(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<PointQuery>,
) -> Result<Json<Option<PlayerSheet>>, ApiError> {
    staff.require_admin()?;
    let name = checked_name(&query.name)?;
    app.worlds
        .ask(path::SAVE_POINT, &SavePointLookup { name, id: query.id })
        .await
        .map(Json)
        .map_err(unavailable)
}

#[derive(Deserialize)]
pub struct RollbackRequest {
    name: String,
    point: i64,
    reason: String,
    case: Option<u64>,
}

/// `POST /api/support/rollback`: puts an earlier save back in place of
/// their current one, which the world keeps in the history. A player online
/// is logged out first.
pub async fn rollback(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<RollbackRequest>,
) -> Result<Json<Outcome>, ApiError> {
    staff.require_admin()?;
    let name = checked_name(&req.name)?;
    super::access::check_conflict(&app, &staff, &name, "roll back their save").await?;
    let why = reason(&req.reason, true)?;
    let case = open_case(&app, req.case)?;
    let record = registered(&app, &name).await?;
    let node = world_for(&app, &record).await?;
    let outcome = send_once(
        &app,
        node,
        path::ROLLBACK,
        &Rollback {
            actor: staff.actor(),
            name: record.name.clone(),
            point: req.point,
            reason: with_case(Some(case), &why),
        },
    )
    .await;
    finish(
        &app,
        &staff,
        "save_rolled_back",
        "rollback",
        &record.name,
        Some(case),
        &why,
        &outcome,
    );
    Ok(Json(outcome))
}

/// What shows a player had what's restored to them.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Proof {
    /// A wealth event: they picked it up, bought it, or the like.
    Wealth { id: i64 },
    /// An earlier save that had it.
    Point { id: i64 },
}

#[derive(Deserialize)]
pub struct RestoreRequest {
    name: String,
    items: Vec<RestoreItem>,
    reason: String,
    case: Option<u64>,
    #[serde(default)]
    proof: Option<Proof>,
}

/// `POST /api/support/restore`: gives back items a player lost, all of them
/// or none. With proof, every item must be covered by it.
pub async fn restore(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<RestoreRequest>,
) -> Result<Json<Outcome>, ApiError> {
    staff.require_admin()?;
    let name = checked_name(&req.name)?;
    super::access::check_conflict(&app, &staff, &name, "restore their items").await?;
    let why = reason(&req.reason, true)?;
    let case = open_case(&app, req.case)?;
    if req.items.is_empty() || req.items.len() > MAX_RESTORE_ITEMS {
        return Err(ApiError::bad_request(format!(
            "Restore 1 to {MAX_RESTORE_ITEMS} items at a time."
        )));
    }
    if req.items.iter().any(|i| i.count == 0) {
        return Err(ApiError::bad_request("Restore at least 1 of each item."));
    }
    let record = registered(&app, &name).await?;
    let proof = match &req.proof {
        None => None,
        Some(proof) => Some(proven(&app, &record.name, proof, &req.items).await?),
    };
    let why_with_proof = match &proof {
        Some(proof) => format!("{}. Proof: {proof}", why.trim_end_matches('.')),
        None => why.clone(),
    };
    let node = world_for(&app, &record).await?;
    let outcome = send_once(
        &app,
        node,
        path::PLAYER_EDIT,
        &PlayerEditRequest {
            actor: staff.actor(),
            name: record.name.clone(),
            edit: PlayerEdit::Restore { items: req.items },
            reason: Some(with_case(Some(case), &why_with_proof)),
        },
    )
    .await;
    finish(
        &app,
        &staff,
        "items_restored",
        "restore",
        &record.name,
        Some(case),
        &why_with_proof,
        &outcome,
    );
    Ok(Json(outcome))
}

/// The item a debug name is, found the way the editor's search finds it.
async fn obj_named(app: &App, debugname: &str) -> Result<Option<ObjLine>, ApiError> {
    let found: Vec<ObjLine> = app
        .worlds
        .ask(
            path::OBJ_SEARCH,
            &ObjSearch {
                query: debugname.to_owned(),
            },
        )
        .await
        .map_err(unavailable)?;
    Ok(found.into_iter().find(|o| o.debugname == debugname))
}

/// What `proof` shows, once it's checked to cover every item restored.
async fn proven(
    app: &App,
    name: &str,
    proof: &Proof,
    items: &[RestoreItem],
) -> Result<String, ApiError> {
    let mut wanted: HashMap<u16, u64> = HashMap::new();
    for item in items {
        *wanted.entry(item.obj).or_default() += u64::from(item.count);
    }
    match *proof {
        Proof::Wealth { id } => {
            let event: Option<WealthLine> = app
                .worlds
                .ask(
                    path::WEALTH,
                    &WealthLookup {
                        name: name.to_owned(),
                        id,
                    },
                )
                .await
                .map_err(unavailable)?;
            let event = event.ok_or_else(|| {
                ApiError::bad_request(format!("Wealth event {id} isn't one of {name}'s."))
            })?;
            let obj = obj_named(app, &event.item).await?;
            let covered = obj.as_ref().is_some_and(|o| {
                wanted.len() == 1
                    && wanted
                        .get(&o.id)
                        .is_some_and(|&n| n <= event.count.max(0) as u64)
            });
            if !covered {
                return Err(ApiError::bad_request(format!(
                    "The wealth event shows {} x {} only; restore just that, or pick other proof.",
                    event.count, event.item
                )));
            }
            let world = app.worlds.get(event.node).map_or_else(
                || format!("node {}", event.node),
                |w| format!("world {}", w.id),
            );
            Ok(format!(
                "they {} {} x {} on {world} at {}, {} on {} (wealth event {id}).",
                wealth_kind(event.event_type),
                event.count,
                event.item,
                event.x,
                event.z,
                utc_time(event.at_ms)
            ))
        }
        Proof::Point { id } => {
            let sheet: Option<PlayerSheet> = app
                .worlds
                .ask(
                    path::SAVE_POINT,
                    &SavePointLookup {
                        name: name.to_owned(),
                        id,
                    },
                )
                .await
                .map_err(unavailable)?;
            let sheet = sheet.ok_or_else(|| {
                ApiError::bad_request(format!(
                    "Save {id} isn't one of {name}'s, or has been pruned."
                ))
            })?;
            let mut had: HashMap<u16, u64> = HashMap::new();
            for item in sheet.invs.iter().flat_map(|inv| &inv.items) {
                *had.entry(item.id).or_default() += u64::from(item.count);
            }
            if let Some((&obj, &count)) = wanted
                .iter()
                .find(|&(obj, &count)| had.get(obj).copied().unwrap_or(0) < count)
            {
                return Err(ApiError::bad_request(format!(
                    "Their save {id} had {} of item {obj}, fewer than the {count} restored.",
                    had.get(&obj).copied().unwrap_or(0)
                )));
            }
            Ok(format!("their earlier save (point {id}) had them."))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_are_one_trimmed_line() {
        assert_eq!(
            reason("  lost to\nthe bug ", true).unwrap(),
            "lost to the bug"
        );
        assert!(reason("  ", true).is_err());
        assert_eq!(reason("", false).unwrap(), "");
        assert!(reason(&"x".repeat(MAX_REASON_CHARS + 1), false).is_err());
    }

    #[test]
    fn notes_carry_the_case() {
        assert_eq!(with_case(Some(4), "bank bug"), "Case #4: bank bug");
        assert_eq!(with_case(Some(4), ""), "Case #4");
        assert_eq!(with_case(None, "stuck"), "stuck");
    }
}
