use futures_util::StreamExt;
use futures_util::stream;
use rs_admin_api::{
    MediaSaved, NONCE_HEADER, OnlinePlayer, PlayerRecord, ReplayGet, SIGNATURE_HEADER,
    TIMESTAMP_HEADER, WorldStatus, path,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// How often every world's status is refreshed.
const POLL_INTERVAL: Duration = Duration::from_secs(5);
/// How many worlds are called at once.
const CONCURRENCY: usize = 32;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
/// For a developer's change to a logged-out player, which the world makes
/// in steps through the database.
pub const EDIT_TIMEOUT: Duration = Duration::from_secs(25);
/// For a news image or video, up to 50 MB.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(120);
/// For a session replay: hours of play are tens of megabytes.
const REPLAY_TIMEOUT: Duration = Duration::from_secs(120);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// A world, as listed in the console's `worlds.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldConfig {
    pub id: u16,
    pub node: u8,
    #[serde(default)]
    pub region: String,
    pub url: String,
    #[serde(default)]
    pub client: Option<String>,
}

pub fn load(path: &Path) -> anyhow::Result<Vec<WorldConfig>> {
    let worlds: Vec<WorldConfig> = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let mut nodes: Vec<u8> = worlds.iter().map(|w| w.node).collect();
    nodes.sort_unstable();
    if nodes.windows(2).any(|w| w[0] == w[1]) {
        anyhow::bail!("two worlds share a node id");
    }
    Ok(worlds)
}

/// What the console last heard from a world.
#[derive(Clone, Debug, Serialize)]
pub struct WorldView {
    #[serde(flatten)]
    pub config: WorldConfig,
    pub online: bool,
    pub status: Option<WorldStatus>,
    pub error: Option<String>,
    pub latency_ms: Option<u64>,
    pub checked_ago_ms: u64,
}

struct Snapshot {
    status: Result<WorldStatus, String>,
    latency: Duration,
    at: Instant,
}

/// One world's answer to a command.
#[derive(Clone, Debug, Serialize)]
pub struct Reply<T> {
    pub node: u8,
    pub world: u16,
    #[serde(flatten)]
    pub result: Outcome<T>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome<T> {
    Answered { answer: T },
    Failed { error: String },
}

/// Why a call to an admin API didn't work.
#[derive(Debug)]
pub struct CallError {
    pub message: String,
    pub refused: bool,
}

impl CallError {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            refused: false,
        }
    }
}

/// Whose admin API answered, for its errors.
#[derive(Clone, Copy)]
enum Server {
    World,
    Website,
}

pub struct Worlds {
    worlds: Vec<WorldConfig>,
    web: Option<String>,
    secret: Box<[u8]>,
    client: reqwest::Client,
    snapshots: RwLock<HashMap<u8, Snapshot>>,
}

impl Worlds {
    pub fn new(worlds: Vec<WorldConfig>, web: Option<String>, secret: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("building the HTTP client");
        Self {
            worlds,
            web,
            secret: secret.as_bytes().into(),
            client,
            snapshots: RwLock::default(),
        }
    }

    pub fn get(&self, node: u8) -> Option<&WorldConfig> {
        self.worlds.iter().find(|w| w.node == node)
    }

    /// Every world as of its last check, in config order.
    pub fn views(&self) -> Vec<WorldView> {
        let snapshots = self.snapshots.read().unwrap_or_else(|e| e.into_inner());
        self.worlds
            .iter()
            .map(|config| {
                let snapshot = snapshots.get(&config.node);
                let (status, error) = match snapshot.map(|s| &s.status) {
                    Some(Ok(status)) => (Some(status.clone()), None),
                    Some(Err(e)) => (None, Some(e.clone())),
                    None => (None, Some("Not checked yet".to_owned())),
                };
                WorldView {
                    config: config.clone(),
                    online: status.is_some(),
                    status,
                    error,
                    latency_ms: snapshot
                        .filter(|s| s.status.is_ok())
                        .map(|s| s.latency.as_millis() as u64),
                    checked_ago_ms: snapshot.map_or(0, |s| s.at.elapsed().as_millis() as u64),
                }
            })
            .collect()
    }

