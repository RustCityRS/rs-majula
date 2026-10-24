use crate::tui::log_layer::{LogBuffer, TuiLogLayer};
use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum};
use futures_util::SinkExt;
use futures_util::StreamExt;
use mpsc::{UnboundedReceiver, unbounded_channel};
use rs_crypto::rsa::{RsaKey, load_rsa_key};
use rs_db::LoginDevice;
use rs_engine::{CycleResult, Engine};
use rs_engine::{EtherInbound, EtherOutbound, ether_client_task};
use rs_engine::{LoginRequest, TickStats};
use rs_pack::cache::CacheStore;
use rs_pack::cache::script::ScriptProvider;
use std::collections::HashMap;
use std::io::IsTerminal;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, watch};
use tokio::time::{self, Duration};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{Bytes, Message};
use tracing::{Level, debug, error, info, warn};
use tracing_subscriber::Layer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use watch::{Receiver, Sender, channel};

use crossterm::execute;
use crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
use mimalloc::MiMalloc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use time::MissedTickBehavior;

mod admin;
mod http;
mod jaggrab;
mod metrics;
mod recovery;
mod socket;
pub mod tui;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn release_freed_memory() {
    unsafe { libmimalloc_sys::mi_collect(true) };
}

#[cfg(windows)]
fn trim_working_set() {
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn SetProcessWorkingSetSize(handle: isize, min: usize, max: usize) -> i32;
    }
    unsafe { SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX) };
}

#[cfg(not(windows))]
fn trim_working_set() {}

#[cfg(debug_assertions)]
const MAX_CONNECTIONS_PER_IP: usize = 4;
#[cfg(not(debug_assertions))]
const MAX_CONNECTIONS_PER_IP: usize = 2;

const MAX_GAME_CONNECTIONS: usize = 8192;

const MAX_HTTP_PER_IP: usize = 16;
const MAX_HTTP_CONNECTIONS: usize = 8192;

const MAX_JAGGRAB_PER_IP: usize = 8;
const MAX_JAGGRAB_CONNECTIONS: usize = 2048;

struct GuardCounts {
    per_ip: HashMap<IpAddr, usize>,
    total: usize,
    served: u64,
}

#[derive(Clone)]
pub struct ConnectionGuard {
    counts: Arc<Mutex<GuardCounts>>,
    max_per_ip: usize,
    max_total: usize,
}

#[derive(Clone)]
pub struct ConnectionGuards {
    pub game: ConnectionGuard,
    pub http: ConnectionGuard,
    pub jaggrab: ConnectionGuard,
}

impl ConnectionGuards {
    fn new() -> Self {
        Self {
            game: ConnectionGuard::new(MAX_CONNECTIONS_PER_IP, MAX_GAME_CONNECTIONS),
            http: ConnectionGuard::new(MAX_HTTP_PER_IP, MAX_HTTP_CONNECTIONS),
            jaggrab: ConnectionGuard::new(MAX_JAGGRAB_PER_IP, MAX_JAGGRAB_CONNECTIONS),
        }
    }
}

impl ConnectionGuard {
    fn new(max_per_ip: usize, max_total: usize) -> Self {
        Self {
            counts: Arc::new(Mutex::new(GuardCounts {
                per_ip: HashMap::new(),
                total: 0,
                served: 0,
            })),
            max_per_ip,
            max_total,
        }
    }

    fn active(&self) -> usize {
        self.counts.lock().unwrap().total
    }

    fn served(&self) -> u64 {
        self.counts.lock().unwrap().served
    }

    fn try_acquire(&self, ip: IpAddr) -> Option<ConnectionPermit> {
        let mut counts = self.counts.lock().unwrap();
        if counts.total >= self.max_total {
            return None;
        }
        let count = counts.per_ip.entry(ip).or_insert(0);
        if *count >= self.max_per_ip {
            return None;
        }
        *count += 1;
        counts.total += 1;
        counts.served += 1;
        Some(ConnectionPermit {
            ip,
            counts: self.counts.clone(),
        })
    }
}

pub struct ConnectionPermit {
    ip: IpAddr,
    counts: Arc<Mutex<GuardCounts>>,
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        let mut counts = self.counts.lock().unwrap();
        counts.total -= 1;
        let count = counts.per_ip.get_mut(&self.ip).unwrap();
        *count -= 1;
        if *count == 0 {
            counts.per_ip.remove(&self.ip);
        }
    }
}

struct ShutdownGuard;

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        restore_terminal_and_sidecar();
    }
}

static SIDECAR_PID: AtomicU32 = AtomicU32::new(0);

fn restore_terminal_and_sidecar() {
    shutdown_sidecar();
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
}

fn graceful_exit(code: i32) -> ! {
    restore_terminal_and_sidecar();
    std::process::exit(code);
}

/// Secrets read at startup. Each comes from its flag or environment variable;
/// only debug builds fall back to the local development values.
struct Secrets {
    db_pass: String,
    pepper: String,
    cookie: String,
    admin: String,
}

