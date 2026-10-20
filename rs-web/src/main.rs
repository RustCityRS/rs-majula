mod admin;
mod app;
mod metrics;
mod news_render;
mod routes;
mod templates;
mod worlds;

use crate::app::{App, SiteConfig};
use crate::templates::Templates;
use anyhow::{Context, Result, anyhow};
use axum::http::HeaderName;
use clap::{Parser, Subcommand, ValueEnum};
use rs_db::{Db, DbConfig, Passwords};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

/// The game website: registration, news, world select and the archived
/// 2004 pages.
#[derive(Parser, Debug)]
#[command(name = "rs-web")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(long, default_value = "8000")]
    port: u16,
    #[arg(long, default_value = "rs-web")]
    root: PathBuf,
    #[arg(long, default_value = "localhost")]
    public_host: String,
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    https: bool,
    #[arg(long)]
    real_ip_header: Option<HeaderName>,
    /// Port of the Prometheus `/metrics` listener; `0` disables it.
    #[arg(long, default_value = "8160")]
    metrics_port: u16,
    #[arg(long, default_value = "127.0.0.1")]
    metrics_host: String,
    #[arg(long, default_value = "8190")]
    admin_port: u16,
    #[arg(long, default_value = "127.0.0.1")]
    admin_host: String,
    #[arg(long, env = "RS_ADMIN_SECRET", hide_env_values = true)]
    admin_secret: Option<String>,
    #[arg(long, env = "RS_WEB_MEDIA_DIR")]
    media_dir: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = LogFormat::Text)]
    log_format: LogFormat,
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
    #[arg(long, default_value_t = cfg!(debug_assertions), action = clap::ArgAction::Set)]
    auto_migrate: bool,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogFormat {
    Text,
    Json,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Apply pending database migrations and exit.
    Migrate,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let logs = tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,sqlx=warn".into()),
    );
    match args.log_format {
        LogFormat::Text => logs.init(),
        LogFormat::Json => logs.json().flatten_event(true).init(),
    }
    let db_config = DbConfig {
        host: args.db_host,
        port: args.db_port,
        name: args.db_name,
        user: args.db_user,
        pass: rs_db::require_secret(args.db_pass, "RS_DB_PASS", "password")
            .map_err(|e| anyhow!(e))?,
    };

    if let Some(Cmd::Migrate) = args.command {
        let db = Db::connect(&db_config, 1)
            .await
            .with_context(|| format!("connecting to {}:{}", db_config.host, db_config.port))?;
        db.migrate().await.context("migrations failed")?;
        info!("Migrations applied");
        return Ok(());
    }
    let pepper =
        rs_db::require_secret(args.pepper, "RS_PEPPER", "localhost").map_err(|e| anyhow!(e))?;

    let worlds_path = args.root.join("worlds.json");
    let worlds =
        worlds::load(&worlds_path).with_context(|| format!("loading {}", worlds_path.display()))?;

    let wordenc = app::load_wordenc();
    if wordenc.is_none() {
        warn!(
            "No word filter found in {}; usernames won't be filtered",
            rs_pack::CONTENT_DIR
        );
    }

    let db = Db::connect_lazy(&db_config, 8);
    // The site still serves its static pages if the database isn't up yet or
    // is behind; only the pages that read it fail.
    if args.auto_migrate {
        if let Err(e) = db.migrate().await {
            warn!(
                "Database migration failed ({e}); database pages will error until it is reachable"
            );
        }
    } else {
        match db.pending_migrations().await {
            Ok(pending) if pending.is_empty() => {}
            Ok(pending) => {
                warn!("Database is missing migrations {pending:?}; run `rs-web migrate`")
            }
            Err(e) => warn!("Database schema check failed ({e}); database pages may error"),
        }
    }

    let media_dir = args
        .media_dir
        .or_else(|| cfg!(debug_assertions).then(|| args.root.join("media")));
    match &media_dir {
        Some(dir) => info!("News media in: {}", dir.display()),
        None => warn!("No --media-dir, so news can't have images or videos uploaded"),
    }
    let app = Arc::new(App {
        db,
        media_dir,
        passwords: Passwords::new(&pepper),
        templates: Templates::new(args.root.join("templates"), args.https),
        worlds,
        live_worlds: Default::default(),
        wordenc,
        site: SiteConfig {
            https: args.https,
            public_host: args.public_host,
            port: args.port,
            real_ip_header: args.real_ip_header,
        },
        metrics: Default::default(),
    });

    tokio::spawn(worlds::refresh(app.clone()));
    if args.metrics_port != 0 {
        tokio::spawn(metrics::serve(
            args.metrics_host,
            args.metrics_port,
            app.clone(),
        ));
    }

    if args.admin_port != 0 {
        match args
            .admin_secret
            .or_else(|| cfg!(debug_assertions).then(|| "rs_admin".to_owned()))
        {
            Some(secret) => {
                tokio::spawn(admin::serve(
                    args.admin_host,
                    args.admin_port,
                    secret,
                    app.clone(),
                ));
            }
            None => warn!("RS_ADMIN_SECRET isn't set, so the admin API (news posting) is off"),
        }
    }

    let url = app.base_url();
    let router = routes::router(app, &args.root.join("public"));
    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .context("parsing --host/--port")?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    info!("Accepting HTTP connections on: {}", addr);
    info!("Website available at: {}", url);
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let signal = shutdown_signal().await;
        info!("{signal} received; finishing open requests");
    })
    .await?;
    info!("Website stopped");
    Ok(())
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