    /// The online worlds at protocol revision `revision`.
    pub fn online_at(&self, revision: &str) -> Vec<WorldConfig> {
        let snapshots = self.snapshots.read().unwrap_or_else(|e| e.into_inner());
        self.worlds
            .iter()
            .filter(|w| {
                snapshots
                    .get(&w.node)
                    .and_then(|s| s.status.as_ref().ok())
                    .is_some_and(|s| s.revision == revision)
            })
            .cloned()
            .collect()
    }

    /// A session's replay file, from any online world at its revision.
    pub async fn replay_file(&self, id: &str, revision: &str) -> Result<Vec<u8>, String> {
        let body =
            serde_json::to_vec(&ReplayGet { id: id.to_owned() }).expect("serialising a request");
        let mut error = format!("No world on revision {revision} is online to play it");
        for world in self.online_at(revision) {
            let response = self
                .send(
                    &world.url,
                    Server::World,
                    path::REPLAY_DATA,
                    Some(body.clone()),
                    REPLAY_TIMEOUT,
                )
                .await;
            match response {
                Ok(response) => {
                    return response
                        .bytes()
                        .await
                        .map(|b| b.to_vec())
                        .map_err(|e| format!("Unreadable answer: {e}"));
                }
                Err(e) if e.message == "Not found" => {
                    return Err("That recording is gone: it was pruned".to_owned());
                }
                Err(e) => error = e.message,
            }
        }
        Err(error)
    }

    /// The worlds that answered their last check.
    pub fn online(&self) -> Vec<WorldConfig> {
        let snapshots = self.snapshots.read().unwrap_or_else(|e| e.into_inner());
        self.worlds
            .iter()
            .filter(|w| snapshots.get(&w.node).is_some_and(|s| s.status.is_ok()))
            .cloned()
            .collect()
    }

    /// Calls one world's admin API, signing the request.
    async fn call<T: DeserializeOwned>(
        &self,
        world: &WorldConfig,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<T, String> {
        self.call_at(&world.url, Server::World, path, body, REQUEST_TIMEOUT)
            .await
    }

    /// Calls the world on `node`, with time for `timeout`.
    pub async fn ask_node<B: Serialize, T: DeserializeOwned>(
        &self,
        node: u8,
        path: &str,
        body: &B,
        timeout: Duration,
    ) -> Result<T, String> {
        let world = self
            .get(node)
            .ok_or("That world isn't in the console's list")?;
        let body = serde_json::to_vec(body).expect("serialising a request");
        self.call_at(&world.url, Server::World, path, Some(body), timeout)
            .await
    }

    /// Calls the website's admin API.
    pub async fn web<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let url = self.web_url()?;
        let body = serde_json::to_vec(body).expect("serialising a request");
        self.call_at(url, Server::Website, path, Some(body), REQUEST_TIMEOUT)
            .await
    }

    async fn call_at<T: DeserializeOwned>(
        &self,
        base: &str,
        server: Server,
        path: &str,
        body: Option<Vec<u8>>,
        timeout: Duration,
    ) -> Result<T, String> {
        self.send(base, server, path, body, timeout)
            .await
            .map_err(|e| e.message)?
            .json()
            .await
            .map_err(|e| format!("Unreadable answer: {e}"))
    }

