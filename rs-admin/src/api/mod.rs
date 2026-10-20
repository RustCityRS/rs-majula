mod account;
mod auth;
mod edit;
mod lookup;
mod news;
mod players;
mod replays;
mod staff;
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
        .route("/players", get(players::search))
        .route("/players/kick", post(players::kick))
        .route("/players/moderate", post(players::moderate))
        .route("/lookup/player", get(lookup::player))
        .route("/lookup/moderation", get(lookup::moderation))
        .route("/lookup/sanctions", get(lookup::sanctions))
        .route("/lookup/chat", get(lookup::chat))
        .route("/replays", get(replays::list))
        .route("/replays/{id}", get(replays::get))
        .route("/replays/{id}/file", get(replays::file))
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