impl Secrets {
    fn resolve(args: &Args) -> Result<Self> {
        Ok(Self {
            db_pass: db_pass(args)?,
            pepper: rs_db::require_secret(args.pepper.clone(), "RS_PEPPER", "localhost")
                .map_err(|e| anyhow!(e))?,
            cookie: rs_db::require_secret(args.cookie.clone(), "RS_COOKIE", "rs_secret")
                .map_err(|e| anyhow!(e))?,
            admin: rs_db::require_secret(args.admin_secret.clone(), "RS_ADMIN_SECRET", "rs_admin")
                .map_err(|e| anyhow!(e))?,
        })
    }
}

fn db_pass(args: &Args) -> Result<String> {
    rs_db::require_secret(args.db_pass.clone(), "RS_DB_PASS", "password").map_err(|e| anyhow!(e))
}

fn db_config(args: &Args, pass: String) -> rs_db::DbConfig {
    rs_db::DbConfig {
        host: args.db_host.clone(),
        port: args.db_port,
        name: args.db_name.clone(),
        user: args.db_user.clone(),
        pass,
    }
}

/// How to launch the ether sidecar, and the environment its
/// `config/runtime.exs` reads.
#[derive(Clone)]
struct EtherEnv {
    release: Option<PathBuf>,
    node_host: String,
    cluster: String,
    auto_migrate: bool,
    db: rs_db::DbConfig,
}

#[cfg(windows)]
const RELEASE_SCRIPT: &str = "rs_ether.bat";
#[cfg(not(windows))]
const RELEASE_SCRIPT: &str = "rs_ether";

impl EtherEnv {
    fn new(args: &Args, db: rs_db::DbConfig) -> Result<Self> {
        let release = match &args.ether_release {
            Some(root) => {
                let root = std::path::absolute(root)?;
                let script = root.join("bin").join(RELEASE_SCRIPT);
                if !script.is_file() {
                    bail!("--ether-release: {} not found", script.display());
                }
                Some(root)
            }
            None => None,
        };
        Ok(Self {
            release,
            node_host: args.node_host.clone(),
            cluster: args.cluster.clone(),
            auto_migrate: args.auto_migrate,
            db,
        })
    }

    /// `mix <args>` in rs-ether/, for running ether from source.
    fn mix(&self, args: &[&str]) -> Command {
        let mut cmd = elixir_tool("mix");
        cmd.args(args).current_dir("rs-ether");
        self.with_env(cmd)
    }

    /// `bin/rs_ether <args>` from the built release.
    fn release(&self, root: &Path, args: &[&str]) -> Command {
        let mut cmd = Command::new(root.join("bin").join(RELEASE_SCRIPT));
        cmd.args(args);
        self.with_env(cmd)
    }

    fn with_env(&self, mut cmd: Command) -> Command {
        cmd.env("RS_NODE_HOST", &self.node_host)
            .env("RS_CLUSTER_HOSTS", &self.cluster)
            .env(
                "RS_AUTO_MIGRATE",
                if self.auto_migrate { "true" } else { "false" },
            )
            .env("RS_DB_HOST", &self.db.host)
            .env("RS_DB_PORT", self.db.port.to_string())
            .env("RS_DB_NAME", &self.db.name)
            .env("RS_DB_USER", &self.db.user)
            .env("RS_DB_PASS", &self.db.pass)
            .stdin(Stdio::null());
        cmd
    }

    /// The command that runs this world's ether node until it's killed.
    fn run(&self, node_id: u8, ether_port: u16, node_name: &str, cookie: &str) -> Command {
        let mut cmd = match &self.release {
            Some(root) => {
                let mut cmd = self.release(root, &["start"]);
                cmd.env("RELEASE_DISTRIBUTION", "name")
                    .env("RELEASE_NODE", node_name)
                    .env("RELEASE_COOKIE", cookie);
                cmd
            }
            None => {
                let mut cmd = elixir_tool("elixir");
                cmd.args(["--name", node_name, "--cookie", cookie])
                    .args(["-S", "mix", "run", "--no-halt"])
                    .current_dir("rs-ether");
                self.with_env(cmd)
            }
        };
        cmd.env("RS_NODE_ID", node_id.to_string())
            .env("RS_ETHER_PORT", ether_port.to_string());
        cmd
    }

    /// The command that applies ether's pending Ecto migrations and exits.
    fn migrate(&self) -> Command {
        match &self.release {
            Some(root) => self.release(root, &["eval", "RsEther.Release.migrate()"]),
            None => self.mix(&["ecto.migrate"]),
        }
    }
}

/// A command for one of Elixir's tool scripts. On Windows those are `.bat`
/// wrappers, which only resolve through `cmd`.
fn elixir_tool(program: &str) -> Command {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/c", program]);
        cmd
    }
    #[cfg(not(windows))]
    {
        Command::new(program)
    }
}

