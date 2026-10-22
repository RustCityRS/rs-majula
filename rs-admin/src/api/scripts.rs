use super::lookup::unavailable;
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::script_errors::Fixed;
use crate::users::now_ms;
use axum::Json;
use axum::extract::{Query, State};
use rs_admin_api::{
    ScriptErrorGroup, ScriptErrorLookup, ScriptErrorRecord, ScriptErrorsLookup, path,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The most errors the list shows, and rows of one.
const MAX_ERRORS: u32 = 300;
const MAX_ROWS: u32 = 200;
/// The longest script name or error message the console asks about.
const MAX_KEY_CHARS: usize = 2000;

const HOUR: i64 = 3_600_000;

#[derive(Deserialize)]
pub struct ListQuery {
    window: Option<String>,
}

#[derive(Serialize)]
pub struct ErrorView {
    #[serde(flatten)]
    error: ScriptErrorGroup,
    worlds: Vec<u16>,
    fixed: Option<Fixed>,
    came_back: bool,
}

#[derive(Serialize)]
pub struct ErrorList {
    since_ms: i64,
    errors: Vec<ErrorView>,
}

/// `GET /api/script-errors?window=24h|7d|30d`: the scripts the worlds
/// aborted, latest first. Maintainers only.
pub async fn list(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ListQuery>,
) -> Result<Json<ErrorList>, ApiError> {
    staff.require_maintainer()?;
    let span = match query.window.as_deref().unwrap_or("7d") {
        "24h" => 24 * HOUR,
        "7d" => 7 * 24 * HOUR,
        "30d" => 30 * 24 * HOUR,
        _ => return Err(ApiError::bad_request("Pick 24h, 7d or 30d.")),
    };
    let since_ms = now_ms() as i64 - span;
    let errors = views(&app, since_ms).await?;
    Ok(Json(ErrorList { since_ms, errors }))
}

/// The errors seen since `since_ms`, with their worlds and fixed marks.
async fn views(app: &App, since_ms: i64) -> Result<Vec<ErrorView>, ApiError> {
    let groups: Vec<ScriptErrorGroup> = app
        .worlds
        .ask(
            path::SCRIPT_ERRORS,
            &ScriptErrorsLookup {
                since_ms,
                limit: MAX_ERRORS,
            },
        )
        .await
        .map_err(unavailable)?;
    let fixed = app.fixed_errors.all();
    Ok(groups
        .into_iter()
        .map(|error| {
            let fixed = fixed
                .iter()
                .find(|f| f.is(&error.script, error.line, &error.message))
                .cloned();
            ErrorView {
                worlds: error
                    .nodes
                    .iter()
                    .filter_map(|&n| app.worlds.get(n).map(|w| w.id))
                    .collect(),
                came_back: fixed.as_ref().is_some_and(|f| error.last_ms > f.at_ms),
                fixed,
                error,
            }
        })
        .collect())
}

#[derive(Serialize)]
pub struct Summary {
    /// Errors seen in the last day that aren't marked fixed, or came back.
    open: usize,
}

/// `GET /api/script-errors/summary`: for the nav's count. Maintainers only.
pub async fn summary(State(app): State<Arc<App>>, staff: Staff) -> Result<Json<Summary>, ApiError> {
    staff.require_maintainer()?;
    let errors = views(&app, now_ms() as i64 - 24 * HOUR).await?;
    Ok(Json(Summary {
        open: errors
            .iter()
            .filter(|e| e.fixed.is_none() || e.came_back)
            .count(),
    }))
}

#[derive(Deserialize)]
pub struct ErrorKey {
    script: String,
    line: i32,
    message: String,
}

impl ErrorKey {
    fn checked(&self) -> Result<(), ApiError> {
        if self.script.chars().count() > MAX_KEY_CHARS
            || self.message.chars().count() > MAX_KEY_CHARS
        {
            return Err(ApiError::bad_request(
                "That isn't a script error the worlds know.",
            ));
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct RowView {
    #[serde(flatten)]
    row: ScriptErrorRecord,
    world: Option<u16>,
}

/// `GET /api/script-errors/rows?script=&line=&message=`: whom one error
/// happened to and where, latest first, with their replays.
pub async fn rows(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(key): Query<ErrorKey>,
) -> Result<Json<Vec<RowView>>, ApiError> {
    staff.require_maintainer()?;
    key.checked()?;
    let rows: Vec<ScriptErrorRecord> = app
        .worlds
        .ask(
            path::SCRIPT_ERROR,
            &ScriptErrorLookup {
                script: key.script,
                line: key.line,
                message: key.message,
                limit: MAX_ROWS,
            },
        )
        .await
        .map_err(unavailable)?;
    Ok(Json(
        rows.into_iter()
            .map(|row| RowView {
                world: app.worlds.get(row.node).map(|w| w.id),
                row,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct FixRequest {
    #[serde(flatten)]
    key: ErrorKey,
    fixed: bool,
}

/// `POST /api/script-errors/fixed`: marks an error fixed, setting it aside
/// until it happens again, or takes the mark off. Audited.
pub async fn set_fixed(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<FixRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    staff.require_maintainer()?;
    req.key.checked()?;
    let key = &req.key;
    app.fixed_errors
        .set(
            &key.script,
            key.line,
            &key.message,
            req.fixed.then(|| (staff.name(), now_ms() as i64)),
        )
        .map_err(ApiError::bad_request)?;
    app.audit.record(
        Event::new(
            Some(staff.name()),
            staff.client.ip,
            if req.fixed {
                "script_error_fixed"
            } else {
                "script_error_reopened"
            },
        )
        .target(format!("{}:{}", key.script, key.line))
        .detail(key.message.clone()),
    );
    Ok(Json(serde_json::json!({"ok": true})))
}
