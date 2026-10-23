pub(crate) mod access;
mod account;
mod auth;
mod cases;
mod controls;
mod economy;
mod edit;
mod export;
mod hiscores;
mod live;
mod lookup;
mod news;
mod notes;
mod players;
mod prefs;
mod replays;
mod schedule;
mod scripts;
mod search;
mod staff;
mod support;
mod templates;
mod worlds;

use crate::audit::Entry;
use crate::cache::Cache;
use crate::http::{ApiError, App, Staff};
use crate::users::name_key;
use access::AuditView;
use axum::extract::{DefaultBodyLimit, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use rs_admin_api::{MAX_MEDIA_BYTES, ReplaySummary, ReportRecord, ScriptErrorGroup};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

/// What an untriaged sessions walk asked for: whose, only flagged ones,
/// from when back, and of staff up to which level.
type ReplaysKey = (Option<String>, bool, Option<i64>, Option<u8>);

/// Answers from the worlds that requests share for a while, rather than
/// each asking again.
pub struct Caches {
    /// The untriaged queues' walks through the worlds' pages, by what they
    /// asked for. What's in them is checked against the cases afresh each
    /// time, so only new reports and sessions wait to show.
    reports: Cache<(Option<String>, Option<i64>), Vec<ReportRecord>>,
    replays: Cache<ReplaysKey, Vec<ReplaySummary>>,
    /// Who's online on each world, for every live map and search open.
    online: Cache<(), Arc<Vec<live::Listed>>>,
    /// The last day's script errors, for the nav's count. Fixed marks are
    /// applied afresh.
    script_errors: Cache<(), Vec<ScriptErrorGroup>>,
    /// Bot farm groups, by how far back they look.
    farms: Cache<i64, Arc<live::Grouped>>,
}

impl Default for Caches {
    fn default() -> Self {
        Self {
            reports: Cache::new(Duration::from_secs(30)),
            replays: Cache::new(Duration::from_secs(30)),
            online: Cache::new(Duration::from_secs(2)),
            script_errors: Cache::new(Duration::from_secs(60)),
            farms: Cache::new(Duration::from_secs(60)),
        }
    }
}

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
        .route("/worlds/shutdown", post(worlds::shutdown))
        .route("/worlds/start", post(worlds::start))
        .route("/worlds/cancel-reboot", post(worlds::cancel_reboot))
        .route("/worlds/broadcast", post(worlds::broadcast))
        .route("/worlds/maintenance", post(worlds::maintenance))
        .route(
            "/worlds/{node}/reload-scripts",
            post(worlds::reload_scripts),
        )
        .route("/economy", get(economy::dashboard))
        .route("/economy/summary", get(economy::summary))
        .route("/economy/sample", post(economy::sample))
        .route("/economy/track", post(economy::track))
        .route("/economy/untrack", post(economy::untrack))
        .route(
            "/economy/alerts/{id}/acknowledge",
            post(economy::acknowledge),
        )
        .route("/economy/holders", get(economy::holders))
        .route("/economy/trace", get(economy::trace))
        .route("/economy/trades", get(economy::trades))
        .route("/economy/receivers", get(economy::receivers))
        .route("/economy/ring", get(economy::ring_preview))
        .route("/economy/ring-rollback", post(economy::ring_rollback))
        .route("/hiscores/player", get(hiscores::player))
        .route("/hiscores/change", post(hiscores::change))
        .route("/hiscores/kept-off", get(hiscores::kept_off))
        .route("/controls", get(controls::get))
        .route("/controls/switch", post(controls::switch))
        .route("/controls/check", post(controls::check))
        .route("/controls/lockdown", post(controls::lockdown))
        .route("/schedule", get(schedule::list).post(schedule::add))
        .route("/schedule/{id}/delete", post(schedule::remove))
        .route("/templates", get(templates::list).post(templates::save))
        .route("/templates/standing", get(templates::standing))
        .route(
            "/canned",
            get(templates::canned).post(templates::save_canned),
        )
        .route("/notes", get(notes::of).post(notes::add))
        .route("/notes/watched", get(notes::watched))
        .route("/notes/{id}/remove", post(notes::remove))
        .route("/players", get(players::search))
        .route("/players/names", get(search::names))
        .route("/players/kick", post(players::kick))
        .route("/players/moderate", post(players::moderate))
        .route("/lookup/player", get(lookup::player))
        .route("/lookup/moderation", get(lookup::moderation))
        .route("/lookup/sanctions", get(lookup::sanctions))
        .route("/lookup/chat", get(lookup::chat))
        .route("/lookup/reports", get(lookup::reports))
        .route("/lookup/timeline", get(lookup::timeline))
        .route("/lookup/links", get(lookup::links))
        .route("/lookup/link-chain", get(lookup::link_chain))
        .route("/lookup/evasion", get(lookup::evasion))
        .route("/lookup/progress", get(lookup::progress))
        .route("/lookup/activity", get(lookup::activity))
        .route("/cases", get(cases::list).post(cases::create))
        .route("/cases/summary", get(cases::summary))
        .route("/cases/read", post(cases::mark_read))
        .route("/cases/staff", get(cases::staff))
        .route("/cases/{id}", get(cases::get).post(cases::change))
        .route("/cases/preview", post(cases::preview))
        .route(
            "/cases/media",
            post(cases::upload).layer(DefaultBodyLimit::max(MAX_MEDIA_BYTES)),
        )
        .route("/cases/media/{name}", get(cases::media))
        .route("/triage", post(cases::triage))
        .route("/triage/bulk", post(cases::triage_bulk))
        .route("/replays", get(replays::list))
        .route("/replays/storage", get(replays::storage))
        .route("/replays/{id}", get(replays::get))
        .route("/replays/{id}/file", get(replays::file))
        .route("/replays/{id}/viewed", post(replays::viewed))
        .route("/replays/{id}/live", get(live::replay))
        .route("/live/players", get(live::players))
        .route("/live/anticheat", get(live::anticheat))
        .route("/live/farms", get(live::farms))
        .route("/staff", get(staff::list).post(staff::add))
        .route("/staff/update", post(staff::update))
        .route("/staff/activity", get(staff::activity))
        .route("/staff/member", get(staff::member))
        .route("/players/sheet", get(edit::player))
        .route("/players/edit", post(edit::edit))
        .route("/objs", get(edit::objs))
        .route("/support/session", get(support::session))
        .route("/support/unstick", post(support::unstick))
        .route("/support/send-home", post(support::send_home))
        .route("/support/password-reset", post(support::password_reset))
        .route("/support/sheet", get(support::sheet))
        .route("/support/history", get(support::history))
        .route("/support/history/sheet", get(support::point_sheet))
        .route("/support/rollback", post(support::rollback))
        .route("/support/restore", post(support::restore))
        .route("/news", get(news::list))
        .route("/news/save", post(news::save))
        .route("/news/delete", post(news::delete))
        .route("/news/preview", post(news::preview))
        .route(
            "/news/media",
            post(news::upload).layer(DefaultBodyLimit::max(MAX_MEDIA_BYTES)),
        )
        .route("/news/media/{name}", get(news::media))
        .route("/script-errors", get(scripts::list))
        .route("/script-errors/rows", get(scripts::rows))
        .route("/script-errors/summary", get(scripts::summary))
        .route("/script-errors/fixed", post(scripts::set_fixed))
        .route("/audit", get(audit))
        .route("/search", get(search::search))
        .route("/export/audit", get(export::audit))
        .route("/export/chat", get(export::chat))
        .route("/export/timeline", get(export::timeline))
        .route("/prefs", get(prefs::get).post(prefs::change))
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

/// Which audit entries a search keeps: whose (see [`Entry::owner`], so
/// failed sign-ins as someone count as theirs), and with what in them.
pub(super) struct AuditFilter {
    actor: Option<String>,
    needle: Option<String>,
}

impl AuditFilter {
    pub(super) fn new(actor: Option<&str>, q: Option<&str>) -> Self {
        Self {
            actor: actor.map(name_key).filter(|a| !a.is_empty()),
            needle: q.map(|q| q.trim().to_lowercase()).filter(|q| !q.is_empty()),
        }
    }

    pub(super) fn keeps(&self, e: &Entry) -> bool {
        self.actor
            .as_ref()
            .is_none_or(|a| e.owner().map(name_key).as_ref() == Some(a))
            && self.needle.as_ref().is_none_or(|n| {
                e.action.contains(n.as_str())
                    || e.target
                        .as_deref()
                        .is_some_and(|t| t.to_lowercase().contains(n))
                    || e.detail.to_lowercase().contains(n)
            })
    }
}

/// `GET /api/audit`: the audit trail, newest first.
async fn audit(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<Entry>>, ApiError> {
    let filter = AuditFilter::new(query.actor.as_deref(), query.q.as_deref());
    let view = AuditView::new(&app, &staff);
    let limit = query.limit.unwrap_or(50).min(200);
    // Older pages and searches read the file.
    tokio::task::spawn_blocking(move || {
        app.audit
            .page(query.before, limit, |e| view.shows(e) && filter.keeps(e))
            .into_iter()
            .map(|e| view.redact(e))
            .collect()
    })
    .await
    .map(Json)
    .map_err(ApiError::internal)
}
