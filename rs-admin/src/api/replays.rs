use super::access::{ceiling, check_player, check_replay};
use super::lookup::{TRIAGE_PAGES, unavailable};
use super::players::checked_name;
use crate::audit::Event;
use crate::cases::{ClipIn, Triage};
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

#[derive(Serialize)]
pub struct ReplayView {
    #[serde(flatten)]
    summary: ReplaySummary,
    world: Option<u16>,
    live: bool,
    #[serde(flatten)]
    triage: Triage,
}

pub(super) fn view(app: &App, summary: ReplaySummary) -> ReplayView {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64);
    let key = format!("replay:{}", summary.id);
    ReplayView {
        world: app.worlds.get(summary.node).map(|w| w.id),
        live: summary.ended_ms.is_none() && now - summary.updated_ms < LIVE_WITHIN_MS,
        triage: app
            .cases
            .triage_of(std::slice::from_ref(&key))
            .remove(&key)
            .unwrap_or_default(),
        summary,
    }
}

pub(super) fn checked_id(id: &str) -> Result<String, ApiError> {
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
    /// Only those in no case and not dismissed.
    #[serde(default)]
    untriaged: bool,
}

/// `GET /api/replays?name=&flagged=&before=&untriaged=`: recordings newest
/// first, with the cases each is in.
pub async fn list(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ReplayView>>, ApiError> {
    let name = query
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(checked_name)
        .transpose()?;
    if let Some(name) = &name {
        check_player(&app, &staff, name).await?;
    }
    let mut views = Vec::new();
    let mut before = query.before;
    for _ in 0..if query.untriaged { TRIAGE_PAGES } else { 1 } {
        let rows: Vec<ReplaySummary> = app
            .worlds
            .ask(
                path::REPLAYS,
                &ReplayLookup {
                    name: name.clone(),
                    flagged: query.flagged,
                    before,
                    max_staff_level: ceiling(&staff),
                    limit: PAGE,
                },
            )
            .await
            .map_err(unavailable)?;
        let full = rows.len() == PAGE as usize;
        before = rows.last().map(|r| r.started_ms);
        views.extend(rows.into_iter().map(|r| view(&app, r)).filter(|v| {
            !query.untriaged || (v.triage.cases.is_empty() && v.triage.dismissed.is_none())
        }));
        if !full || views.len() >= PAGE as usize {
            break;
        }
    }
    Ok(Json(views))
}

#[derive(Serialize)]
pub struct ReplayDetail {
    replay: ReplayView,
    /// Its clips in cases.
    clips: Vec<ClipIn>,
    /// The worlds pruned it: this is the copy the console kept for a clip.
    kept: bool,
    /// The world page to play it on: an online world at its revision with a
    /// public address in the console's list. `None` with `why` otherwise.
    player_url: Option<String>,
    why: Option<String>,
}

/// `GET /api/replays/{id}`: one recording, and where it can play.
pub async fn get(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<String>,
) -> Result<Json<ReplayDetail>, ApiError> {
    let id = checked_id(&id)?;
    let found: Result<Option<ReplaySummary>, String> = app
        .worlds
        .ask(path::REPLAY, &ReplayGet { id: id.clone() })
        .await;
    let (summary, kept) = match found {
        Ok(Some(summary)) => (summary, false),
        // Pruned, or the worlds can't say: the copy kept for a clip.
        other => match app.clips.summary(&id) {
            Some(summary) => (summary, true),
            None => {
                return Err(match other {
                    Err(e) => unavailable(e),
                    Ok(_) => ApiError::new(StatusCode::NOT_FOUND, "That recording is gone."),
                });
            }
        },
    };
    check_player(&app, &staff, &summary.player).await?;
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
        clips: app.cases.clips_of(&summary.id),
        kept,
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

#[derive(Deserialize)]
pub struct Viewed {
    player: String,
}

/// `POST /api/replays/{id}/viewed`: audits a recording watched from the
/// copy this browser kept, which isn't downloaded (and audited) again.
pub async fn viewed(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<String>,
    Json(req): Json<Viewed>,
) -> Result<StatusCode, ApiError> {
    let id = checked_id(&id)?;
    let player = checked_name(&req.player)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "replay_viewed")
            .target(player)
            .detail(format!("session {id}, from this browser's copy")),
    );
    Ok(StatusCode::NO_CONTENT)
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
    check_replay(&app, &staff, &id).await?;
    // The worlds' copy, else (pruned) the one kept for a clip.
    let (bytes, kept) = match app.worlds.replay_file(&id, &query.revision).await {
        Ok(bytes) => (bytes, false),
        Err(e) => (app.clips.file(&id).ok_or_else(|| unavailable(e))?, true),
    };
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "replay_viewed")
            .target(player)
            .detail(format!(
                "session {id}{}",
                if kept {
                    ", the copy kept for a clip"
                } else {
                    ""
                }
            )),
    );
    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response())
}
