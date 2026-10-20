use crate::api;
use crate::audit::Audit;
use crate::canned::CannedTexts;
use crate::cases::Cases;
use crate::clips::Clips;
use crate::controls::Controls;
use crate::notes::Notes;
use crate::prefs::Prefs;
use crate::schedule::Schedule;
use crate::sessions::{Session, Sessions, Stage};
use crate::templates::Templates;
use crate::users::{Role, User, Users};
use crate::worlds::Worlds;
use axum::extract::{ConnectInfo, DefaultBodyLimit, FromRequestParts, Request, State};
use axum::http::header::{self, HeaderName, HeaderValue};
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

/// API request bodies are a few small JSON fields.
const MAX_BODY: usize = 16 * 1024;
/// Where images and videos are uploaded, as raw bytes: for news, and for
/// cases.
pub const UPLOAD_PATHS: [&str; 2] = ["/api/news/media", "/api/cases/media"];

pub struct HttpConfig {
    pub https: bool,
    pub origin: Option<String>,
    pub real_ip_header: Option<HeaderName>,
    pub csp: HeaderValue,
}

pub struct App {
    pub users: Users,
    pub sessions: Sessions,
    pub worlds: Arc<Worlds>,
    pub audit: Audit,
    pub cases: Cases,
    pub controls: Controls,
    pub notes: Notes,
    pub schedule: Schedule,
    pub templates: Templates,
    pub canned: CannedTexts,
    pub clips: Clips,
    pub prefs: Prefs,
    pub staff_levels: crate::api::access::StaffLevels,
    pub address_salt: String,
    pub media_dir: PathBuf,
    pub config: HttpConfig,
    pub dummy_hash: String,
}

impl App {
    pub fn cookie_name(&self) -> &'static str {
        // `__Host-` makes the browser insist on Secure, Path=/ and no Domain.
        if self.config.https {
            "__Host-rs-admin"
        } else {
            "rs-admin"
        }
    }

    pub fn set_cookie(&self, token: &str) -> HeaderValue {
        let secure = if self.config.https { "; Secure" } else { "" };
        HeaderValue::from_str(&format!(
            "{}={token}; Path=/; HttpOnly; SameSite=Strict{secure}",
            self.cookie_name()
        ))
        .expect("tokens are hex")
    }

    pub fn clear_cookie(&self) -> HeaderValue {
        let secure = if self.config.https { "; Secure" } else { "" };
        HeaderValue::from_str(&format!(
            "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{secure}",
            self.cookie_name()
        ))
        .expect("static cookie")
    }

    /// The session token the request carries, if any.
    pub fn token(&self, headers: &HeaderMap) -> Option<String> {
        let prefix = format!("{}=", self.cookie_name());
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .find_map(|c| c.trim().strip_prefix(&prefix).map(str::to_owned))
            .filter(|t| t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit()))
    }
}

/// An API failure, answered as `{"error": "..."}`.
#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self(status, message.into())
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "Your session has ended. Sign in again.",
        )
    }

    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, "Your role can't do that.")
    }

    pub fn internal(e: impl std::fmt::Display) -> Self {
        tracing::error!("{e:#}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong.")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

/// Where a request came from.
#[derive(Clone, Debug)]
pub struct Client {
    pub ip: IpAddr,
    pub user_agent: String,
}

impl FromRequestParts<Arc<App>> for Client {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &Arc<App>) -> Result<Self, ApiError> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip())
            .ok_or_else(|| ApiError::internal("no peer address"))?;
        let ip = app
            .config
            .real_ip_header
            .as_ref()
            .and_then(|name| parts.headers.get(name))
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(peer);
        let user_agent = parts
            .headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        Ok(Self { ip, user_agent })
    }
}

/// Unsafe requests must carry the session's CSRF token.
fn check_csrf(parts: &Parts, session: &Session) -> Result<(), ApiError> {
    if parts.method == Method::GET || parts.method == Method::HEAD {
        return Ok(());
    }
    let sent = parts
        .headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if constant_time_eq(sent.as_bytes(), session.csrf.as_bytes()) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "This page is out of date. Reload it and try again.",
        ))
    }
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// A signed-in staff member, re-checked against the accounts file on every
/// request so a disabled account or changed role takes effect at once.
pub struct Staff {
    pub session: Session,
    pub user: User,
    pub client: Client,
}