/// Server revision, taken from the `REV` build env var (see `.cargo/config.toml`).
pub const REVISION: &str = env!("REV");

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "rs-server")]
#[command(about = "RuneScape Private Server")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(long)]
    http_port: Option<u16>,
    #[arg(long)]
    tcp_port: Option<u16>,
    #[arg(long)]
    jaggrab_port: Option<u16>,
    #[arg(long, default_value = ".keys/private.pem")]
    private_key: PathBuf,
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    members: bool,
    #[arg(long, default_value = "1")]
    multi_xp: u8,
    #[arg(long, default_value = "true")]
    client_pathfinder: bool,
    #[arg(long, action = clap::ArgAction::Set)]
    staff_isolation: Option<bool>,
    #[arg(long, default_value = "false")]
    no_tui: bool,
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    verify: bool,
    #[arg(long, default_value = "false")]
    jaggrab_only: bool,
    #[arg(long)]
    real_ip_header: Option<String>,
    #[arg(long, env = "RS_SITE_ORIGIN")]
    site_origin: Option<String>,
    #[arg(long, default_value = "10")]
    node_id: u8,
    #[arg(long)]
    metrics_port: Option<u16>,
    #[arg(long, default_value = "127.0.0.1")]
    metrics_host: String,
    #[arg(long)]
    admin_port: Option<u16>,
    #[arg(long, default_value = "127.0.0.1")]
    admin_host: String,
    #[arg(long, env = "RS_ADMIN_SECRET", hide_env_values = true)]
    admin_secret: Option<String>,
    #[arg(long, value_enum, default_value_t = LogFormat::Text)]
    log_format: LogFormat,
    #[arg(long, default_value = "rs-server.log")]
    log_file: PathBuf,
    #[arg(long)]
    no_log_file: bool,
    #[arg(long)]
    ether_port: Option<u16>,
    #[arg(long, env = "RS_DB_HOST", default_value = "localhost")]
    db_host: String,
    #[arg(long, env = "RS_DB_PORT", default_value = "5432")]
    db_port: u16,
    #[arg(long, env = "RS_DB_NAME", default_value = "postgres")]
    db_name: String,
    #[arg(long, env = "RS_DB_USER", default_value = "postgres")]
    db_user: String,
    #[arg(long, env = "RS_DB_PASS", hide_env_values = true)]
    db_pass: Option<String>,
    #[arg(long, env = "RS_PEPPER", hide_env_values = true)]
    pepper: Option<String>,
    #[arg(long, env = "RS_COOKIE", hide_env_values = true)]
    cookie: Option<String>,
    #[arg(long, env = "RS_NODE_HOST", default_value = "127.0.0.1")]
    node_host: String,
    #[arg(long, env = "RS_CLUSTER_HOSTS", default_value = "")]
    cluster: String,
    #[arg(long, env = "RS_ETHER_RELEASE")]
    ether_release: Option<PathBuf>,
    #[arg(long, default_value_t = cfg!(debug_assertions), action = clap::ArgAction::Set)]
    auto_register: bool,
    #[arg(long, default_value_t = cfg!(debug_assertions), action = clap::ArgAction::Set)]
    auto_migrate: bool,
    #[arg(long, env = "RS_REPLAY_DAYS", default_value_t = 7)]
    replay_days: u32,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogFormat {
    Text,
    Json,
}

/// A log output in `format`; `time` only applies to text (JSON lines always
/// carry a timestamp).
fn fmt_layer<S, W>(
    format: LogFormat,
    writer: W,
    ansi: bool,
    time: bool,
) -> Box<dyn Layer<S> + Send + Sync>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    W: for<'w> tracing_subscriber::fmt::MakeWriter<'w> + Send + Sync + 'static,
{
    let layer = tracing_subscriber::fmt::layer().with_writer(writer);
    match format {
        LogFormat::Text if time => layer.with_ansi(ansi).boxed(),
        LogFormat::Text => layer.with_ansi(ansi).without_time().boxed(),
        LogFormat::Json => layer.json().flatten_event(true).boxed(),
    }
}

#[derive(Subcommand, Debug)]
enum Cmd {
    Migrate,
}

#[derive(Clone)]
pub struct ServerIO {
    cache: &'static CacheStore,
    rsa: &'static RsaKey,
    new_player_tx: UnboundedSender<LoginRequest>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _shutdown_guard = ShutdownGuard;
    let args = Args::parse();

    // Honor RUST_LOG if set; otherwise default to INFO globally but
    // silence the chatty save + protocol logs and sqlx's per-statement
    // logs and migration NOTICEs.
    let make_filter = || {
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(
                "info,\
                     rs_engine::player_save=warn,\
                     rs_protocol=warn,\
                     sqlx=warn,\
                     runec=error",
            )
        })
    };

    if let Some(Cmd::Migrate) = args.command {
        tracing_subscriber::fmt()
            .without_time()
            .with_env_filter(make_filter())
            .init();
        return migrate(&args).await;
    }
    let secrets = Secrets::resolve(&args)?;

    // File appender - `--log-file`, overwritten each run (no rotation; log
    // volume per session is bounded). Held in `_log_guard` for the lifetime
    // of `main` so the background writer thread keeps draining until shutdown.
    let (file_layer, _log_guard) = if args.no_log_file {
        (None, None)
    } else {
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&args.log_file)
            .with_context(|| format!("opening {}", args.log_file.display()))?;
        let (file_writer, guard) = tracing_appender::non_blocking(log_file);
        let layer = fmt_layer(args.log_format, file_writer, false, true).with_filter(make_filter());
        (Some(layer), Some(guard))
    };

    let registry = tracing_subscriber::registry().with(file_layer);

    // The TUI requires a real terminal - auto-fall-back to headless when
    // stdout is e.g. piped, redirected, or running under IntelliJ's Run
    // tool window (which doesn't interpret cursor/raw-mode escapes).
    let tty = std::io::stdout().is_terminal();
    let use_tui = !args.no_tui && tty;

    if !use_tui {
        let stdout_layer =
            fmt_layer(args.log_format, std::io::stdout, true, false).with_filter(make_filter());
        registry.with(stdout_layer).init();
        let _ = Level::INFO;

        if !args.no_tui && !tty {
            info!(
                "stdout is not a TTY - falling back to headless mode (use a real terminal for the TUI dashboard)"
            );
        }
        info!("Running headless");

        let (stats_tx, _stats_rx) = channel(TickStats::default());
        let (_trigger_tx, trigger_rx) = unbounded_channel::<()>();
        let (shutdown_tx, shutdown_rx) = unbounded_channel::<()>();
        tokio::spawn(async move {
            let signal = shutdown_signal().await;
            info!("{signal} -- saving every player and shutting down ({signal} again to force)");
            let _ = shutdown_tx.send(());
            shutdown_signal().await;
            warn!("Forced exit -- players since their last save are not saved");
            graceful_exit(130);
        });
        bootstrap(
            args,
            secrets,
            stats_tx,
            trigger_rx,
            shutdown_rx,
            ConnectionGuards::new(),
        )
        .await
    } else {
        let log_buf = tui::log_layer::new_buffer();
        let tui_layer = TuiLogLayer::new(log_buf.clone()).with_filter(make_filter());
        registry.with(tui_layer).init();

        run_with_tui(args, secrets, log_buf).await
    }
}

