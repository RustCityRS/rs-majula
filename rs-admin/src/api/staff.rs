//! Console accounts, managed from the console by developers: the same
//! changes as `rs-admin user ...`, each confirmed with the developer's own
//! password and authenticator code, and audited.

use super::account::reauthenticate;
use super::players::checked_name;
use crate::audit::Event;
use crate::http::{ApiError, App, Staff};
use crate::users::{Role, User, hash_password, name_key, now_ms, password_problem};
use crate::worlds::Worlds;
use axum::Json;
use axum::extract::State;
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

/// `GET /api/staff`.
pub async fn list(
    State(app): State<Arc<App>>,
    staff: Staff,
) -> Result<Json<Vec<StaffAccount>>, ApiError> {
    staff.require_developer()?;
    let mut accounts: Vec<StaffAccount> = app
        .users
        .all()
        .map_err(ApiError::internal)?
        .into_iter()
        .map(|u| StaffAccount {
            sessions: app
                .sessions
                .of(&u.name)
                .iter()
                .filter(|s| u.accepts_session(s.created_ms))
                .count(),
            you: name_key(&u.name) == name_key(staff.name()),
            name: u.name,
            role: u.role,
            two_factor: u.totp_secret.is_some(),
            disabled: u.disabled,
            created_ms: u.created_ms,
        })
        .collect();
    accounts.sort_by_key(|a| (std::cmp::Reverse(a.role), name_key(&a.name)));
    Ok(Json(accounts))
}

/// The acting developer's password and a fresh authenticator code.
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
        Role::Moderator => "moderator",
        Role::Admin => "admin",
        Role::Developer => "developer",
    }
}

/// `POST /api/staff`: adds an account. They set up their authenticator at
/// their first sign-in.
pub async fn add(
    State(app): State<Arc<App>>,
    staff: Staff,
    Json(req): Json<NewAccount>,
) -> Result<Json<serde_json::Value>, ApiError> {
    staff.require_developer()?;
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
    staff.require_developer()?;
    let target = app
        .users
        .find(&req.name)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::bad_request("There's no account by that name."))?;
    // Your own account changes on the Account page; this also keeps at least
    // one developer (you) in charge.
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
                        Change::Password { .. } | Change::Remove => {}
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
