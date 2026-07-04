extern crate core;

use futures_util::StreamExt;
use std::io::IsTerminal;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
mod http;
mod jaggrab;
mod socket;
pub mod tui;

use crate::tui::log_layer::{LogBuffer, TuiLogLayer};
use anyhow::Result;
use clap::Parser;
use futures_util::SinkExt;
use mpsc::{UnboundedReceiver, unbounded_channel};
use rs_crypto::rsa::{RsaKey, load_rsa_key};
use rs_engine::{CycleResult, Engine};
use rs_engine::{EtherInbound, EtherOutbound, ether_client_task};
use rs_engine::{LoginRequest, TickStats};
use rs_pack::cache::CacheStore;
use rs_pack::cache::script::ScriptProvider;
use std::collections::HashMap;
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
use tracing::{Level, debug, error, info};
use tracing_subscriber::Layer;
use tracing_subscriber::util::SubscriberInitExt;
use watch::{Receiver, Sender, channel};

use crossterm::execute;
use crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
use std::sync::atomic::{AtomicU32, Ordering};
use time::MissedTickBehavior;

static SIDECAR_PID: AtomicU32 = AtomicU32::new(0);

#[cfg(debug_assertions)]
const MAX_CONNECTIONS_PER_IP: usize = 4;
#[cfg(not(debug_assertions))]
const MAX_CONNECTIONS_PER_IP: usize = 2;

#[derive(Clone)]
pub struct ConnectionGuard {
    ip_counts: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

impl ConnectionGuard {
    fn new() -> Self {
        Self {
            ip_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn try_acquire(&self, ip: IpAddr) -> Option<ConnectionPermit> {
        let mut counts = self.ip_counts.lock().unwrap();
        let count = counts.entry(ip).or_insert(0);
        if *count >= MAX_CONNECTIONS_PER_IP {
            return None;
        }
        *count += 1;
        Some(ConnectionPermit {
            ip,
            ip_counts: self.ip_counts.clone(),
        })
    }
}

pub struct ConnectionPermit {
    ip: IpAddr,
    ip_counts: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        let mut counts = self.ip_counts.lock().unwrap();
        let count = counts.get_mut(&self.ip).unwrap();
        *count -= 1;
        if *count == 0 {
            counts.remove(&self.ip);
        }
    }
}

struct ShutdownGuard;

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        restore_terminal_and_sidecar();
    }
}

fn restore_terminal_and_sidecar() {
    shutdown_sidecar();
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
}

fn graceful_exit(code: i32) -> ! {
    restore_terminal_and_sidecar();
    std::process::exit(code);
}

struct DbEnv {
    host: String,
    port: u16,
    name: String,
    user: String,
    pass: String,
    cluster: String,
}

/// Server revision, taken from the `REV` build env var (see `.cargo/config.toml`).
pub const REVISION: &str = env!("REV");

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "rs-server")]
#[command(about = "RuneScape Private Server (Rev 225)")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    /// HTTP port. Defaults to 8070 + node_id (8080 for node 10).
    #[arg(long)]
    http_port: Option<u16>,
    /// TCP game port. Defaults to 43584 + node_id (43594 for node 10).
    #[arg(long)]
    tcp_port: Option<u16>,
    /// JAGGRAB port for the standalone desktop client's cache bootstrap.
    /// Defaults to tcp_port + 1 (43595 for node 10). The web client doesn't use it.
    #[arg(long)]
    jaggrab_port: Option<u16>,
    #[arg(long, default_value = ".keys/private.pem")]
    private_key: PathBuf,
    #[arg(long, default_value = "true")]
    members: bool,
    #[arg(long, default_value = "1")]
    multi_xp: u8,
    #[arg(long, default_value = "true")]
    client_pathfinder: bool,
    /// Disable the TUI dashboard and run with classic stdout logging.
    #[arg(long, default_value = "false")]
    no_tui: bool,
    #[arg(long, default_value = "true")]
    verify: bool,
    /// World node ID (10 = world 1, 11 = world 2, etc.)
    #[arg(long, default_value = "10")]
    node_id: u8,
    /// Ether sidecar TCP port. Defaults to 5000 + node_id.
    #[arg(long)]
    ether_port: Option<u16>,
    /// Postgres hostname.
    #[arg(long, default_value = "localhost")]
    db_host: String,
    /// Postgres port.
    #[arg(long, default_value = "5432")]
    db_port: u16,
    /// Postgres database name.
    #[arg(long, default_value = "postgres")]
    db_name: String,
    /// Postgres username.
    #[arg(long, default_value = "postgres")]
    db_user: String,
    /// Postgres password.
    #[arg(long, default_value = "password")]
    db_pass: String,
    /// Comma-separated list of cluster node names (e.g. "world10@127.0.0.1,world11@127.0.0.1").
    #[arg(long, default_value = "")]
    cluster: String,
    /// Server-side pepper for password hashing.
    #[arg(long, default_value = "localhost")]
    pepper: String,
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
    // silence the chatty save + protocol logs and tokio-postgres'
    // schema-migration NOTICEs that would otherwise flood at startup.
    let make_filter = || {
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(
                "info,\
                     rs_engine::player_save=warn,\
                     rs_protocol=warn,\
                     tokio_postgres=warn,\
                     runec=error",
            )
        })
    };

    // File appender - `rs-server.log` in the current working directory,
    // overwritten each run (no rotation; log volume per session is bounded).
    // Held in `_log_guard` for the lifetime of `main` so the background
    // writer thread keeps draining until shutdown.
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("rs-server.log")?;
    let (file_writer, _log_guard) = tracing_appender::non_blocking(log_file);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_filter(make_filter());

    let registry = tracing_subscriber::registry().with(file_layer);

    // The TUI requires a real terminal - auto-fall-back to headless when
    // stdout is e.g. piped, redirected, or running under IntelliJ's Run
    // tool window (which doesn't interpret cursor/raw-mode escapes).
    let tty = std::io::stdout().is_terminal();
    let use_tui = !args.no_tui && tty;

    if !use_tui {
        let stdout_layer = tracing_subscriber::fmt::layer()
            .without_time()
            .with_filter(make_filter());
        registry.with(stdout_layer).init();
        let _ = Level::INFO;

        if !args.no_tui && !tty {
            info!(
                "stdout is not a TTY - falling back to headless mode (use a real terminal for the TUI dashboard)"
            );
        }
        info!("RuneScape Private Server (Rev 225) starting (headless)");

        let (stats_tx, _stats_rx) = channel(TickStats::default());
        let (_trigger_tx, trigger_rx) = unbounded_channel::<()>();
        bootstrap(args, stats_tx, trigger_rx).await
    } else {
        let log_buf = tui::log_layer::new_buffer();
        let tui_layer = TuiLogLayer::new(log_buf.clone()).with_filter(make_filter());
        registry.with(tui_layer).init();

        run_with_tui(args, log_buf).await
    }
}

