use crate::audit::Event;
use crate::http::{ApiError, App, Client, Pending};
use crate::sessions::Stage;
use crate::totp;
use crate::users::{User, verify_password};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Shown in authenticator apps beside the account name.
const ISSUER: &str = "RS Admin";

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum StageName {
    SignedOut,
    Totp,
    Enroll,
    SignedIn,
}

fn stage_name(stage: Stage) -> StageName {
    match stage {
        Stage::Totp => StageName::Totp,
        Stage::Enroll => StageName::Enroll,
        Stage::SignedIn => StageName::SignedIn,
    }
}

#[derive(Serialize)]
pub struct SessionInfo {
    stage: StageName,
    name: Option<String>,
    role: Option<crate::users::Role>,
    csrf: Option<String>,
}

/// `GET /api/session`: where the visitor's sign-in stands.
pub async fn session(State(app): State<Arc<App>>, headers: HeaderMap) -> Json<SessionInfo> {
    let session = app.token(&headers).and_then(|t| app.sessions.get(&t));
    let user = session
        .as_ref()
        .and_then(|s| app.users.find(&s.user).ok().flatten())
        .filter(|u| !u.disabled);
    match (session, user) {
        (Some(session), Some(user)) => Json(SessionInfo {
            stage: stage_name(session.stage),
            name: Some(user.name),
            role: (session.stage == Stage::SignedIn).then_some(user.role),
            csrf: Some(session.csrf),
        }),
        _ => Json(SessionInfo {
            stage: StageName::SignedOut,
            name: None,
            role: None,
            csrf: None,
        }),
    }
}

#[derive(Deserialize)]
pub struct Login {
    name: String,
    password: String,
}

fn with_cookie(app: &App, token: &str, body: serde_json::Value) -> Response {
    ([(header::SET_COOKIE, app.set_cookie(token))], Json(body)).into_response()
}

/// `POST /api/auth/login`: checks the password, and on success opens a
/// session waiting on the second factor.
pub async fn login(
    State(app): State<Arc<App>>,
    client: Client,
    headers: HeaderMap,
    Json(req): Json<Login>,
) -> Result<Response, ApiError> {
    let name = req.name.trim().to_owned();
    if name.is_empty() || name.len() > 64 || req.password.len() > 256 {
        return Err(ApiError::bad_request("Enter your name and password."));
    }
    if !app.sessions.may_attempt(client.ip, &name) {
        app.audit.record(
            Event::new(None, client.ip, "sign_in_locked")
                .target(&name)
                .ok(false),
        );
        return Err(too_many_attempts());
    }

    let user = app.users.find(&name).map_err(ApiError::internal)?;
    let (stored_hash, user) = match user {
        Some(user) if !user.disabled => (user.password_hash.clone(), Some(user)),
        _ => (app.dummy_hash.clone(), None),
    };
    let password = req.password;
    // Argon2 is deliberately slow; keep it off the async workers.
    let verified = tokio::task::spawn_blocking(move || verify_password(&password, &stored_hash))
        .await
        .map_err(ApiError::internal)?;

    let Some(user) = user.filter(|_| verified) else {
        app.sessions.record_failure(client.ip, &name);
        app.audit.record(
            Event::new(None, client.ip, "sign_in_failed")
                .target(&name)
                .detail("wrong name or password")
                .ok(false),
        );
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "Wrong name or password.",
        ));
    };

    // A sign-in starts from scratch: any session this browser had ends.
    if let Some(old) = app.token(&headers) {
        app.sessions.end(&old);
    }
    let stage = if user.totp_secret().is_some() {
        Stage::Totp
    } else {
        Stage::Enroll
    };
    let token = app
        .sessions
        .create(&user.name, stage, client.ip, &client.user_agent);
    let csrf = app.sessions.get(&token).map(|s| s.csrf);
    Ok(with_cookie(
        &app,
        &token,
        json!({ "stage": stage_name(stage), "name": user.name, "csrf": csrf }),
    ))
}

fn too_many_attempts() -> ApiError {
    ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "Too many failed attempts. Wait 15 minutes and try again.",
    )
}

#[derive(Deserialize)]
pub struct Code {
    code: String,
}

/// The pending session's account, if it still exists and is enabled.
fn pending_user(app: &App, pending: &Pending) -> Result<User, ApiError> {
    app.users
        .find(&pending.session.user)
        .map_err(ApiError::internal)?
        .filter(|u| u.accepts_session(pending.session.created_ms))
        .ok_or_else(|| {
            app.sessions.end(&pending.token);
            ApiError::unauthorized()
        })
}

/// Checks a code against `secret`, counting a wrong one as a failed
/// sign-in. Too many and the pending sign-in ends.
fn check_code(app: &App, pending: &Pending, secret: &[u8], code: &str) -> Result<(), ApiError> {
    let user = &pending.session.user;
    if !app.sessions.may_attempt(pending.client.ip, user) {
        app.sessions.end(&pending.token);
        return Err(too_many_attempts());
    }
    let step = totp::matching_step(secret, code, totp::current_step());
    if step.is_some_and(|step| app.sessions.accept_totp_step(user, step)) {
        return Ok(());
    }
    app.sessions.record_failure(pending.client.ip, user);
    app.audit.record(
        Event::new(Some(user), pending.client.ip, "sign_in_failed")
            .detail("wrong authenticator code")
            .ok(false),
    );
    Err(ApiError::new(
        StatusCode::UNAUTHORIZED,
        "That code didn't work. Use the newest one in your app.",
    ))
}

