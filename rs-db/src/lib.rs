/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// A save point's columns, in `save_history` and read from a live save
/// (where `account_details` has the chat filters and `player_saves` the rest).
macro_rules! columns {
    () => {
        "x, z, y, body, colors, gender, runenergy, playtime, stats, levels, \
         afk_zones, last_afk_zone, public_chat, private_chat, trade_chat, \
         last_date, varp_ids, varp_values, inv_types, inv_slots, inv_objs, inv_counts, \
         inv_vars, exit_x, exit_z, exit_y"
    };
}

/// A live save: `player_saves s` joined to its account's `account_details d`.
macro_rules! live_save {
    () => {
        " FROM player_saves s JOIN account_details d ON d.player_id = s.player_id"
    };
}

/// A live save as [`PlayerSave`] reads it.
macro_rules! select_live_save {
    () => {
        concat!(
            "SELECT s.*, d.public_chat, d.private_chat, d.trade_chat",
            live_save!()
        )
    };
}

/// Whether heartbeat column `column` is under `secs` seconds old: its world
/// or lobby is live.
macro_rules! live {
    ($column:literal, $secs:literal) => {
        concat!($column, " >= now() - make_interval(secs => ", $secs, ")")
    };
}

mod account;
mod appeal;
mod device;
mod economy;
mod hiscores;
mod history;
mod investigate;
mod links;
mod lobby;
mod logs;
mod messages;
mod moderation;
mod multilog;
mod name;
mod news;
mod password;
mod player;
mod recovery;
mod recovery_request;
mod replay;
mod save;
mod script_error;
mod session;
mod snapshot;
mod staff;
mod world;

pub use account::{Account, AccountDeleted, AccountLogin};
pub use appeal::{AppealFiled, AppealLogin, AppealRecord, NewAppeal};
pub use device::{DeviceCheck, software_graphics};
pub use economy::{
    GraphAccount, ItemHolder, ItemSource, ItemTotal, TopReceiver, TraceStep, TradeEdge, TradeGraph,
};
pub use hiscores::{
    HISCORE_CATEGORIES, HiscoreEntry, HiscoreRank, HiscoreStanding, KeptOff, PlayerHiscores,
    StandingRow,
};
pub use history::{SavePoint, SaveProgress};
pub use investigate::{AccountActivity, AnticheatLeader};
pub use links::{
    BannedLink, CROWDED_ADDRESS, CROWDED_BROWSER, CROWDED_FINGERPRINT, ChainLink, EvasionRecord,
    StaffAccount, link_rank,
};
pub use lobby::{AccountDetails, LobbyWorld, WorldEndpoint, lobby_id, lobby_node};
pub use logs::{
    ChatLog, LogBatch, ScriptErrorLog, SessionLog, WealthEvent, chat_channel, log_type, transfer,
};
pub use messages::{PlayerMessage, StaffMessageRecord};
pub use moderation::ModAction;
pub use multilog::{MultilogRecord, TransferRecord, Turns};
pub use name::{NameError, PlayerName, validate_name};
pub use news::{NewsAdminPost, NewsCategory, NewsPost, NewsSummary};
pub use password::{
    MAX_PASSWORD_CHARS, MIN_PASSWORD_CHARS, PasswordError, Passwords, check_new_password,
    is_password_char, is_typeable_password,
};
pub use player::{Credentials, NewPlayer, Sanctions, millis};
pub use recovery::{CODE_ALPHABET, RESET_LOCKED, ResetCode, code_from_bits};
pub use recovery_request::{
    NewRecovery, RECOVERIES_PER_ADDRESS, RECOVERIES_WAITING, RecoveryAnswers, RecoveryDecided,
    RecoveryFacts, RecoveryFiled, RecoveryRecord,
};
pub use replay::{
    ACTIVITY_STEP_MS, NewReplay, PlayedSession, ReplayChunk, ReplayRecord, ReplayReport,
    ReplayStorage, ReplayStore,
};
pub use save::{PlayerSave, SaveResult};
pub use script_error::{ScriptErrorGroup, ScriptErrorRecord};
pub use session::{
    Claim, LobbyVisit, LoginAdmission, LoginAttempt, LoginDevice, Reconciled, SessionOwner, refusal,
};
pub use snapshot::NewSnapshot;
pub use sqlx::Error;
pub use staff::{
    ChatRecord, DeviceRecord, LinkedAccount, LoginAddress, MAX_OTHERS, ModerationRecord,
    PlayerRecord, ReportRecord, SanctionRecord, TIMELINE_KINDS, TimelineCursor, TimelineRow,
    WealthRecord,
};
pub use world::WorldStatus;

use sqlx::PgConnection;
use sqlx::migrate::{MigrateError, Migrator};
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::time::Duration;

