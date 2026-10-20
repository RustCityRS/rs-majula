//! Session recordings: the list, the anti-cheat checks' verdicts, and the
//! replay file the viewer hands to a world's replay page. Any world answers
//! the lookups; only one at the recording's protocol revision can build its
//! replay. Watching one is recorded in the audit log.

use super::players::checked_name;
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rs_admin_api::{ReplayGet, ReplayLookup, ReplaySummary, path};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Rows per page.
const PAGE: u32 = 50;
/// A recording that hasn't ended is still going if its world stored some of
/// it this recently (worlds store every 30 seconds).
const LIVE_WITHIN_MS: i64 = 90_000;

fn unavailable(error: String) -> ApiError {
    ApiError::new(StatusCode::BAD_GATEWAY, format!("{error}."))
}

#[derive(Serialize)]
pub struct ReplayView {
    #[serde(flatten)]
    summary: ReplaySummary,
    world: Option<u16>,
    live: bool,
}

fn view(app: &App, summary: ReplaySummary) -> ReplayView {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64);
    ReplayView {
        world: app.worlds.get(summary.node).map(|w| w.id),
        live: summary.ended_ms.is_none() && now - summary.updated_ms < LIVE_WITHIN_MS,
        summary,
    }
}

fn checked_id(id: &str) -> Result<String, ApiError> {
    if !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit()) {
        Ok(id.to_owned())
    } else {
        Err(ApiError::bad_request("No such recording."))
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    name: Option<String>,
    #[serde(default)]
    flagged: bool,
    before: Option<i64>,
}

/// `GET /api/replays?name=&flagged=&before=`: recordings newest first.
pub async fn list(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ReplayView>>, ApiError> {
    let name = query
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(checked_name)
        .transpose()?;
    let rows: Vec<ReplaySummary> = app
        .worlds
        .ask(
            path::REPLAYS,
            &ReplayLookup {
                name,
                flagged: query.flagged,
                before: query.before,
                limit: PAGE,
            },
        )
        .await
        .map_err(unavailable)?;
    Ok(Json(rows.into_iter().map(|r| view(&app, r)).collect()))
}

#[derive(Serialize)]
pub struct ReplayDetail {
    replay: ReplayView,
    /// The world page to play it on: an online world at its revision with a
    /// public address in the console's list. `None` with `why` otherwise.
    player_url: Option<String>,
    why: Option<String>,
}

/// `GET /api/replays/{id}`: one recording, and where it can play.
pub async fn get(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Path(id): Path<String>,
) -> Result<Json<ReplayDetail>, ApiError> {
    let id = checked_id(&id)?;
    let summary: Option<ReplaySummary> = app
        .worlds
        .ask(path::REPLAY, &ReplayGet { id })
        .await
        .map_err(unavailable)?;
    let summary =
        summary.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "That recording is gone."))?;
    let at_revision = app.worlds.online_at(&summary.revision);
    let player_url = at_revision
        .iter()
        .find_map(|w| w.client.as_deref())
        .map(|base| format!("{}/replay", base.trim_end_matches('/')));
    let why = player_url.is_none().then(|| {
        if at_revision.is_empty() {
            format!(
                "It was recorded on revision {}, and no world on that revision is online.",
                summary.revision
            )
        } else {
            "No online world on its revision has a public address (client) in the console's worlds file.".to_owned()
        }
    });
    Ok(Json(ReplayDetail {
        replay: view(&app, summary),
        player_url,
        why,
    }))
}

#[derive(Deserialize)]
pub struct FileQuery {
    revision: String,
    player: String,
}

/// `GET /api/replays/{id}/file?revision=&player=`: the replay file, for the
/// viewer to hand to the replay page. Audited as watching it.
pub async fn file(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<String>,
    Query(query): Query<FileQuery>,
) -> Result<Response, ApiError> {
    let id = checked_id(&id)?;
    let player = checked_name(&query.player)?;
    let bytes = app
        .worlds
        .replay_file(&id, &query.revision)
        .await
        .map_err(unavailable)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "replay_viewed")
            .target(player)
            .detail(format!("session {id}")),
    );
    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response())
}