/// Finishes a sign-in: a fresh token for the now fully trusted session.
fn sign_in(app: &App, pending: &Pending, user: &User) -> Result<Response, ApiError> {
    let token = app
        .sessions
        .advance(&pending.token, Stage::SignedIn)
        .ok_or_else(ApiError::unauthorized)?;
    let csrf = app.sessions.get(&token).map(|s| s.csrf);
    app.audit.record(
        Event::new(Some(&user.name), pending.client.ip, "sign_in")
            .detail(pending.client.user_agent.clone()),
    );
    Ok(with_cookie(
        app,
        &token,
        json!({ "stage": "signed_in", "name": user.name, "role": user.role, "csrf": csrf }),
    ))
}

/// `POST /api/auth/totp`: the second factor of an enrolled account.
pub async fn verify_totp(
    State(app): State<Arc<App>>,
    pending: Pending,
    Json(req): Json<Code>,
) -> Result<Response, ApiError> {
    if pending.session.stage != Stage::Totp {
        return Err(ApiError::bad_request("Set up your authenticator first."));
    }
    let user = pending_user(&app, &pending)?;
    let secret = user.totp_secret().ok_or_else(ApiError::unauthorized)?;
    check_code(&app, &pending, &secret, &req.code)?;
    sign_in(&app, &pending, &user)
}

/// The secret a pending enrolment uses, made on first ask.
fn enrol_secret(app: &App, pending: &Pending) -> Result<Vec<u8>, ApiError> {
    if pending.session.stage != Stage::Enroll {
        return Err(ApiError::bad_request(
            "Your authenticator is already set up.",
        ));
    }
    if let Some(secret) = &pending.session.pending_totp {
        return Ok(secret.clone());
    }
    let mut secret = vec![0u8; totp::SECRET_LEN];
    getrandom::fill(&mut secret).map_err(ApiError::internal)?;
    app.sessions
        .set_pending_totp(&pending.token, secret.clone());
    Ok(secret)
}

/// `GET /api/auth/enroll`: the secret to add to an authenticator app.
pub async fn enrolment(
    State(app): State<Arc<App>>,
    pending: Pending,
) -> Result<Json<serde_json::Value>, ApiError> {
    let secret = enrol_secret(&app, &pending)?;
    let key = totp::base32(&secret);
    // Grouped in fours for typing by hand.
    let grouped: Vec<String> = key
        .as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect();
    Ok(Json(json!({
        "key": grouped.join(" "),
        "uri": totp::provisioning_uri(ISSUER, &pending.session.user, &secret),
        "issuer": ISSUER,
    })))
}

/// `GET /api/auth/enroll/qr.svg`: the same secret as a QR code.
pub async fn enrolment_qr(
    State(app): State<Arc<App>>,
    pending: Pending,
) -> Result<Response, ApiError> {
    let secret = enrol_secret(&app, &pending)?;
    let uri = totp::provisioning_uri(ISSUER, &pending.session.user, &secret);
    let code = qrcode::QrCode::new(uri.as_bytes()).map_err(ApiError::internal)?;
    let svg = code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(220, 220)
        .quiet_zone(true)
        .dark_color(qrcode::render::svg::Color("#000000"))
        .light_color(qrcode::render::svg::Color("#ffffff"))
        .build();
    Ok(([(header::CONTENT_TYPE, "image/svg+xml")], svg).into_response())
}

/// `POST /api/auth/enroll`: confirms the new secret with a code from the app,
/// saves it, and signs in.
pub async fn confirm_enrolment(
    State(app): State<Arc<App>>,
    pending: Pending,
    Json(req): Json<Code>,
) -> Result<Response, ApiError> {
    if pending.session.stage != Stage::Enroll {
        return Err(ApiError::bad_request(
            "Your authenticator is already set up.",
        ));
    }
    let Some(secret) = pending.session.pending_totp.clone() else {
        return Err(ApiError::bad_request("Start the setup again."));
    };
    // Another sign-in finished enrolling first: this one mustn't replace
    // that authenticator.
    if pending_user(&app, &pending)?.totp_secret.is_some() {
        app.sessions.end(&pending.token);
        return Err(ApiError::unauthorized());
    }
    check_code(&app, &pending, &secret, &req.code)?;
    let user = app
        .users
        .update(&pending.session.user, |u| {
            u.totp_secret = Some(totp::base32(&secret))
        })
        .map_err(ApiError::internal)?;
    app.audit.record(Event::new(
        Some(&user.name),
        pending.client.ip,
        "2fa_enrolled",
    ));
    sign_in(&app, &pending, &user)
}

/// `POST /api/auth/logout`.
pub async fn logout(State(app): State<Arc<App>>, client: Client, headers: HeaderMap) -> Response {
    if let Some(token) = app.token(&headers)
        && let Some(session) = app.sessions.get(&token)
    {
        // Signing out is harmless, so it isn't CSRF checked; it only needs
        // the same-origin and JSON checks every unsafe request gets.
        app.sessions.end(&token);
        if session.stage == Stage::SignedIn {
            app.audit
                .record(Event::new(Some(&session.user), client.ip, "sign_out"));
        }
    }
    (
        [(header::SET_COOKIE, app.clear_cookie())],
        Json(json!({ "stage": "signed_out" })),
    )
        .into_response()
}