/// Resolves on Ctrl+C, or on SIGTERM where it exists: the signal service
/// managers (systemd, Docker) stop a process with. Returns which one came.
async fn shutdown_signal() -> &'static str {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
        tokio::select! {
            Ok(()) = tokio::signal::ctrl_c() => "Ctrl+C",
            _ = term.recv() => "SIGTERM",
        }
    }
    #[cfg(not(unix))]
    {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
        "Ctrl+C"
    }
}

/// Run the TUI dashboard. The server bootstraps immediately as a sibling
/// task; the TUI just renders state. Hot-reload (`c`) is debug-only - the
/// key is hidden and the channel ignored in release builds.
async fn run_with_tui(args: Args, secrets: Secrets, log_buf: LogBuffer) -> Result<()> {
    let (handles, sinks) = tui::make_channels();
    let tui::TuiHandles {
        stats_tx,
        trigger_rx,
    } = handles;

    let guards = ConnectionGuards::new();
    let tui_guards = guards.clone();
    let (shutdown_tx, shutdown_rx) = unbounded_channel::<()>();

    // Bootstrap right away in the background - the TUI shows "loading" until
    // the first tick lands, then "RUNNING".
    let mut bootstrap_task = tokio::spawn(async move {
        if let Err(e) = bootstrap(args, secrets, stats_tx, trigger_rx, shutdown_rx, guards).await {
            error!("server bootstrap failed: {e:#}");
        }
    });

    // Foreground: render loop. Returns when the user presses `q`.
    let result = tui::run(log_buf, sinks, tui_guards).await;

    // Shut down like a reboot: every player is logged out and saved, and the
    // tick loop exits the process once the saves are flushed. Nothing to
    // save if the world never started.
    if ENGINE_RUNNING.load(Ordering::Acquire) {
        let _ = shutdown_tx.send(());
        println!("Saving every player and shutting down... (Ctrl+C to force)");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("Forced exit -- players since their last save are not saved");
            }
            _ = &mut bootstrap_task => {}
        }
    }

    bootstrap_task.abort();
    shutdown_sidecar();
    result
}

/// Set once the tick loop is running, i.e. once there can be players to save.
static ENGINE_RUNNING: AtomicBool = AtomicBool::new(false);

