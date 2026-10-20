mod account;
mod auth;
mod cases;
mod controls;
mod edit;
mod lookup;
mod news;
mod notes;
mod players;
mod replays;
mod schedule;
mod staff;
mod templates;
mod worlds;

use crate::audit::Entry;
use crate::http::{App, Staff};
use crate::users::name_key;
use axum::extract::{DefaultBodyLimit, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use rs_admin_api::MAX_MEDIA_BYTES;
use serde::Deserialize;
use std::sync::Arc;

pub fn router() -> Router<Arc<App>> {
    Router::new()
        .route("/session", get(auth::session))
        .route("/auth/login", post(auth::login))
        .route("/auth/totp", post(auth::verify_totp))
        .route(
            "/auth/enroll",
            get(auth::enrolment).post(auth::confirm_enrolment),
        )
        .route("/auth/enroll/qr.svg", get(auth::enrolment_qr))
        .route("/auth/logout", post(auth::logout))
        .route("/worlds", get(worlds::list))
        .route("/worlds/{node}/players", get(worlds::players))
        .route("/worlds/reboot", post(worlds::reboot))
        .route("/worlds/cancel-reboot", post(worlds::cancel_reboot))
        .route("/worlds/broadcast", post(worlds::broadcast))
        .route("/worlds/maintenance", post(worlds::maintenance))
        .route("/controls", get(controls::get))
        .route("/controls/switch", post(controls::switch))
        .route("/controls/lockdown", post(controls::lockdown))
        .route("/schedule", get(schedule::list).post(schedule::add))
        .route("/schedule/{id}/delete", post(schedule::remove))
        .route("/templates", get(templates::list).post(templates::save))
        .route("/templates/standing", get(templates::standing))
        .route("/notes", get(notes::of).post(notes::add))
        .route("/notes/watched", get(notes::watched))
        .route("/notes/{id}/remove", post(notes::remove))
        .route("/players", get(players::search))
        .route("/players/kick", post(players::kick))
        .route("/players/moderate", post(players::moderate))
        .route("/lookup/player", get(lookup::player))
        .route("/lookup/moderation", get(lookup::moderation))
        .route("/lookup/sanctions", get(lookup::sanctions))
        .route("/lookup/chat", get(lookup::chat))
        .route("/lookup/reports", get(lookup::reports))
        .route("/lookup/timeline", get(lookup::timeline))
        .route("/lookup/links", get(lookup::links))
        .route("/cases", get(cases::list).post(cases::create))
        .route("/cases/summary", get(cases::summary))
        .route("/cases/staff", get(cases::staff))
        .route("/cases/{id}", get(cases::get).post(cases::change))
        .route("/cases/preview", post(cases::preview))
        .route(
            "/cases/media",
            post(cases::upload).layer(DefaultBodyLimit::max(MAX_MEDIA_BYTES)),
        )
        .route("/cases/media/{name}", get(cases::media))
        .route("/triage", post(cases::triage))
        .route("/replays", get(replays::list))
        .route("/replays/{id}", get(replays::get))
        .route("/replays/{id}/file", get(replays::file))
        .route("/replays/{id}/viewed", post(replays::viewed))
        .route("/staff", get(staff::list).post(staff::add))
        .route("/staff/update", post(staff::update))
        .route("/players/sheet", get(edit::player))
        .route("/players/edit", post(edit::edit))
        .route("/objs", get(edit::objs))
        .route("/news", get(news::list))
        .route("/news/save", post(news::save))
        .route("/news/delete", post(news::delete))
        .route("/news/preview", post(news::preview))
        .route(
            "/news/media",
            post(news::upload).layer(DefaultBodyLimit::max(MAX_MEDIA_BYTES)),
        )
        .route("/news/media/{name}", get(news::media))
        .route("/audit", get(audit))
        .route("/account", get(account::get))
        .route("/account/sessions/{id}/end", post(account::end_session))
        .route(
            "/account/sessions/end-others",
            post(account::end_other_sessions),
        )
        .route("/account/password", post(account::change_password))
}

#[derive(Deserialize)]
struct AuditQuery {
    before: Option<u64>,
    limit: Option<usize>,
    actor: Option<String>,
    q: Option<String>,
}

/// `GET /api/audit`: the audit trail, newest first.
async fn audit(
    State(app): State<Arc<App>>,
    _staff: Staff,
    Query(query): Query<AuditQuery>,
) -> Json<Vec<Entry>> {
    let actor = query.actor.as_deref().map(name_key);
    let needle = query.q.as_deref().map(str::to_lowercase);
    Json(
        app.audit
            .page(query.before, query.limit.unwrap_or(50).min(200), |e| {
                actor
                    .as_ref()
                    .is_none_or(|a| e.actor.as_deref().map(name_key).as_ref() == Some(a))
                    && needle.as_ref().is_none_or(|n| {
                        e.action.contains(n.as_str())
                            || e.target
                                .as_deref()
                                .is_some_and(|t| t.to_lowercase().contains(n))
                            || e.detail.to_lowercase().contains(n)
                    })
            }),
    )
}
