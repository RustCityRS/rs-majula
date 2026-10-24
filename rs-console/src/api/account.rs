use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::sessions::Session;
use crate::totp;
use crate::users::{Role, hash_password, password_problem, verify_password};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Serialize)]
pub struct SessionView {
    id: u64,
    ip: String,
    user_agent: String,
    created_ms: u64,
    last_seen_ms: u64,
    current: bool,
}

impl SessionView {
    /// `s`, as seen from the session `current`.
    pub(super) fn of(s: Session, current: u64) -> Self {
        Self {
            id: s.id,
            ip: s.ip.to_string(),
            current: s.id == current,
            created_ms: s.created_ms,
            last_seen_ms: s.last_seen_ms(),
            user_agent: s.user_agent,
        }
    }
}

#[derive(Serialize)]
pub struct Account {
    name: String,
    role: Role,
    sessions: Vec<SessionView>,
}

/// `GET /api/account`.
pub async fn get(State(app): State<Arc<App>>, staff: Staff) -> Json<Account> {
    let sessions = app
        .sessions
        .of(staff.name())
        .into_iter()
        // Signed out from the CLI since: they end at their next request.
        .filter(|s| staff.user.accepts_session(s.created_ms))
        .map(|s| SessionView::of(s, staff.session.id))
        .collect();
    Json(Account {
        name: staff.user.name.clone(),
        role: staff.user.role,
        sessions,
    })
}

/// `POST /api/account/sessions/{id}/end`.
pub async fn end_session(
    State(app): State<Arc<App>>,
    staff: Staff,
    Path(id): Path<u64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !app.sessions.end_by_id(staff.name(), id) {
        return Err(ApiError::bad_request("That session has already ended."));
    }
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "session_ended")
            .detail(format!("session {id}")),
    );
    Ok(Json(json!({ "ok": true })))
}

/// `POST /api/account/sessions/end-others`.
pub async fn end_other_sessions(
    State(app): State<Arc<App>>,
    staff: Staff,
) -> Json<serde_json::Value> {
    let ended = app.sessions.end_all(staff.name(), Some(staff.session.id));
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "session_ended")
            .detail(format!("{ended} other session(s)")),
    );
    Json(json!({ "ended": ended }))
}

/// Checks that it's really `staff` making a sensitive change: their password
/// and a fresh authenticator code, rate limited like a sign-in. A failure is
/// audited under `action`.
pub async fn reauthenticate(
    app: &App,
    staff: &Staff,
    password: String,
    code: &str,
    action: &'static str,
) -> Result<(), ApiError> {
    let name = staff.name();
    if !app.sessions.may_attempt(staff.client.ip, name) {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many failed attempts. Wait 15 minutes and try again.",
        ));
    }
    let stored = staff.user.password_hash.clone();
    let password_ok = tokio::task::spawn_blocking(move || verify_password(&password, &stored))
        .await
        .map_err(ApiError::internal)?;
    let code_ok = staff.user.totp_secret().is_some_and(|secret| {
        totp::matching_step(&secret, code, totp::current_step())
            .is_some_and(|step| app.sessions.accept_totp_step(name, step))
    });
    if password_ok && code_ok {
        return Ok(());
    }
    app.sessions.record_failure(staff.client.ip, name);
    app.audit.record(
        Event::new(Some(name), staff.client.ip, action)
            .detail("wrong password or code")
            .ok(false),
    );
    Err(ApiError::new(
        StatusCode::FORBIDDEN,
        "Your password or authenticator code is wrong.",
    ))
}

#[derive(Deserialize)]
pub struct PasswordChange {
    current: String,
    new: String,
    code: String,
}

/// `POST /api/account/password`: needs the current password and a fresh
/// authenticator code; signs out every other session.
pub async fn change_password(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<PasswordChange>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let name = staff.name().to_owned();
    if let Some(problem) = password_problem(&req.new, &name) {
        return Err(ApiError::bad_request(problem));
    }
    reauthenticate(&app, &staff, req.current, &req.code, "password_changed").await?;

    let new = req.new;
    let hash = tokio::task::spawn_blocking(move || hash_password(&new))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::internal)?;
    app.users
        .update(&name, |u| u.password_hash = hash)
        .map_err(ApiError::internal)?;
    let ended = app.sessions.end_all(&name, Some(staff.session.id));
    app.audit.record(
        Event::new(Some(&name), staff.client.ip, "password_changed")
            .detail(format!("signed out {ended} other session(s)")),
    );
    Ok(Json(json!({ "ok": true })))
}
