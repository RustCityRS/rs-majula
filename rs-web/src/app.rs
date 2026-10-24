use crate::metrics::Metrics;
use crate::templates::Templates;
use crate::token::SignInTokens;
use crate::worlds::{LiveWorlds, World};
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::response::{IntoResponse, Response};
use rs_db::{Db, Passwords};
use rs_pack::cache::wordenc::WordEncProvider;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use tokio::sync::Semaphore;
use tokio::task::spawn_blocking;
use tracing::error;

/// Passwords hashed or checked at once: argon2 takes 19 MiB for each.
pub const HASHING_AT_ONCE: usize = 4;

pub struct App {
    pub db: Db,
    pub media_dir: Option<PathBuf>,
    pub passwords: Passwords,
    /// [`HASHING_AT_ONCE`] permits, one for each hash running.
    pub hashing: Semaphore,
    pub templates: Templates,
    /// The Message Centre's sign-ins, carried from page to page.
    pub tokens: SignInTokens,
    pub worlds: Vec<World>,
    pub live_worlds: LiveWorlds,
    pub wordenc: Option<WordEncProvider>,
    pub site: SiteConfig,
    pub metrics: Metrics,
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

    /// Runs `work` with the passwords on a blocking thread, no more than
    /// [`HASHING_AT_ONCE`] at a time.
    pub async fn with_passwords<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Passwords) -> T + Send + 'static,
    ) -> Result<T, AppError> {
        let _permit = self.hashing.acquire().await?;
        let passwords = self.passwords.clone();
        Ok(spawn_blocking(move || work(&passwords)).await?)
    }

    /// Every world with its current player count and online state.
    pub fn worlds_now(&self) -> Vec<World> {
        self.live_worlds.apply(&self.worlds)
    }

    /// The site's own base URL, e.g. `http://localhost:8000`.
    pub fn base_url(&self) -> String {
        self.site.base_url()
    }
}

impl SiteConfig {
    /// See [`App::base_url`].
    pub fn base_url(&self) -> String {
        let (scheme, default_port) = if self.https {
            ("https", 443)
        } else {
            ("http", 80)
        };
        if self.port == default_port {
            format!("{scheme}://{}", self.public_host)
        } else {
            format!("{scheme}://{}:{}", self.public_host, self.port)
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
