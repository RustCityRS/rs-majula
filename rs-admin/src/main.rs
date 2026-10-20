mod activity;
mod api;
mod appeals;
mod audit;
mod canned;
mod cases;
mod clips;
mod controls;
mod economy;
mod farms;
mod http;
mod markdown;
mod notes;
mod prefs;
mod schedule;
mod sessions;
mod store;
mod templates;
mod totp;
mod users;
mod worlds;

use crate::audit::Audit;
use crate::canned::CannedTexts;
use crate::cases::Cases;
use crate::clips::Clips;
use crate::controls::Controls;
use crate::http::{App, HttpConfig};
use crate::notes::Notes;
use crate::prefs::Prefs;
use crate::schedule::Schedule;
use crate::sessions::Sessions;
use crate::templates::Templates;
use crate::users::{Role, User, Users, hash_password, now_ms, password_problem};
use crate::worlds::Worlds;
use anyhow::{Context, Result, bail};
use axum::http::HeaderName;
use clap::{Parser, Subcommand, ValueEnum};
use std::io::BufRead;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(name = "rs-admin", about = "The staff console")]
struct Args {
    #[arg(long, env = "RS_ADMIN_DATA", default_value = "rs-admin/data")]
    data_dir: PathBuf,
    #[command(subcommand)]
    command: Option<Cmd>,
    #[command(flatten)]
    serve: ServeArgs,
}

#[derive(clap::Args, Debug)]
struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value = "8150")]
    port: u16,
    #[arg(long, default_value = "rs-admin/worlds.json")]
    worlds: PathBuf,
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    https: bool,
    #[arg(long, env = "RS_ADMIN_ORIGIN")]
    origin: Option<String>,
    #[arg(long)]
    real_ip_header: Option<HeaderName>,
    #[arg(long, env = "RS_ADMIN_SECRET", hide_env_values = true)]
    admin_secret: Option<String>,
    #[arg(long, env = "RS_ADMIN_WEB_URL")]
    web_url: Option<String>,
    #[arg(long, value_enum, default_value_t = LogFormat::Text)]
    log_format: LogFormat,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LogFormat {
    Text,
    Json,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    #[command(subcommand)]
    User(UserCmd),
}

#[derive(Subcommand, Debug)]
enum UserCmd {
    List,
    Add {
        name: String,
        #[arg(long, value_enum)]
        role: Role,
        #[arg(long)]
        password_stdin: bool,
    },
    Passwd {
        name: String,
        #[arg(long)]
        password_stdin: bool,
    },
    Role {
        name: String,
        #[arg(value_enum)]
        role: Role,
    },
    #[command(name = "reset-2fa")]
    Reset2fa {
        name: String,
    },
    Disable {
        name: String,
    },
    Enable {
        name: String,
    },
    Remove {
        name: String,
    },
}

