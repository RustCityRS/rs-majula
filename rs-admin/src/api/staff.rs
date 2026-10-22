use super::access::AuditView;
use super::account::{SessionView, reauthenticate};
use super::players::checked_name;
use crate::activity::{self, StaffActivity};
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::sessions::Session;
use crate::users::{Role, User, hash_password, name_key, now_ms, password_problem};
use crate::worlds::Worlds;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use rs_admin_api::{PlayerLookup, PlayerRecord, path};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Serialize)]
pub struct StaffAccount {
    name: String,
    role: Role,
    two_factor: bool,
    disabled: bool,
    created_ms: u64,
    sessions: usize,
    you: bool,
}

impl StaffAccount {
    fn of(app: &App, staff: &Staff, u: User) -> Self {
        Self {
            sessions: open_sessions(app, &u).len(),
            you: name_key(&u.name) == name_key(staff.name()),
            name: u.name,
            role: u.role,
            two_factor: u.totp_secret.is_some(),
            disabled: u.disabled,
            created_ms: u.created_ms,
        }
    }
}

/// `u`'s signed-in sessions, less those signed out from the command line
/// since (they end at their next request).
fn open_sessions(app: &App, u: &User) -> Vec<Session> {
    let mut sessions = app.sessions.of(&u.name);
    sessions.retain(|s| u.accepts_session(s.created_ms));
    sessions
}

/// `GET /api/staff`.
pub async fn list(
    State(app): State<Arc<App>>,
    staff: Staff,
) -> Result<Json<Vec<StaffAccount>>, ApiError> {
    staff.require_maintainer()?;
    let mut accounts: Vec<StaffAccount> = app
        .users
        .all()
        .map_err(ApiError::internal)?
        .into_iter()
        .map(|u| StaffAccount::of(&app, &staff, u))
        .collect();
    accounts.sort_by_key(|a| (std::cmp::Reverse(a.role), name_key(&a.name)));
    Ok(Json(accounts))
}

#[derive(Deserialize)]
pub struct MemberQuery {
    name: String,
    days: Option<u64>,
}

#[derive(Serialize)]
pub struct Member {
    #[serde(flatten)]
    account: StaffAccount,
    open_sessions: Vec<SessionView>,
    /// What they did over the last `days`, if anything.
    activity: Option<StaffActivity>,
}

/// `GET /api/staff/member?name=&days=`: one staff account, where it's
/// signed in and what they did over the last `days` (30 by default).
pub async fn member(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<MemberQuery>,
) -> Result<Json<Member>, ApiError> {
    staff.require_maintainer()?;
    let user = app
        .users
        .find(&query.name)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "There's no account by that name."))?;
    let key = name_key(&user.name);
    let activity = activity_rows(&app, query.days, |u| name_key(&u.name) == key)
        .await?
        .into_iter()
        .find(|a| name_key(&a.name) == key);
    let open_sessions = open_sessions(&app, &user)
        .into_iter()
        .map(|s| SessionView::of(s, staff.session.id))
        .collect();
    Ok(Json(Member {
        account: StaffAccount::of(&app, &staff, user),
        open_sessions,
        activity,
    }))
}

/// The acting maintainer's password and a fresh authenticator code.
#[derive(Deserialize)]
pub struct Confirm {
    password: String,
    code: String,
}

#[derive(Deserialize)]
pub struct NewAccount {
    name: String,
    role: Role,
    password: String,
    confirm: Confirm,
}

async fn hashed(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::internal)
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Trial => "trial moderator",
        Role::Moderator => "moderator",
        Role::Admin => "admin",
        Role::Maintainer => "maintainer",
    }
}

/// `POST /api/staff`: adds an account. They set up their authenticator at
/// their first sign-in.
pub async fn add(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<NewAccount>,
) -> Result<Json<serde_json::Value>, ApiError> {
    staff.require_maintainer()?;
    let name = checked_name(&req.name)?;
    let lookup: Option<PlayerRecord> = app
        .worlds
        .ask(path::PLAYER, &PlayerLookup { name: name.clone() })
        .await
        .map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, format!("{e}.")))?;
    // Their game name, as the game spells it.
    let name = Worlds::game_staff(lookup, &name)
        .map_err(|e| ApiError::bad_request(format!("{e}.")))?
        .name;
    if let Some(problem) = password_problem(&req.password, &name) {
        return Err(ApiError::bad_request(problem));
    }
    if app.users.find(&name).map_err(ApiError::internal)?.is_some() {
        return Err(ApiError::bad_request(format!(
            "An account named {name} already exists."
        )));
    }
    reauthenticate(
        &app,
        &staff,
        req.confirm.password,
        &req.confirm.code,
        "staff_added",
    )
    .await?;
    let password_hash = hashed(req.password).await?;
    app.users
        .add(User {
            name: name.clone(),
            role: req.role,
            password_hash,
            totp_secret: None,
            disabled: false,
            created_ms: now_ms(),
            signed_out_ms: 0,
        })
        .map_err(|e| ApiError::bad_request(format!("{e}.")))?;
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, "staff_added")
            .target(&name)
            .detail(role_name(req.role)),
    );
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    Role { role: Role },
    Password { password: String },
    Reset2fa,
    Disable,
    Enable,
    Remove,
    SignOut,
}