/// Run the full server startup sequence and accept-loop. Used by both the
/// headless (`--no-tui`) and TUI paths.
async fn bootstrap(
    args: Args,
    secrets: Secrets,
    stats_tx: Sender<TickStats>,
    trigger_rx: UnboundedReceiver<()>,
    shutdown_rx: UnboundedReceiver<()>,
    guards: ConnectionGuards,
) -> Result<()> {
    let host = args.host.clone();
    let http = args.http_port.unwrap_or(8070 + args.node_id as u16);
    let tcp = args.tcp_port.unwrap_or(43584 + args.node_id as u16);
    let jaggrab = args.jaggrab_port.unwrap_or(tcp + 1);

    info!("RuneScape Private Server (Rev {}) starting", REVISION);
    info!("Host: {}", host);
    info!("Node ID: {}", args.node_id);
    info!("HTTP Port: {}", http);
    info!("TCP Port: {}", tcp);
    info!("RSA: {:?}", args.private_key);

    // Subscribed before the engine starts, so the metrics see every tick.
    let metrics_stats = stats_tx.subscribe();
    let admin_stats = stats_tx.subscribe();

    info!("Compiling content assets & building cache...");
    let (store, scripts) = rs_pack::pack_all(
        Path::new(rs_pack::CONTENT_DIR),
        Path::new(rs_pack::PACK_DIR),
        args.verify,
        args.members,
    )?;
    let cache_ptr_val = Box::into_raw(store) as usize;
    let cache: &'static CacheStore = unsafe { &*(cache_ptr_val as *const CacheStore) };

    #[cfg(since_244)]
    for (archive, files) in cache.ondemand.iter().enumerate() {
        for (file, blob) in files.iter().enumerate() {
            if blob.len() > u16::MAX as usize {
                warn!(
                    "ondemand {}/{} is {} bytes, larger than the u16 wire size field; clients will see it as missing",
                    archive,
                    file,
                    blob.len()
                );
            }
        }
    }

    let rsa_path = Path::new(&args.private_key);
    let rsa: &'static RsaKey = Box::leak(Box::new(load_rsa_key(rsa_path)?));

    let (ether_tx, ether_rx) = {
        let (outbound_tx, outbound_rx) = unbounded_channel::<EtherOutbound>();
        let (inbound_tx, inbound_rx) = unbounded_channel::<EtherInbound>();
        let ether_port = args.ether_port.unwrap_or(5000 + args.node_id as u16);
        let node_id = args.node_id;

        let node_name = format!("world{}@{}", node_id, args.node_host);
        let ether = EtherEnv::new(&args, db_config(&args, secrets.db_pass.clone()))?;
        if ether.release.is_none() {
            if !cfg!(debug_assertions) {
                warn!("Running ether from source with mix; use --ether-release in production");
            }
            prepare_ether_sidecar(&ether);
        }

        tokio::spawn(supervise_ether_sidecar(
            ether,
            node_id,
            ether_port,
            node_name,
            secrets.cookie,
        ));

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(ether_client_task(
            ether_port,
            node_id,
            outbound_rx,
            inbound_tx,
            ready_tx,
        ));
        let _ = ready_rx.await;
        (Some(outbound_tx), inbound_rx)
    };

    let db_settings = db_config(&args, secrets.db_pass);
    let (db_tx, db_rx) = {
        let (req_tx, req_rx) = unbounded_channel::<rs_engine::DbRequest>();
        let (resp_tx, resp_rx) = unbounded_channel::<rs_engine::DbResponse>();
        let config = rs_engine::DbClientConfig {
            db: db_settings.clone(),
            node_id: args.node_id,
            pepper: secrets.pepper.clone(),
            auto_register: args.auto_register,
            auto_migrate: args.auto_migrate,
        };
        tokio::spawn(rs_engine::db_client_task(config, req_rx, resp_tx));
        (Some(req_tx), resp_rx)
    };

    // Session recordings, for the staff console's replays.
    let (replay_tx, replay_task) = if args.replay_days > 0 {
        let (tx, rx) = tokio::sync::mpsc::channel(rs_engine::watch::QUEUE);
        let task = tokio::spawn(rs_engine::replay_task(
            db_settings.clone(),
            args.node_id,
            REVISION,
            args.replay_days,
            rx,
        ));
        (Some(tx), Some(task))
    } else {
        (None, None)
    };

    let (new_player_tx, new_player_rx) = unbounded_channel();
    let (reload_tx, reload_rx) = unbounded_channel();
    let (reload_world_tx, reload_world_rx) = unbounded_channel();

    let (mut engine, clock_rate_rx) = Engine::new(
        args.members,
        args.multi_xp,
        args.client_pathfinder,
        new_player_rx,
        scripts,
        cache,
        cache_ptr_val as *mut CacheStore,
        stats_tx,
        reload_world_tx,
        args.node_id,
        ether_tx,
        ether_rx,
        db_tx,
        db_rx,
    );
    engine.replay_tx = replay_tx;
    if let Some(isolation) = args.staff_isolation {
        engine.staff_isolation = isolation;
    }
    info!(
        "Staff isolation: {}",
        if engine.staff_isolation { "on" } else { "off" }
    );

    release_freed_memory();
    trim_working_set();

    let (admin_tx, admin_rx) = unbounded_channel();
    tokio::spawn(engine_tick(
        engine,
        reload_rx,
        clock_rate_rx,
        shutdown_rx,
        admin_rx,
        replay_task,
    ));

    let admin_port = args.admin_port.unwrap_or(8270 + args.node_id as u16);
    if admin_port != 0 {
        tokio::spawn(admin::serve(
            args.admin_host.clone(),
            admin_port,
            secrets.admin,
            admin_tx,
            args.node_id,
            args.members,
            db_settings,
            secrets.pepper,
            admin_stats,
            guards.game.clone(),
        ));
    }

    #[cfg(debug_assertions)]
    tokio::spawn(reload_coordinator(
        args.verify,
        args.members,
        trigger_rx,
        reload_tx,
        reload_world_rx,
    ));
    #[cfg(not(debug_assertions))]
    drop(trigger_rx);

    let server_state = ServerIO {
        cache,
        rsa,
        new_player_tx,
    };

    let metrics_port = args.metrics_port.unwrap_or(8170 + args.node_id as u16);
    if metrics_port != 0 {
        tokio::spawn(metrics::serve(
            args.metrics_host.clone(),
            metrics_port,
            args.members,
            metrics_stats,
            guards.clone(),
        ));
    }

    let ConnectionGuards {
        game: guard,
        http: http_guard,
        jaggrab: jaggrab_guard,
    } = guards;

    info!("Accepting HTTP connections on: {}:{}", host, http);
    info!("Webclient available at: http://localhost:{}/rs2.cgi", http);
    if !cache.static_assets.contains_key("/mapview.js") {
        warn!("public/mapview.js is missing: the world map and the console's live map won't load");
    }
    if args.jaggrab_only {
        info!("--jaggrab-only: cache downloads over HTTP will 404 to force the JAGGRAB fallback");
    }
    tokio::spawn(http::serve(
        host.to_string(),
        http,
        args.node_id.to_string(),
        args.members,
        server_state.clone(),
        guard.clone(),
        http_guard,
        args.jaggrab_only,
        args.real_ip_header
            .clone()
            .map(|header| &*Box::leak(header.into_boxed_str())),
        args.site_origin
            .clone()
            .map(|origin| &*Box::leak(origin.into_boxed_str())),
    ));

    info!("Accepting JAGGRAB connections on: {}:{}", host, jaggrab);
    tokio::spawn(jaggrab::serve(
        host.to_string(),
        jaggrab,
        cache,
        jaggrab_guard,
    ));

    info!("Accepting TCP connections on: {}:{}", host, tcp);
    socket::serve(host, tcp, server_state, guard).await
}

