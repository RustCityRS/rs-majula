use super::players::checked_name;
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::worlds::EDIT_TIMEOUT;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use rs_admin_api::{
    ObjLine, ObjSearch, Outcome, PlayerEdit, PlayerEditRequest, PlayerLookup, PlayerRecord,
    PlayerSheet, SheetLookup, path,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

fn unavailable(error: String) -> ApiError {
    ApiError::new(StatusCode::BAD_GATEWAY, format!("{error}."))
}

/// The player's game record, which says where they're logged in.
async fn record(app: &App, name: &str) -> Result<Option<PlayerRecord>, ApiError> {
    app.worlds
        .ask(
            path::PLAYER,
            &PlayerLookup {
                name: name.to_owned(),
            },
        )
        .await
        .map_err(unavailable)
}

/// The world to ask about a player: the one they're on, or (`None`) any
/// world while they're logged out.
fn their_world(app: &App, record: &PlayerRecord) -> Result<Option<u8>, ApiError> {
    match record.online_node {
        None => Ok(None),
        Some(node) if app.worlds.online().iter().any(|w| w.node == node) => Ok(Some(node)),
        Some(node) => Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            format!(
                "{} is logged in on a world the console can't reach (node {node}).",
                record.name
            ),
        )),
    }
}

#[derive(Deserialize)]
pub struct NameQuery {
    name: String,
}

#[derive(Serialize)]
pub struct SheetView {
    #[serde(flatten)]
    sheet: PlayerSheet,
    world: Option<u16>,
}

/// `GET /api/players/sheet?name=`: `null` when no such player is registered.
pub async fn player(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<NameQuery>,
) -> Result<Json<Option<SheetView>>, ApiError> {
    staff.require_developer()?;
    let name = checked_name(&query.name)?;
    let Some(record) = record(&app, &name).await? else {
        return Ok(Json(None));
    };
    let ask = SheetLookup { name };
    let sheet: Option<PlayerSheet> = match their_world(&app, &record)? {
        Some(node) => {
            app.worlds
                .ask_node(node, path::PLAYER_SHEET, &ask, EDIT_TIMEOUT)
                .await
        }
        None => app.worlds.ask(path::PLAYER_SHEET, &ask).await,
    }
    .map_err(unavailable)?;
    Ok(Json(sheet.map(|sheet| {
        SheetView {
            world: sheet
                .online
                .then(|| {
                    record
                        .online_node
                        .and_then(|n| app.worlds.get(n))
                        .map(|w| w.id)
                })
                .flatten(),
            sheet,
        }
    })))
}

#[derive(Deserialize)]
pub struct EditRequest {
    name: String,
    edit: PlayerEdit,
}

fn describe(edit: &PlayerEdit) -> String {
    match edit {
        PlayerEdit::SetStat { stat, level } => format!("stat {stat} to {level}"),
        PlayerEdit::Give { inv, obj, count } => format!("give {count} x obj {obj} into {inv}"),
        PlayerEdit::Take { inv, obj, count } => format!("take {count} x obj {obj} from {inv}"),
        PlayerEdit::Move { x, z, level } => format!("move to {x}, {z}, level {level}"),
    }
}

/// `POST /api/players/edit`: sent to one world only, never retried on another,
/// so a change can't happen twice.
pub async fn edit(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<EditRequest>,
) -> Result<Json<Outcome>, ApiError> {
    staff.require_developer()?;
    let name = checked_name(&req.name)?;
    if let PlayerEdit::Give { count: 0, .. } | PlayerEdit::Take { count: 0, .. } = req.edit {
        return Err(ApiError::bad_request("Give or take at least 1."));
    }
    let Some(record) = record(&app, &name).await? else {
        return Err(ApiError::bad_request(format!(
            "No player named {name} is registered."
        )));
    };
    let node = match their_world(&app, &record)? {
        Some(node) => node,
        None => app
            .worlds
            .online()
            .first()
            .map(|w| w.node)
            .ok_or_else(|| unavailable("No world is online to ask".to_owned()))?,
    };
    let what = describe(&req.edit);
    let outcome: Result<Outcome, String> = app
        .worlds
        .ask_node(
            node,
            path::PLAYER_EDIT,
            &PlayerEditRequest {
                actor: staff.actor(),
                name: record.name.clone(),
                edit: req.edit,
            },
            EDIT_TIMEOUT,
        )
        .await;
    let outcome = outcome.unwrap_or_else(|e| {
        Outcome::refused(format!(
            "{e}. The change may not have been made; look them up again."
        ))
    });
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "player_edited")
            .target(&record.name)
            .detail(format!("{what}: {}", outcome.message))
            .ok(outcome.ok),
    );
    Ok(Json(outcome))
}

#[derive(Deserialize)]
pub struct ObjQuery {
    q: String,
}

/// `GET /api/objs?q=`: items by name, config name or id.
pub async fn objs(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ObjQuery>,
) -> Result<Json<Vec<ObjLine>>, ApiError> {
    staff.require_developer()?;
    let q = query.q.trim();
    if q.is_empty() || q.chars().count() > 40 {
        return Ok(Json(Vec::new()));
    }
    app.worlds
        .ask(
            path::OBJ_SEARCH,
            &ObjSearch {
                query: q.to_owned(),
            },
        )
        .await
        .map(Json)
        .map_err(unavailable)
}