#[derive(Deserialize)]
pub struct Update {
    name: String,
    change: Change,
    confirm: Confirm,
}

/// `POST /api/staff/update`: changes someone else's account. Every change
/// signs them out everywhere.
pub async fn update(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<Update>,
) -> Result<Json<serde_json::Value>, ApiError> {
    staff.require_maintainer()?;
    let target = app
        .users
        .find(&req.name)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::bad_request("There's no account by that name."))?;
    // Your own account changes on the Account page; this also keeps at least
    // one maintainer (you) in charge.
    if name_key(&target.name) == name_key(staff.name()) {
        return Err(ApiError::bad_request(
            "You can't change your own account here. Use the Account page.",
        ));
    }
    let name = target.name.clone();
    let (action, detail) = match &req.change {
        Change::Role { role } => ("staff_role", role_name(*role).to_owned()),
        Change::Password { password } => {
            if let Some(problem) = password_problem(password, &name) {
                return Err(ApiError::bad_request(problem));
            }
            ("staff_password", String::new())
        }
        Change::Reset2fa => ("staff_2fa_reset", String::new()),
        Change::Disable => ("staff_disabled", String::new()),
        Change::Enable => ("staff_enabled", String::new()),
        Change::Remove => ("staff_removed", String::new()),
        Change::SignOut => ("staff_signed_out", String::new()),
    };
    reauthenticate(
        &app,
        &staff,
        req.confirm.password,
        &req.confirm.code,
        action,
    )
    .await?;

    let signed_out = now_ms();
    match req.change {
        Change::Remove => app.users.remove(&name).map_err(ApiError::internal)?,
        change => {
            let password_hash = match &change {
                Change::Password { password } => Some(hashed(password.clone()).await?),
                _ => None,
            };
            app.users
                .update(&name, |u| {
                    match change {
                        Change::Role { role } => u.role = role,
                        Change::Reset2fa => u.totp_secret = None,
                        Change::Disable => u.disabled = true,
                        Change::Enable => u.disabled = false,
                        Change::Password { .. } | Change::Remove | Change::SignOut => {}
                    }
                    if let Some(hash) = password_hash {
                        u.password_hash = hash;
                    }
                    u.signed_out_ms = signed_out;
                })
                .map_err(ApiError::internal)?;
        }
    }
    app.sessions.end_all(&name, None);
    app.audit.record(
        Event::new(Some(staff.name()), staff.client.ip, action)
            .target(&name)
            .detail(detail),
    );
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ActivityQuery {
    days: Option<u64>,
}

/// `GET /api/staff/activity?days=`: what each staff member did over the
/// last `days` (30 by default), for coaching. Admins and up.
pub async fn activity(
    State(app): State<Arc<App>>,
    staff: Staff,
    Query(query): Query<ActivityQuery>,
) -> Result<Json<Vec<StaffActivity>>, ApiError> {
    staff.require_admin()?;
    let mut rows = activity_rows(&app, query.days, |u| !u.disabled).await?;
    let view = AuditView::new(&app, &staff);
    rows.retain(|row| view.sees(&row.name));
    Ok(Json(rows))
}

/// What staff did over the last `days` (30 by default): a row for each
/// account `listed` keeps, and for anyone else who did something.
async fn activity_rows(
    app: &Arc<App>,
    days: Option<u64>,
    listed: impl Fn(&User) -> bool,
) -> Result<Vec<StaffActivity>, ApiError> {
    let days = days.unwrap_or(30).clamp(1, 365);
    let since_ms = now_ms().saturating_sub(days * 86_400_000);
    let accounts: Vec<(String, Role)> = app
        .users
        .all()
        .map_err(ApiError::internal)?
        .into_iter()
        .filter(|u| listed(u))
        .map(|u| (u.name, u.role))
        .collect();
    let app = app.clone();
    // It reads back through the audit file.
    tokio::task::spawn_blocking(move || {
        let entries = app.audit.since(since_ms, |e| {
            matches!(
                e.action.as_str(),
                "kick" | "mute" | "ban" | "unmute" | "unban"
            )
        });
        app.cases
            .read(|cases, dismissed| activity::of(&accounts, cases, dismissed, &entries, since_ms))
    })
    .await
    .map_err(ApiError::internal)
}
