mod password;
mod player;
mod save;

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

/// A handle to the game database. Cheap to clone (it wraps a connection pool).
#[derive(Clone, Debug)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    /// Opens a connection pool and verifies the database is reachable.
    pub async fn connect(config: &DbConfig, max_connections: u32) -> Result<Self, Error> {
        let options = PgConnectOptions::new()
            .host(&config.host)
            .port(config.port)
            .database(&config.name)
            .username(&config.user)
            .password(&config.pass);
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
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
