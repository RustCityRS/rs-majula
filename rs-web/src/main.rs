mod app;
mod routes;
mod templates;
mod worlds;

use crate::app::{App, SiteConfig};
use crate::templates::Templates;
use anyhow::{Context, Result};
use axum::http::HeaderName;
use clap::Parser;
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
    #[arg(long, default_value = "localhost")]
    db_host: String,
    #[arg(long, default_value = "5432")]
    db_port: u16,
    #[arg(long, default_value = "postgres")]
    db_name: String,
    #[arg(long, default_value = "postgres")]
    db_user: String,
    #[arg(long, default_value = "password")]
    db_pass: String,
    #[arg(long, default_value = "localhost")]
    pepper: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();
    let args = Args::parse();

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

    let db = Db::connect_lazy(
        &DbConfig {
            host: args.db_host,
            port: args.db_port,
            name: args.db_name,
            user: args.db_user,
            pass: args.db_pass,
        },
        8,
    );
    // The game server applies the same migrations; the site still serves its
    // static pages if the database isn't up yet.
    if let Err(e) = db.migrate().await {
        warn!("Database migration failed ({e}); database pages will error until it is reachable");
    }

    let app = Arc::new(App {
        db,
        passwords: Passwords::new(&args.pepper),
        templates: Templates::new(args.root.join("templates"), args.https),
        worlds,
        wordenc,
        site: SiteConfig {
            https: args.https,
            public_host: args.public_host,
            port: args.port,
            real_ip_header: args.real_ip_header,
        },
    });

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
    .await?;
    Ok(())
}