impl Staff {
    pub fn name(&self) -> &str {
        &self.user.name
    }

    pub fn require_admin(&self) -> Result<(), ApiError> {
        if self.user.role.can_command() {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }

    /// Refuses a trial moderator, saying what they can't do.
    pub fn refuse_trial(&self, what: &str) -> Result<(), ApiError> {
        if self.user.role.is_trial() {
            Err(ApiError::new(
                StatusCode::FORBIDDEN,
                format!("Trial moderators can't {what}; ask a moderator."),
            ))
        } else {
            Ok(())
        }
    }

    pub fn require_developer(&self) -> Result<(), ApiError> {
        if self.user.role == Role::Developer {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }

    pub fn actor(&self) -> rs_admin_api::Actor {
        rs_admin_api::Actor {
            name: self.user.name.clone(),
            staff_level: self.user.role.staff_level(),
        }
    }
}

impl FromRequestParts<Arc<App>> for Staff {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &Arc<App>) -> Result<Self, ApiError> {
        let client = Client::from_request_parts(parts, app).await?;
        let token = app
            .token(&parts.headers)
            .ok_or_else(ApiError::unauthorized)?;
        let session = app
            .sessions
            .get(&token)
            .ok_or_else(ApiError::unauthorized)?;
        if session.stage != Stage::SignedIn {
            return Err(ApiError::unauthorized());
        }
        check_csrf(parts, &session)?;
        let user = app
            .users
            .find(&session.user)
            .map_err(ApiError::internal)?
            .filter(|u| u.accepts_session(session.created_ms));
        let Some(user) = user else {
            app.sessions.end(&token);
            return Err(ApiError::unauthorized());
        };
        Ok(Self {
            session,
            user,
            client,
        })
    }
}

/// A sign-in part way through: password accepted, second factor not yet.
pub struct Pending {
    pub token: String,
    pub session: Session,
    pub client: Client,
}

impl FromRequestParts<Arc<App>> for Pending {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &Arc<App>) -> Result<Self, ApiError> {
        let client = Client::from_request_parts(parts, app).await?;
        let token = app
            .token(&parts.headers)
            .ok_or_else(ApiError::unauthorized)?;
        let session = app
            .sessions
            .get(&token)
            .ok_or_else(ApiError::unauthorized)?;
        if session.stage == Stage::SignedIn {
            return Err(ApiError::bad_request("You're already signed in."));
        }
        check_csrf(parts, &session)?;
        Ok(Self {
            token,
            session,
            client,
        })
    }
}

/// Refuses unsafe requests from other origins, or that aren't JSON: with
/// SameSite=Strict cookies and the CSRF token, a third line of defence
/// against cross-site requests.
async fn same_origin(State(app): State<Arc<App>>, req: Request, next: Next) -> Response {
    let method = req.method();
    if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
        return next.run(req).await;
    }
    let headers = req.headers();
    let expected = app.config.origin.clone().or_else(|| {
        let host = headers.get(header::HOST)?.to_str().ok()?;
        let scheme = if app.config.https { "https" } else { "http" };
        Some(format!("{scheme}://{host}"))
    });
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    if expected.is_none() || origin != expected.as_deref() {
        return ApiError::new(StatusCode::FORBIDDEN, "Cross-origin request refused.")
            .into_response();
    }
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    // Uploads are the file itself; they need the CSRF token like the rest.
    let json = content_type.starts_with("application/json")
        || (UPLOAD_PATHS.contains(&req.uri().path()) && content_type == "application/octet-stream");
    if !json {
        return ApiError::new(StatusCode::UNSUPPORTED_MEDIA_TYPE, "Requests must be JSON.")
            .into_response();
    }
    next.run(req).await
}

/// The console's Content-Security-Policy. Only the worlds' replay pages
/// (`client` in the worlds file) may be framed.
pub fn csp(worlds: &[crate::worlds::WorldConfig]) -> HeaderValue {
    let mut frames: Vec<String> = worlds
        .iter()
        .filter_map(|w| reqwest::Url::parse(w.client.as_deref()?).ok())
        .map(|url| url.origin().ascii_serialization())
        .filter(|origin| origin != "null")
        .collect();
    frames.sort();
    frames.dedup();
    let frame_src = if frames.is_empty() {
        "'none'".to_owned()
    } else {
        frames.join(" ")
    };
    HeaderValue::from_str(&format!(
        "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' blob:;          media-src 'self'; font-src 'self'; connect-src 'self'; manifest-src 'self';          frame-src {frame_src}; base-uri 'none'; form-action 'self'; frame-ancestors 'none'"
    ))
    .expect("origins are header-safe")
}

