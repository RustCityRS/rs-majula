mod name;
mod news;
mod password;
mod player;
mod save;

pub use name::{NameError, PlayerName, validate_name};
pub use news::{NewsCategory, NewsPost, NewsSummary};
pub use password::Passwords;
pub use player::{Credentials, NewPlayer};
pub use save::PlayerSave;
pub use sqlx::Error;

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

    /// Closes every pooled connection.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
