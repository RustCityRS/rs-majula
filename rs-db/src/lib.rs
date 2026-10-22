mod appeal;
mod economy;
mod hiscores;
mod history;
mod investigate;
mod links;
mod logs;
mod moderation;
mod name;
mod news;
mod password;
mod player;
mod recovery;
mod replay;
mod save;
mod script_error;
mod session;
mod snapshot;
mod staff;
mod world;

pub use appeal::{AppealFiled, AppealLogin, AppealRecord, NewAppeal};
pub use economy::{
    GraphAccount, ItemHolder, ItemSource, ItemTotal, TopReceiver, TraceStep, TradeEdge, TradeGraph,
};
pub use hiscores::{
    HISCORE_CATEGORIES, HiscoreEntry, HiscoreRank, HiscoreStanding, KeptOff, PlayerHiscores,
    StandingRow,
};
pub use history::{SavePoint, SaveProgress};
pub use investigate::{AccountActivity, AnticheatLeader};
pub use links::{BannedLink, CROWDED_ADDRESS, ChainLink, EvasionRecord, StaffAccount};
pub use logs::{ChatLog, LogBatch, ScriptErrorLog, SessionLog, WealthEvent, log_type, transfer};
pub use moderation::ModAction;
pub use name::{NameError, PlayerName, validate_name};
pub use news::{NewsAdminPost, NewsCategory, NewsPost, NewsSummary};
pub use password::{
    MAX_PASSWORD_CHARS, MIN_PASSWORD_CHARS, PasswordError, Passwords, check_new_password,
    is_password_char, is_typeable_password,
};
pub use player::{Credentials, NewPlayer, Sanctions};
pub use recovery::{RESET_LOCKED, ResetCode};
pub use replay::{
    NewReplay, PlayedSession, ReplayChunk, ReplayRecord, ReplayReport, ReplayStorage,
};
pub use save::{PlayerSave, SaveResult};
pub use script_error::{ScriptErrorGroup, ScriptErrorRecord};
pub use session::{Claim, LoginDevice, Reconciled, SessionOwner};
pub use snapshot::NewSnapshot;
pub use sqlx::Error;
pub use staff::{
    ChatRecord, DeviceRecord, LinkedAccount, LoginAddress, ModerationRecord, PlayerRecord,
    ReportRecord, SanctionRecord, TIMELINE_KINDS, TimelineCursor, TimelineRow, WealthRecord,
};
pub use world::WorldStatus;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};

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

impl DbConfig {
    fn options(&self) -> PgConnectOptions {
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
                ("statement_timeout", "20s"),
                ("idle_in_transaction_session_timeout", "30s"),
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
            .connect_with(config.options())
            .await?;
        Ok(Self { pool })
    }

    /// Creates a pool that connects on first use, so a caller can start while
    /// the database is still down.
    pub fn connect_lazy(config: &DbConfig, max_connections: u32) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect_lazy_with(config.options());
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
