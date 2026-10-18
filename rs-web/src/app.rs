use crate::templates::Templates;
use crate::worlds::World;
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::response::{IntoResponse, Response};
use rs_db::{Db, Passwords};
use rs_pack::cache::wordenc::WordEncProvider;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use tracing::error;

pub struct App {
    pub db: Db,
    pub passwords: Passwords,
    pub templates: Templates,
    pub worlds: Vec<World>,
    pub wordenc: Option<WordEncProvider>,
    pub site: SiteConfig,
}

pub struct SiteConfig {
    pub https: bool,
    pub public_host: String,
    pub port: u16,
    pub real_ip_header: Option<HeaderName>,
}

impl App {
    /// The visitor's IP: the configured proxy header if present and valid,
    /// otherwise the socket peer.
    pub fn client_ip(&self, headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
        self.site
            .real_ip_header
            .as_ref()
            .and_then(|name| headers.get(name))
            .and_then(|v| v.to_str().ok())
            // x-forwarded-for style lists put the client first.
            .and_then(|v| v.split(',').next())
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(peer.ip())
    }

    /// The site's own base URL, e.g. `http://localhost:8000`.
    pub fn base_url(&self) -> String {
        let (scheme, default_port) = if self.site.https {
            ("https", 443)
        } else {
            ("http", 80)
        };
        if self.site.port == default_port {
            format!("{scheme}://{}", self.site.public_host)
        } else {
            format!("{scheme}://{}:{}", self.site.public_host, self.site.port)
        }
    }
}

/// Builds the game's chat filter from `content/<REV>/wordenc`, the same way
/// the game server does. `None` if that folder doesn't exist.
pub fn load_wordenc() -> Option<WordEncProvider> {
    let jag = rs_pack::pack::wordenc::pack_wordenc(Path::new(rs_pack::CONTENT_DIR));
    (!jag.is_empty()).then(|| WordEncProvider::from_jag(&jag))
}

/// Any handler failure: logged, and shown to the visitor as a bare 500.
pub struct AppError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(e: E) -> Self {
        Self(e.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("{:#}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}