    /// Signs and sends a request (a `POST` of `body`, or a `GET`), and turns
    /// the ways it can fail into what to tell staff.
    async fn send(
        &self,
        base: &str,
        server: Server,
        path: &str,
        body: Option<Vec<u8>>,
        timeout: Duration,
    ) -> Result<reqwest::Response, CallError> {
        let method = if body.is_some() { "POST" } else { "GET" };
        let body = body.unwrap_or_default();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let nonce = crate::sessions::random_token();
        let signature = rs_admin_api::sign(&self.secret, timestamp, &nonce, method, path, &body);

        let url = format!("{}{}", base.trim_end_matches('/'), path);
        let request = if method == "POST" {
            self.client
                .post(&url)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body)
        } else {
            self.client.get(&url)
        };
        let response = request
            .timeout(timeout)
            .header(TIMESTAMP_HEADER, timestamp.to_string())
            .header(NONCE_HEADER, nonce)
            .header(SIGNATURE_HEADER, signature)
            .send()
            .await
            .map_err(|e| {
                CallError::failed(if e.is_timeout() {
                    "Timed out".to_owned()
                } else if e.is_connect() {
                    "Unreachable".to_owned()
                } else {
                    format!("Request failed: {e}")
                })
            })?;
        let status = response.status();
        let (world, website) = match status {
            s if s.is_success() => return Ok(response),
            reqwest::StatusCode::UNAUTHORIZED => {
                return Err(CallError::failed(
                    "Refused the console's signature (check RS_ADMIN_SECRET and clocks)",
                ));
            }
            reqwest::StatusCode::SERVICE_UNAVAILABLE => (
                "The world is busy",
                "The website isn't set up to keep uploads (--media-dir)",
            ),
            reqwest::StatusCode::BAD_GATEWAY => (
                "The world couldn't reach the game database",
                "The website couldn't reach the game database",
            ),
            reqwest::StatusCode::UNPROCESSABLE_ENTITY => (
                "That isn't a valid name",
                "The website refused it: the post or its category is gone, or the title or text is empty",
            ),
            reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE => (
                "Unsupported file",
                "That isn't an image or video the site takes (PNG, JPEG, GIF, WebP, MP4 or WebM)",
            ),
            reqwest::StatusCode::PAYLOAD_TOO_LARGE => ("Too large", "That file is over 50 MB"),
            reqwest::StatusCode::NOT_FOUND => ("Not found", "The website has no such file"),
            s => return Err(CallError::failed(format!("Answered {s}"))),
        };
        Err(CallError {
            message: match server {
                Server::World => world,
                Server::Website => website,
            }
            .to_owned(),
            refused: status == reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE
                || status == reqwest::StatusCode::PAYLOAD_TOO_LARGE,
        })
    }

    /// Uploads an image or video for news to the website.
    pub async fn web_upload(&self, bytes: Vec<u8>) -> Result<MediaSaved, CallError> {
        let url = self.web_url().map_err(CallError::failed)?;
        self.send(
            url,
            Server::Website,
            path::NEWS_MEDIA,
            Some(bytes),
            UPLOAD_TIMEOUT,
        )
        .await?
        .json()
        .await
        .map_err(|e| CallError::failed(format!("Unreadable answer: {e}")))
    }

    /// An uploaded image or video back from the website, with its type.
    pub async fn web_media(&self, name: &str) -> Result<(String, Vec<u8>), String> {
        let url = self.web_url()?;
        let path = format!("{}/{name}", path::NEWS_MEDIA);
        let response = self
            .send(url, Server::Website, &path, None, UPLOAD_TIMEOUT)
            .await
            .map_err(|e| e.message)?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_owned();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Unreadable answer: {e}"))?;
        Ok((content_type, bytes.to_vec()))
    }

    fn web_url(&self) -> Result<&str, String> {
        self.web
            .as_deref()
            .ok_or_else(|| "The console isn't set up to reach the website (--web-url)".to_owned())
    }

    pub async fn players(&self, world: &WorldConfig) -> Result<Vec<OnlinePlayer>, String> {
        self.call(world, path::PLAYERS, None).await
    }

    /// Asks the online worlds in turn until one answers: for lookups in the
    /// game database, which any of them can make.
    pub async fn ask<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        self.ask_in(self.online(), path, body).await
    }

    /// Like [`Self::ask`], trying every listed world: for the command line,
    /// which has no status checks to go by.
    pub async fn ask_any<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        self.ask_in(self.worlds.clone(), path, body).await
    }

    async fn ask_in<B: Serialize, T: DeserializeOwned>(
        &self,
        worlds: Vec<WorldConfig>,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let body = serde_json::to_vec(body).expect("serialising a request");
        let mut error = "No world is online to ask".to_owned();
        for world in worlds {
            match self.call(&world, path, Some(body.clone())).await {
                Ok(answer) => return Ok(answer),
                // A bad request is bad everywhere.
                Err(e) if e == "That isn't a valid name" => return Err(e),
                Err(e) => error = e,
            }
        }
        Err(error)
    }

    /// The game account behind a new console account, which must be staff
    /// (level 1 or more) in the game.
    pub fn game_staff(record: Option<PlayerRecord>, name: &str) -> Result<PlayerRecord, String> {
        match record {
            None => Err(format!("No player named {name} is registered in the game")),
            Some(player) if player.staff_level == 0 => Err(format!(
                "{} isn't staff in the game; give them staff level 1 or more first",
                player.name
            )),
            Some(player) => Ok(player),
        }
    }

    /// Sends a command to each of `targets`, all at once. `body` builds the
    /// request for the n-th target.
    pub async fn command<B: Serialize, T: DeserializeOwned + Send>(
        &self,
        targets: &[WorldConfig],
        path: &'static str,
        body: impl Fn(usize) -> B,
    ) -> Vec<Reply<T>> {
        // Owned items: futures borrowing the iterator's items trip up the
        // compiler's Send inference for handlers.
        let calls = targets.iter().cloned().enumerate().map(|(i, world)| {
            let body = serde_json::to_vec(&body(i)).expect("serialising a request");
            async move {
                let result = match self.call(&world, path, Some(body)).await {
                    Ok(answer) => Outcome::Answered { answer },
                    Err(error) => Outcome::Failed { error },
                };
                Reply {
                    node: world.node,
                    world: world.id,
                    result,
                }
            }
        });
        let mut replies: Vec<Reply<T>> = stream::iter(calls)
            .buffer_unordered(CONCURRENCY)
            .collect()
            .await;
        let order: HashMap<u8, usize> = targets
            .iter()
            .enumerate()
            .map(|(i, w)| (w.node, i))
            .collect();
        replies.sort_by_key(|r| order.get(&r.node).copied());
        replies
    }

    /// Re-checks `targets` (every world when `None`).
    pub async fn refresh(&self, targets: Option<&[u8]>) {
        let worlds: Vec<WorldConfig> = self
            .worlds
            .iter()
            .filter(|w| targets.is_none_or(|t| t.contains(&w.node)))
            .cloned()
            .collect();
        let checks = worlds.into_iter().map(|world| async move {
            let start = Instant::now();
            let status = self.call::<WorldStatus>(&world, path::STATUS, None).await;
            (
                world.node,
                Snapshot {
                    status,
                    latency: start.elapsed(),
                    at: Instant::now(),
                },
            )
        });
        let results: Vec<(u8, Snapshot)> = stream::iter(checks)
            .buffer_unordered(CONCURRENCY)
            .collect()
            .await;

        let mut snapshots = self.snapshots.write().unwrap_or_else(|e| e.into_inner());
        for (node, snapshot) in results {
            let was_online = snapshots.get(&node).map(|s| s.status.is_ok());
            match (&snapshot.status, was_online) {
                (Ok(_), Some(false)) => info!("World node {} is reachable again", node),
                (Err(e), Some(true) | None) => warn!("World node {} is unreachable: {}", node, e),
                _ => {}
            }
            snapshots.insert(node, snapshot);
        }
    }
}

/// Keeps every world's status fresh.
pub async fn poll(worlds: Arc<Worlds>) {
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        worlds.refresh(None).await;
    }
}
