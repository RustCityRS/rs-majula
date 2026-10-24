use crate::metrics::Metrics;
use crate::rate_limit::RateLimit;
use crate::templates::Templates;
use crate::token::SignInTokens;
use crate::turnstile::Turnstile;
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
    /// Registration's person check; `None` leaves it off.
    pub turnstile: Option<Turnstile>,
    /// `/create` submissions, each of which checks a username.
    pub name_checks: RateLimit,
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
    /// See [`App::base_url`]. The site doesn't serve HTTPS itself, so with
    /// `https` it's behind a proxy on the standard port, whatever `port` it
    /// listens on.
    pub fn base_url(&self) -> String {
        if self.https {
            format!("https://{}", self.public_host)
        } else if self.port == 80 {
            format!("http://{}", self.public_host)
        } else {
            format!("http://{}:{}", self.public_host, self.port)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn site(https: bool, port: u16) -> SiteConfig {
        SiteConfig {
            https,
            public_host: "rustcity.org".to_owned(),
            port,
            real_ip_header: None,
        }
    }

    #[test]
    fn base_url_leaves_out_ports_a_visitor_never_uses() {
        // Behind a TLS proxy, the port it listens on isn't the public one.
        assert_eq!(site(true, 8000).base_url(), "https://rustcity.org");
        assert_eq!(site(false, 80).base_url(), "http://rustcity.org");
        assert_eq!(site(false, 8000).base_url(), "http://rustcity.org:8000");
    }
}