/// Fetches ether's dependencies before running it from source. Its schema is
/// migrated by the node itself on boot when --auto-migrate is on.
fn prepare_ether_sidecar(ether: &EtherEnv) {
    let mut steps = vec![["deps.get"]];
    if ether.auto_migrate {
        steps.push(["ecto.create"]);
    }
    for step in steps {
        let status = ether
            .mix(&step)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => warn!("mix {} exited with {}", step.join(" "), s),
            Err(e) => warn!("mix {} failed: {}", step.join(" "), e),
        }
    }
}

/// `rs-server migrate`: the deploy step that brings the shared database up to
/// date before new servers start, so running servers never migrate it under
/// each other.
async fn migrate(args: &Args) -> Result<()> {
    let db_config = db_config(args, db_pass(args)?);
    let db = rs_db::Db::connect(&db_config, 1)
        .await
        .with_context(|| format!("connecting to {}:{}", db_config.host, db_config.port))?;
    db.migrate().await.context("game migrations failed")?;
    db.close().await;
    info!("Game migrations applied");

    let ether = EtherEnv::new(args, db_config)?;
    if ether.release.is_none() {
        prepare_ether_sidecar(&ether);
    }
    let status = ether
        .migrate()
        .status()
        .context("starting ether migrations")?;
    if !status.success() {
        bail!("ether migrations exited with {status}");
    }
    info!("Ether migrations applied");
    Ok(())
}