/// The classes of the advisory locks [`xact_lock`] takes: per display name
/// while one is given out, per address while a login claims its session, and
/// per address while it files a recovery request.
pub(crate) const NAME_LOCK: i32 = 0x4E41_4D45;
pub(crate) const ADDRESS_LOCK: i32 = 14567;
pub(crate) const RECOVERY_LOCK: i32 = 0x5245_4356;

/// Holds `key` in lock class `class` against every other holder until the
/// transaction on `conn` ends.
pub(crate) async fn xact_lock(conn: &mut PgConnection, class: i32, key: &str) -> Result<(), Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2))")
        .bind(class)
        .bind(key)
        .execute(conn)
        .await?;
    Ok(())
}

/// The embedded schema migrations. Public so other crates' `#[sqlx::test]`s
/// can build their throwaway databases from it.
pub static MIGRATOR: Migrator = sqlx::migrate!();

/// Connection settings for the game database.
#[derive(Clone, Debug)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    pub pass: String,
}

/// How long a statement may run before Postgres cancels it.
const STATEMENT_TIMEOUT: Duration = Duration::from_secs(20);

impl DbConfig {
    fn options(&self, statement_timeout: Duration) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .database(&self.name)
            .username(&self.user)
            .password(&self.pass)
            // A hung query errors out instead of stalling its caller, and a
            // client that stalls mid-transaction is cut off so it can't hold
            // a player's row locked.
            .options([
                (
                    "statement_timeout",
                    format!("{}ms", statement_timeout.as_millis()),
                ),
                ("idle_in_transaction_session_timeout", "30s".to_owned()),
                // Every query here is small: compiling one costs far more
                // (hundreds of ms for a hiscores rank) than it ever saves.
                ("jit", "off".to_owned()),
            ])
    }
}

/// A secret from its flag or environment variable. Debug builds fall back to
/// `dev_default`; release builds refuse to start without a real one.
pub fn require_secret(
    value: Option<String>,
    env: &str,
    dev_default: &str,
) -> Result<String, String> {
    match value {
        Some(value) => Ok(value),
        None if cfg!(debug_assertions) => Ok(dev_default.to_owned()),
        None => Err(format!("{env} must be set in release builds")),
    }
}

/// A handle to the game database. Cheap to clone (it wraps a connection pool).
#[derive(Clone, Debug)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    /// Opens a connection pool and verifies the database is reachable.
    pub async fn connect(config: &DbConfig, max_connections: u32) -> Result<Self, Error> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect_with(config.options(STATEMENT_TIMEOUT))
            .await?;
        Ok(Self { pool })
    }

    /// Creates a pool that connects on first use, so a caller can start while
    /// the database is still down.
    pub fn connect_lazy(config: &DbConfig, max_connections: u32) -> Self {
        Self::connect_lazy_with_timeout(config, max_connections, STATEMENT_TIMEOUT)
    }

    /// [`Db::connect_lazy`], with statements cancelled after
    /// `statement_timeout` instead.
    pub fn connect_lazy_with_timeout(
        config: &DbConfig,
        max_connections: u32,
        statement_timeout: Duration,
    ) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect_lazy_with(config.options(statement_timeout));
        Self { pool }
    }

    /// Wraps an existing pool (used by tests).
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Applies any pending migrations. Safe to call from several processes at
    /// once: sqlx serialises migrators with a Postgres advisory lock.
    pub async fn migrate(&self) -> Result<(), MigrateError> {
        MIGRATOR.run(&self.pool).await
    }

    /// The versions of this build's migrations the database hasn't applied
    /// yet, without changing anything. Migrations from a newer build are
    /// ignored, so an older server keeps running through a rolling deploy.
    pub async fn pending_migrations(&self) -> Result<Vec<i64>, MigrateError> {
        let exists: bool = sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
            .fetch_one(&self.pool)
            .await?;
        let applied: Vec<(i64, Vec<u8>, bool)> = if exists {
            sqlx::query_as("SELECT version, checksum, success FROM _sqlx_migrations")
                .fetch_all(&self.pool)
                .await?
        } else {
            Vec::new()
        };

        let mut pending = Vec::new();
        for migration in MIGRATOR.iter() {
            if migration.migration_type.is_down_migration() {
                continue;
            }
            match applied.iter().find(|(v, ..)| *v == migration.version) {
                Some((_, _, false)) => return Err(MigrateError::Dirty(migration.version)),
                Some((_, checksum, _)) if *checksum != *migration.checksum => {
                    return Err(MigrateError::VersionMismatch(migration.version));
                }
                Some(_) => {}
                None => pending.push(migration.version),
            }
        }
        Ok(pending)
    }

    /// Closes every pooled connection.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
