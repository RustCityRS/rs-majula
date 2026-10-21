use super::access::CaseView;
use super::players::checked_name;
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::notes::{MAX_TEXT, Note};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct PlayerQuery {
    player: String,
}

/// `GET /api/notes?player=`: the notes on a player, newest first.
pub async fn of(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<PlayerQuery>,
) -> Result<Json<Vec<Note>>, ApiError> {
    let player = checked_name(&query.player)?;
    let view = CaseView::new(&app, &staff);
    Ok(Json(
        app.notes
            .of(&player)
            .into_iter()
            .filter(|n| sees(&view, &staff, n))
            .collect(),
    ))
}

/// Notes on staff are for those above them, as their cases are, but
/// whoever wrote one sees it.
fn sees(view: &CaseView, staff: &Staff, note: &Note) -> bool {
    view.sees_subject(&note.player)
        || crate::users::name_key(&note.by) == crate::users::name_key(staff.name())
}

#[derive(Deserialize)]
pub struct NewNote {
    player: String,
    text: String,
    #[serde(default)]
    watch: bool,
}

/// `POST /api/notes`: adds a note. Any staff can.
pub async fn add(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<NewNote>,
) -> Result<Json<Note>, ApiError> {
    let player = checked_name(&req.player)?;
    let text: String = req
        .text
        .trim()
        .chars()
        .filter(|&c| c == '\n' || !c.is_control())
        .collect();
    if text.is_empty() || text.chars().count() > MAX_TEXT {
        return Err(ApiError::bad_request(format!(
            "A note is 1 to {MAX_TEXT} characters."
        )));
    }
    let note = app
        .notes
        .add(staff.name(), &player, &text, req.watch)
        .map_err(ApiError::bad_request)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "note_added")
            .target(&note.player)
            .detail(format!(
                "{}{}",
                if note.watch { "(watch) " } else { "" },
                clip(&note.text)
            )),
    );
    Ok(Json(note))
}

/// `POST /api/notes/{id}/remove`: its author, or an admin, can.
pub async fn remove(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    let note = app
        .notes
        .remove(id, staff.name(), staff.require_admin().is_ok())
        .map_err(ApiError::bad_request)?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "note_removed")
            .target(&note.player)
            .detail(format!("by {}: {}", note.by, clip(&note.text))),
    );
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct Watched {
    #[serde(flatten)]
    note: Note,
    /// The world they're on, when online.
    world: Option<u16>,
}

/// `GET /api/notes/watched`: the watchlist, with who's online now.
pub async fn watched(State(app): State<Arc<App>>, staff: Staff) -> Json<Vec<Watched>> {
    let view = CaseView::new(&app, &staff);
    let notes: Vec<Note> = app
        .notes
        .watched()
        .into_iter()
        .filter(|n| sees(&view, &staff, n))
        .collect();
    let worlds = app.worlds.online();
    let lists = join_all(worlds.iter().map(|w| app.worlds.players(w))).await;
    let online: std::collections::HashMap<String, u16> = worlds
        .iter()
        .zip(lists)
        .filter_map(|(w, list)| list.ok().map(|l| (w.id, l)))
        .flat_map(|(world, list)| {
            list.into_iter()
                .map(move |p| (crate::users::name_key(&p.name), world))
        })
        .collect();
    Json(
        notes
            .into_iter()
            .map(|note| Watched {
                world: online.get(&crate::users::name_key(&note.player)).copied(),
                note,
            })
            .collect(),
    )
}

/// A note as the audit log shows it: its first 120 characters.
fn clip(text: &str) -> String {
    let line = text.replace('\n', " ");
    if line.chars().count() > 120 {
        format!("{}...", line.chars().take(117).collect::<String>())
    } else {
        line
    }
}