async fn supervise_ether_sidecar(
    ether: EtherEnv,
    node_id: u8,
    ether_port: u16,
    node_name: String,
    cookie: String,
) {
    use std::process::Stdio;
    use tokio::time::sleep;
    use tracing::warn;

    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    loop {
        let result = ether
            .run(node_id, ether_port, &node_name, &cookie)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match result {
            Ok(mut child) => {
                let pid = child.id();
                SIDECAR_PID.store(pid, Ordering::Relaxed);
                backoff = Duration::from_secs(30);

                if let Some(stdout) = child.stdout.take() {
                    std::thread::spawn(move || {
                        use std::io::BufRead;
                        let reader = std::io::BufReader::new(stdout);
                        for line in reader.lines().map_while(Result::ok) {
                            debug!(target: "ether", "{}", line);
                        }
                    });
                }

                if let Some(stderr) = child.stderr.take() {
                    std::thread::spawn(move || {
                        use std::io::BufRead;
                        let reader = std::io::BufReader::new(stderr);
                        for line in reader.lines().map_while(Result::ok) {
                            if line.contains("erroneous line, SKIPPED") {
                                continue;
                            }
                            debug!(target: "ether", "{}", line);
                        }
                    });
                }

                let status = tokio::task::spawn_blocking(move || child.wait()).await;
                SIDECAR_PID.store(0, Ordering::Relaxed);

                match status {
                    Ok(Ok(s)) if s.success() => {
                        info!("Ether sidecar exited cleanly");
                        return;
                    }
                    Ok(Ok(s)) => {
                        warn!(
                            "Ether sidecar exited with {}, restarting in {:?}",
                            s, backoff
                        );
                    }
                    Ok(Err(e)) => {
                        warn!(
                            "Ether sidecar wait error: {}, restarting in {:?}",
                            e, backoff
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Ether sidecar task error: {}, restarting in {:?}",
                            e, backoff
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to start Ether sidecar: {}, retrying in {:?}",
                    e, backoff
                );
            }
        }

        sleep(backoff).await;
        backoff = (backoff * 2).min(max_backoff);
    }
}

fn shutdown_sidecar() {
    let pid = SIDECAR_PID.swap(0, Ordering::Relaxed);
    if pid != 0 {
        info!("Shutting down Ether sidecar (pid {})", pid);
        kill_ether_sidecar(pid);
    }
}

fn kill_ether_sidecar(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

/// Hot-reload coordinator - watches the `content/` directory for changes and
/// also accepts manual trigger signals (TUI 'c' key). Runs pack_all on a
/// blocking thread and sends the result to the world tick for instant swap.
#[cfg(debug_assertions)]
async fn reload_coordinator(
    verify: bool,
    members: bool,
    mut trigger_rx: UnboundedReceiver<()>,
    result_tx: UnboundedSender<(Box<CacheStore>, ScriptProvider)>,
    mut reload_world_rx: UnboundedReceiver<()>,
) {
    use notify::{RecursiveMode, Watcher};

    let (watch_tx, mut watch_rx) = unbounded_channel::<()>();

    // File watcher on a dedicated thread (notify uses blocking I/O).
    let _watcher_thread = std::thread::spawn(move || {
        let debounce_tx = watch_tx;
        let (notify_tx, notify_rx) = std::sync::mpsc::channel();

        let mut watcher = match notify::recommended_watcher(notify_tx) {
            Ok(w) => w,
            Err(e) => {
                warn!("File watcher failed to start: {e}");
                return;
            }
        };

        let content = Path::new(rs_pack::CONTENT_DIR);
        if let Ok(entries) = std::fs::read_dir(content) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.file_name().is_some_and(|n| n != "pack") {
                    let _ = watcher.watch(&path, RecursiveMode::Recursive);
                }
            }
        }

        info!(
            "File watcher active on {} (excluding pack/)",
            content.display()
        );

        while notify_rx.recv().is_ok() {
            std::thread::sleep(Duration::from_millis(300));
            while notify_rx.try_recv().is_ok() {}
            let _ = debounce_tx.send(());
        }
    });

    loop {
        tokio::select! {
            Some(_) = trigger_rx.recv() => {}
            Some(_) = watch_rx.recv() => {}
            Some(_) = reload_world_rx.recv() => {}
            else => break,
        }

        info!("Hot-reload: repacking content sources...");
        let start = std::time::Instant::now();

        let result = tokio::task::spawn_blocking(move || {
            rs_pack::pack_all(
                Path::new(rs_pack::CONTENT_DIR),
                Path::new(rs_pack::PACK_DIR),
                verify,
                members,
            )
        })
        .await;

        // Drain any triggers that queued up while packing.
        while trigger_rx.try_recv().is_ok() {}
        while watch_rx.try_recv().is_ok() {}

        match result {
            Ok(Ok((store, scripts))) => {
                let elapsed = start.elapsed();
                info!(
                    "Hot-reload: pack complete in {:.2}ms, sending to world tick",
                    elapsed.as_secs_f64() * 1000.0
                );
                let _ = result_tx.send((store, scripts));
            }
            Ok(Err(e)) => error!("Hot-reload pack failed: {e:#}"),
            Err(e) => error!("Hot-reload task panicked: {e}"),
        }
    }
}

/// Starts a reboot-style shutdown on request (Ctrl+C, SIGTERM, the TUI's `q`): every
/// player is logged out and saved, then the tick loop exits the process.
fn begin_shutdown(engine: &mut Engine) {
    if engine.is_pending_shutdown() || engine.is_shutdown() {
        return;
    }
    info!("Shutdown requested -- logging every player out with a save");
    let ptr = &raw mut *engine;
    rs_engine::with_engine(engine, || unsafe { &mut *ptr }.reboot_timer(0));
}

/// Runs an admin API request on the engine, between ticks.
fn run_admin_job(engine: &mut Engine, job: admin::EngineJob) {
    let ptr = &raw mut *engine;
    rs_engine::with_engine(engine, || job(unsafe { &mut *ptr }));
}

/// World tick task - runs Engine::cycle() every 600ms.
///
/// Also wakes immediately when a hot-reload result arrives to swap the
/// CacheStore + scripts between ticks, or when a shutdown is requested.
/// Lets the recording task store what it was sent, for a few seconds at
/// most (a fatal stop can leave recordings open, and the task waiting).
async fn finish_recordings(task: Option<tokio::task::JoinHandle<()>>) {
    if let Some(task) = task {
        info!("Storing session recordings...");
        let _ = time::timeout(Duration::from_secs(10), task).await;
    }
}

async fn engine_tick(
    mut engine: Engine,
    mut reload_rx: UnboundedReceiver<(Box<CacheStore>, ScriptProvider)>,
    mut clock_rate_rx: Receiver<u64>,
    mut shutdown_rx: UnboundedReceiver<()>,
    mut admin_rx: UnboundedReceiver<admin::EngineJob>,
    mut replay_task: Option<tokio::task::JoinHandle<()>>,
) {
    ENGINE_RUNNING.store(true, Ordering::Release);
    let mut clock_ms: u64 = 600;
    let mut interval = time::interval(Duration::from_millis(clock_ms));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        if clock_ms == 0 {
            tokio::task::yield_now().await;
            if shutdown_rx.try_recv().is_ok() {
                begin_shutdown(&mut engine);
            }
            while let Ok(job) = admin_rx.try_recv() {
                run_admin_job(&mut engine, job);
            }
            while let Ok((store, scripts)) = reload_rx.try_recv() {
                let ptr = &raw mut engine;
                rs_engine::with_engine(&mut engine, || {
                    unsafe { &mut *ptr }.reload_assets(store, scripts);
                });
                release_freed_memory();
            }
            if clock_rate_rx.has_changed().unwrap_or(false) {
                clock_ms = *clock_rate_rx.borrow_and_update();
                if clock_ms != 0 {
                    interval = time::interval(Duration::from_millis(clock_ms));
                    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
                    info!("Engine clock rate changed to {}ms", clock_ms);
                }
            }
        } else {
            // A tick that's due goes first, ahead of any admin jobs queued.
            tokio::select! {
                biased;
                _ = interval.tick() => {}
                Some(()) = shutdown_rx.recv() => {
                    begin_shutdown(&mut engine);
                    continue;
                }
                Some(job) = admin_rx.recv() => {
                    run_admin_job(&mut engine, job);
                    continue;
                }
                Some((store, scripts)) = reload_rx.recv() => {
                    let ptr = &raw mut engine;
                    rs_engine::with_engine(&mut engine, || {
                        unsafe { &mut *ptr }.reload_assets(store, scripts);
                    });
                    release_freed_memory();
                    continue;
                }
                Ok(()) = clock_rate_rx.changed() => {
                    clock_ms = *clock_rate_rx.borrow_and_update();
                    if clock_ms != 0 {
                        interval = time::interval(Duration::from_millis(clock_ms));
                        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
                    }
                    info!("Engine clock rate changed to {}ms", clock_ms);
                    continue;
                }
            }
        }

        match engine.cycle() {
            CycleResult::Continue => {}
            CycleResult::Fatal => {
                error!("Engine shutting down due to fatal phase panic");
                engine.ether_tx = None;
                engine.db_tx = None;
                engine.replay_tx = None;
                info!("Waiting for database saves to complete...");
                while engine.db_rx.recv().await.is_some() {}
                finish_recordings(replay_task.take()).await;
                info!("All saves flushed -- shutting down");
                graceful_exit(1);
            }
            CycleResult::Shutdown => {
                info!("Reboot complete -- taking the world offline");
                engine.ether_tx = None;
                engine.db_tx = None;
                engine.replay_tx = None;
                info!("Waiting for database saves to complete...");
                while engine.db_rx.recv().await.is_some() {}
                finish_recordings(replay_task.take()).await;
                if admin::STAY_DOWN.load(std::sync::atomic::Ordering::Relaxed) {
                    info!("All saves flushed -- server offline until started again");
                    graceful_exit(admin::STAY_DOWN_EXIT);
                }
                info!("All saves flushed -- server offline");
                graceful_exit(0);
            }
        }
    }
}