/// Run the TUI dashboard. The server bootstraps immediately as a sibling
/// task; the TUI just renders state. Hot-reload (`c`) is debug-only - the
/// key is hidden and the channel ignored in release builds.
async fn run_with_tui(args: Args, log_buf: LogBuffer) -> Result<()> {
    let (handles, sinks) = tui::make_channels();
    let tui::TuiHandles {
        stats_tx,
        trigger_rx,
    } = handles;

    // Bootstrap right away in the background - the TUI shows "loading" until
    // the first tick lands, then "RUNNING".
    let bootstrap_task = tokio::spawn(async move {
        if let Err(e) = bootstrap(args, stats_tx, trigger_rx).await {
            error!("server bootstrap failed: {e:#}");
        }
    });

    // Foreground: render loop. Returns when the user presses `q`.
    let result = tui::run(log_buf, sinks).await;

    bootstrap_task.abort();
    shutdown_sidecar();
    result
}

/// Run the full server startup sequence and accept-loop. Used by both the
/// headless (`--no-tui`) and TUI paths.
async fn bootstrap(
    args: Args,
    stats_tx: Sender<TickStats>,
    trigger_rx: UnboundedReceiver<()>,
) -> Result<()> {
    let host = args.host;
    let http = args.http_port.unwrap_or(8070 + args.node_id as u16);
    let tcp = args.tcp_port.unwrap_or(43584 + args.node_id as u16);
    let jaggrab = args.jaggrab_port.unwrap_or(tcp + 1);

    info!("RuneScape Private Server (Rev {}) starting", REVISION);
    info!("Host: {}", host);
    info!("Node ID: {}", args.node_id);
    info!("HTTP Port: {}", http);
    info!("TCP Port: {}", tcp);
    info!("RSA: {:?}", args.private_key);

    info!("Compiling content assets & building cache...");
    let (store, scripts) = rs_pack::pack_all(
        Path::new(rs_pack::CONTENT_DIR),
        Path::new(rs_pack::PACK_DIR),
        args.verify,
        args.members,
    )?;
    let cache_ptr_val = Box::into_raw(store) as usize;
    let cache: &'static CacheStore = unsafe { &*(cache_ptr_val as *const CacheStore) };

    let rsa_path = Path::new(&args.private_key);
    let rsa: &'static RsaKey = Box::leak(Box::new(load_rsa_key(rsa_path)?));

    let (ether_tx, ether_rx) = {
        let (outbound_tx, outbound_rx) = unbounded_channel::<EtherOutbound>();
        let (inbound_tx, inbound_rx) = unbounded_channel::<EtherInbound>();
        let ether_port = args.ether_port.unwrap_or(5000 + args.node_id as u16);
        let node_id = args.node_id;

        let node_name = format!("world{}@127.0.0.1", node_id);
        let db_env = DbEnv {
            host: args.db_host.clone(),
            port: args.db_port,
            name: args.db_name.clone(),
            user: args.db_user.clone(),
            pass: args.db_pass.clone(),
            cluster: args.cluster.clone(),
        };

        prepare_ether_sidecar(&db_env);

        tokio::spawn(supervise_ether_sidecar(
            node_id, ether_port, node_name, db_env,
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

    let (db_tx, db_rx) = {
        let (req_tx, req_rx) = unbounded_channel::<rs_engine::DbRequest>();
        let (resp_tx, resp_rx) = unbounded_channel::<rs_engine::DbResponse>();
        tokio::spawn(rs_engine::db_client_task(
            args.db_host.clone(),
            args.db_port,
            args.db_name.clone(),
            args.db_user.clone(),
            args.db_pass.clone(),
            args.pepper.clone(),
            req_rx,
            resp_tx,
        ));
        (Some(req_tx), resp_rx)
    };

    let (new_player_tx, new_player_rx) = unbounded_channel();
    let (reload_tx, reload_rx) = unbounded_channel();
    let (reload_world_tx, reload_world_rx) = unbounded_channel();

    let (engine, clock_rate_rx) = Engine::new(
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
    tokio::spawn(engine_tick(engine, reload_rx, clock_rate_rx));

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

    let guard = ConnectionGuard::new();

    info!("Accepting HTTP connections on: {}:{}", host, http);
    info!("Webclient available at: http://localhost:{}/rs2.cgi", http);
    tokio::spawn(http::serve(
        host.to_string(),
        http,
        args.node_id.to_string(),
        (args.node_id - 10).to_string(),
        args.members,
        server_state.clone(),
        guard.clone(),
    ));

    info!("Accepting JAGGRAB connections on: {}:{}", host, jaggrab);
    tokio::spawn(jaggrab::serve(host.to_string(), jaggrab, cache));

    info!("Accepting TCP connections on: {}:{}", host, tcp);
    socket::serve(host, tcp, server_state, guard).await
}

fn prepare_ether_sidecar(db: &DbEnv) {
    let run = |args: &[&str]| {
        let status = Command::new("cmd")
            .args(["/c", "mix"])
            .args(args)
            .current_dir("rs-ether")
            .env("RS_DB_HOST", &db.host)
            .env("RS_DB_PORT", db.port.to_string())
            .env("RS_DB_NAME", &db.name)
            .env("RS_DB_USER", &db.user)
            .env("RS_DB_PASS", &db.pass)
            .env("RS_CLUSTER_HOSTS", &db.cluster)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => true,
            Ok(s) => {
                tracing::warn!("mix {} exited with {}", args.join(" "), s);
                false
            }
            Err(e) => {
                tracing::warn!("mix {} failed: {}", args.join(" "), e);
                false
            }
        }
    };

    run(&["deps.get"]);
    run(&["ecto.create"]);
    run(&["ecto.migrate"]);
}

async fn supervise_ether_sidecar(node_id: u8, ether_port: u16, node_name: String, db: DbEnv) {
    use std::process::Stdio;
    use tokio::time::sleep;
    use tracing::warn;

    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    loop {
        let result = std::process::Command::new("cmd")
            .args([
                "/c",
                "elixir",
                "--name",
                &node_name,
                "--cookie",
                "rs_secret",
                "-S",
                "mix",
                "run",
                "--no-halt",
            ])
            .current_dir("rs-ether")
            .env("RS_NODE_ID", node_id.to_string())
            .env("RS_ETHER_PORT", ether_port.to_string())
            .env("RS_DB_HOST", &db.host)
            .env("RS_DB_PORT", db.port.to_string())
            .env("RS_DB_NAME", &db.name)
            .env("RS_DB_USER", &db.user)
            .env("RS_DB_PASS", &db.pass)
            .env("RS_CLUSTER_HOSTS", &db.cluster)
            .stdin(Stdio::null())
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
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
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
                tracing::warn!("File watcher failed to start: {e}");
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

/// World tick task - runs Engine::cycle() every 600ms.
///
/// Also wakes immediately when a hot-reload result arrives to swap the
/// CacheStore + scripts between ticks.
async fn engine_tick(
    mut engine: Engine,
    mut reload_rx: UnboundedReceiver<(Box<CacheStore>, ScriptProvider)>,
    mut clock_rate_rx: Receiver<u64>,
) {
    let mut clock_ms: u64 = 600;
    let mut interval = time::interval(Duration::from_millis(clock_ms));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        if clock_ms == 0 {
            tokio::task::yield_now().await;
            while let Ok((store, scripts)) = reload_rx.try_recv() {
                let ptr = &raw mut engine;
                rs_engine::with_engine(&mut engine, || {
                    unsafe { &mut *ptr }.reload_assets(store, scripts);
                });
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
            tokio::select! {
                _ = interval.tick() => {}
                Some((store, scripts)) = reload_rx.recv() => {
                    let ptr = &raw mut engine;
                    rs_engine::with_engine(&mut engine, || {
                        unsafe { &mut *ptr }.reload_assets(store, scripts);
                    });
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
                info!("Waiting for database saves to complete...");
                while engine.db_rx.recv().await.is_some() {}
                info!("All saves flushed -- shutting down");
                graceful_exit(1);
            }
            CycleResult::Shutdown => {
                info!("Reboot complete -- taking the world offline");
                engine.ether_tx = None;
                engine.db_tx = None;
                info!("Waiting for database saves to complete...");
                while engine.db_rx.recv().await.is_some() {}
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
                ws.close().await?;
            }
        }
        Ok(())
    }
}