/// A secret from its flag or environment variable. Debug builds fall back to
/// `dev_default`; release builds refuse to start without a real one.
fn require_secret(value: Option<String>, env: &str, dev_default: &str) -> Result<String> {
    match value {
        Some(value) => Ok(value),
        None if cfg!(debug_assertions) => Ok(dev_default.to_owned()),
        None => bail!("{env} must be set in release builds"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let users = Users::new(args.data_dir.join("users.json"));
    match args.command {
        Some(Cmd::User(cmd)) => user_command(&users, cmd, &args.serve).await,
        None => serve(args.serve, users, args.data_dir).await,
    }
}

fn read_password(name: &str, from_stdin: bool) -> Result<String> {
    let password = if from_stdin {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        line.trim_end_matches(['\r', '\n']).to_owned()
    } else {
        let first = rpassword::prompt_password(format!("Password for {name}: "))?;
        let again = rpassword::prompt_password("Again: ")?;
        if first != again {
            bail!("the passwords didn't match");
        }
        first
    };
    if let Some(problem) = password_problem(&password, name) {
        bail!("{problem}");
    }
    Ok(password)
}

/// Console accounts are only for game staff: asks the worlds for `name`'s
/// game account and returns its name as the game spells it.
async fn game_staff_name(name: &str, args: &ServeArgs) -> Result<String> {
    let secret = require_secret(args.admin_secret.clone(), "RS_ADMIN_SECRET", "rs_admin")?;
    let world_list =
        worlds::load(&args.worlds).with_context(|| format!("loading {}", args.worlds.display()))?;
    let lookup = rs_admin_api::PlayerLookup {
        name: name.to_owned(),
    };
    let record = Worlds::new(world_list, None, &secret)
        .ask_any(rs_admin_api::path::PLAYER, &lookup)
        .await
        .map_err(|e| anyhow::anyhow!("couldn't look {name} up in the game: {e}"))?;
    Ok(Worlds::game_staff(record, name)
        .map_err(anyhow::Error::msg)?
        .name)
}

async fn user_command(users: &Users, cmd: UserCmd, args: &ServeArgs) -> Result<()> {
    match cmd {
        UserCmd::List => {
            let all = users.all()?;
            if all.is_empty() {
                println!("No accounts in {}.", users.path().display());
            }
            for user in all {
                println!(
                    "{:<14} {:<10} 2fa:{:<4} {}",
                    user.name,
                    format!("{:?}", user.role).to_lowercase(),
                    if user.totp_secret.is_some() {
                        "on"
                    } else {
                        "off"
                    },
                    if user.disabled { "disabled" } else { "" }
                );
            }
        }
        UserCmd::Add {
            name,
            role,
            password_stdin,
        } => {
            let name = name.trim().to_owned();
            if name.is_empty() || name.chars().count() > 32 {
                bail!("names are 1 to 32 characters");
            }
            let name = game_staff_name(&name, args).await?;
            if users.find(&name)?.is_some() {
                bail!("an account named {name} already exists");
            }
            let password = read_password(&name, password_stdin)?;
            users.add(User {
                name: name.clone(),
                role,
                password_hash: hash_password(&password)?,
                totp_secret: None,
                disabled: false,
                created_ms: now_ms(),
                signed_out_ms: 0,
            })?;
            println!("Added {name}. They set up their authenticator at their first sign-in.");
        }
        UserCmd::Passwd {
            name,
            password_stdin,
        } => {
            users.find(&name)?.context("no such account")?;
            let hash = hash_password(&read_password(&name, password_stdin)?)?;
            users.update(&name, |u| {
                u.password_hash = hash;
                u.signed_out_ms = now_ms();
            })?;
            println!("Changed {name}'s password and signed them out.");
        }
        UserCmd::Role { name, role } => {
            users.update(&name, |u| {
                u.role = role;
                u.signed_out_ms = now_ms();
            })?;
            println!("{name} is now {}.", format!("{role:?}").to_lowercase());
        }
        UserCmd::Reset2fa { name } => {
            users.update(&name, |u| {
                u.totp_secret = None;
                u.signed_out_ms = now_ms();
            })?;
            println!(
                "Cleared {name}'s authenticator; they set up a new one at their next sign-in."
            );
        }
        UserCmd::Disable { name } => {
            users.update(&name, |u| {
                u.disabled = true;
                u.signed_out_ms = now_ms();
            })?;
            println!("Disabled {name}.");
        }
        UserCmd::Enable { name } => {
            users.update(&name, |u| u.disabled = false)?;
            println!("Enabled {name}.");
        }
        UserCmd::Remove { name } => {
            users.remove(&name)?;
            println!("Removed {name}.");
        }
    }
    Ok(())
}

async fn serve(args: ServeArgs, users: Users, data_dir: PathBuf) -> Result<()> {
    let logs = tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
    );
    match args.log_format {
        LogFormat::Text => logs.init(),
        LogFormat::Json => logs.json().flatten_event(true).init(),
    }

    let secret = require_secret(args.admin_secret, "RS_ADMIN_SECRET", "rs_admin")?;
    let world_list =
        worlds::load(&args.worlds).with_context(|| format!("loading {}", args.worlds.display()))?;
    let accounts = users.all()?;
    if !accounts.iter().any(|u| u.role.can_command() && !u.disabled) {
        warn!(
            "No admin account yet; add one with `rs-admin user add <name> --role admin` (accounts: {})",
            users.path().display()
        );
    }

    let web_url = args
        .web_url
        .or_else(|| cfg!(debug_assertions).then(|| "http://127.0.0.1:8190".to_owned()));
    if web_url.is_none() {
        warn!("No --web-url, so news can't be posted from the console");
    }
    let csp = http::csp(&world_list);
    let worlds = Arc::new(Worlds::new(world_list, web_url, &secret));
    let app = Arc::new(App {
        users,
        sessions: Sessions::default(),
        worlds: worlds.clone(),
        audit: Audit::open(data_dir.join("audit.jsonl")),
        cases: Cases::open(data_dir.join("cases.jsonl")),
        controls: Controls::open(data_dir.join("controls.json")),
        notes: Notes::open(data_dir.join("notes.jsonl")),
        schedule: Schedule::open(data_dir.join("schedule.json")),
        templates: Templates::open(
            data_dir.join("templates.json"),
            data_dir.join("offences.json"),
        ),
        canned: CannedTexts::open(data_dir.join("canned.json")),
        clips: Clips::new(data_dir.join("clips")),
        prefs: Prefs::open(data_dir.join("prefs.json")),
        economy: economy::Economy::open(&data_dir),
        staff_levels: Default::default(),
        address_salt: sessions::random_token(),
        media_dir: data_dir.join("media"),
        config: HttpConfig {
            https: args.https,
            origin: args.origin.map(|o| o.trim_end_matches('/').to_owned()),
            real_ip_header: args.real_ip_header,
            csp,
        },
        dummy_hash: hash_password("not any account's password")?,
    });

    tokio::spawn(worlds::poll(worlds));
    tokio::spawn(controls::keep_in_sync(app.clone()));
    tokio::spawn(schedule::keep_running(app.clone()));
    tokio::spawn(economy::keep_sampling(app.clone()));
    if app.worlds.has_web() {
        tokio::spawn(appeals::keep_taking_in(app.clone()));
    }
    let pruner = app.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            pruner.sessions.prune();
        }
    });

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .context("parsing --host/--port")?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    if !addr.ip().is_loopback() && !args.https {
        warn!("Serving on {addr} without --https; keep it on a private network");
    }
    info!(
        "Console available at: {}://{}",
        if args.https { "https" } else { "http" },
        addr
    );
    axum::serve(
        listener,
        http::router(app).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
        info!("Shutting down");
    })
    .await?;
    Ok(())
}