pub struct Socket {
    socket_type: SocketType,
    pub addr: SocketAddr,
    pub server_io: ServerIO,
    pub guard: ConnectionGuard,
    /// What the client said about its device (the web client's browser).
    pub device: LoginDevice,
}

enum SocketType {
    Tcp(TcpStream),
    // without box, the enum becomes 328 bytes
    // so we reduce the size of WebSocket to 8 bytes
    // at the cost of an extra heap allocation on the socket.
    WebSocket(Box<WebSocketStream<TcpStream>>),
}

impl Socket {
    pub fn from_tcp(
        stream: TcpStream,
        addr: SocketAddr,
        server_io: ServerIO,
        guard: ConnectionGuard,
    ) -> Self {
        Self {
            socket_type: SocketType::Tcp(stream),
            addr,
            server_io,
            guard,
            device: LoginDevice::default(),
        }
    }

    pub fn from_ws(
        stream: WebSocketStream<TcpStream>,
        addr: SocketAddr,
        server_state: ServerIO,
        guard: ConnectionGuard,
    ) -> Self {
        Self {
            socket_type: SocketType::WebSocket(Box::new(stream)),
            addr,
            server_io: server_state,
            guard,
            device: LoginDevice::default(),
        }
    }

    pub async fn read(&mut self) -> Result<Option<Vec<u8>>> {
        match &mut self.socket_type {
            SocketType::Tcp(stream) => {
                let mut buf = vec![0u8; 512];
                let n = stream.read(&mut buf).await?;
                if n == 0 {
                    Ok(None)
                } else {
                    buf.truncate(n);
                    Ok(Some(buf))
                }
            }
            SocketType::WebSocket(ws) => match ws.next().await {
                Some(Ok(Message::Binary(data))) => Ok(Some(data.into())),
                Some(Ok(Message::Close(_))) | None => Ok(None),
                Some(Err(e)) => Err(e.into()),
                _ => Ok(Some(vec![])),
            },
        }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        match &mut self.socket_type {
            SocketType::Tcp(stream) => {
                stream.write_all(data).await?;
            }
            SocketType::WebSocket(ws) => {
                ws.send(Message::Binary(Bytes::copy_from_slice(data)))
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn write_owned(&mut self, data: Vec<u8>) -> Result<Option<Vec<u8>>> {
        match &mut self.socket_type {
            SocketType::Tcp(stream) => {
                stream.write_all(&data).await?;
                Ok(Some(data))
            }
            SocketType::WebSocket(ws) => {
                ws.send(Message::Binary(Bytes::from(data))).await?;
                Ok(None)
            }
        }
    }

    pub async fn flush(&mut self) -> Result<()> {
        match &mut self.socket_type {
            SocketType::Tcp(stream) => {
                stream.flush().await?;
            }
            SocketType::WebSocket(ws) => {
                ws.flush().await?;
            }
        }
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        match &mut self.socket_type {
            SocketType::Tcp(stream) => {
                stream.shutdown().await?;
            }
            SocketType::WebSocket(ws) => {
                // Named explicitly: plain `ws.close()` resolves to either this or
                // `SinkExt::close` on the Box, depending on which futures-sink
                // features the rest of the build happens to enable.
                WebSocketStream::close(ws, None).await?;
            }
        }
        Ok(())
    }
}