/// Headers every response carries.
async fn security_headers(State(app): State<Arc<App>>, req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_SECURITY_POLICY, app.config.csp.clone());
    // Uploads, named by their content, may be kept; nothing else.
    let cacheable = headers.contains_key(header::CACHE_CONTROL);
    let mut set = |name: HeaderName, value: &'static str| {
        headers.insert(name, HeaderValue::from_static(value));
    };
    set(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    set(header::X_FRAME_OPTIONS, "DENY");
    set(header::REFERRER_POLICY, "no-referrer");
    if !cacheable {
        set(header::CACHE_CONTROL, "no-store");
    }
    set(
        HeaderName::from_static("permissions-policy"),
        "camera=(), microphone=(), geolocation=(), payment=(), usb=()",
    );
    set(
        HeaderName::from_static("cross-origin-opener-policy"),
        "same-origin",
    );
    set(
        HeaderName::from_static("cross-origin-resource-policy"),
        "same-origin",
    );
    set(HeaderName::from_static("x-robots-tag"), "noindex, nofollow");
    if app.config.https {
        set(
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=31536000; includeSubDomains",
        );
    }
    response
}

/// The frontend's files: embedded in release builds, read from disk in
/// debug builds so edits show on reload.
mod assets {
    use std::borrow::Cow;

    pub struct Asset {
        pub name: &'static str,
        pub content_type: &'static str,
        #[cfg(not(debug_assertions))]
        pub body: &'static [u8],
    }

    macro_rules! asset {
        ($name:literal, $type:literal) => {
            Asset {
                name: $name,
                content_type: $type,
                #[cfg(not(debug_assertions))]
                body: include_bytes!(concat!("../web/", $name)),
            }
        };
    }

    pub const ASSETS: &[Asset] = &[
        asset!("index.html", "text/html; charset=utf-8"),
        asset!("app.css", "text/css; charset=utf-8"),
        asset!("app.js", "text/javascript; charset=utf-8"),
        asset!("icon.svg", "image/svg+xml"),
        asset!("manifest.webmanifest", "application/manifest+json"),
        asset!("fonts/inter.woff2", "font/woff2"),
        asset!("fonts/jetbrains-mono.woff2", "font/woff2"),
    ];

    impl Asset {
        pub fn body(&self) -> Cow<'static, [u8]> {
            #[cfg(debug_assertions)]
            {
                let path = concat!(env!("CARGO_MANIFEST_DIR"), "/web/");
                std::fs::read(format!("{path}{}", self.name))
                    .unwrap_or_default()
                    .into()
            }
            #[cfg(not(debug_assertions))]
            {
                self.body.into()
            }
        }
    }
}

fn serve_asset(name: &str) -> Response {
    let asset = assets::ASSETS
        .iter()
        .find(|a| a.name == name)
        .unwrap_or(&assets::ASSETS[0]);
    ([(header::CONTENT_TYPE, asset.content_type)], asset.body()).into_response()
}

/// Static files by name; any other page path is the app itself, which
/// routes on the client.
async fn frontend(req: Request) -> Response {
    let path = req.uri().path().trim_start_matches('/');
    if path.starts_with("api/") || path == "api" {
        return ApiError::new(StatusCode::NOT_FOUND, "No such endpoint.").into_response();
    }
    if req.method() != Method::GET && req.method() != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    serve_asset(if assets::ASSETS.iter().any(|a| a.name == path) {
        path
    } else {
        "index.html"
    })
}

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .nest("/api", api::router())
        .route("/", get(|| async { serve_asset("index.html") }))
        .fallback(frontend)
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(middleware::from_fn_with_state(app.clone(), same_origin))
        .layer(middleware::from_fn_with_state(
            app.clone(),
            security_headers,
        ))
        .with_state(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_compares_whole_strings() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"", b"a"));
    }
}
