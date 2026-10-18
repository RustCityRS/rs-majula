use crate::{Db, Error};
use std::net::IpAddr;

/// What the game login needs to authenticate a player.
#[derive(Debug, sqlx::FromRow)]
pub struct Credentials {
    pub password_hash: String,
    pub staff_mod_level: i16,
    pub banned: bool,
}

/// A player to register.
#[derive(Debug)]
pub struct NewPlayer<'a> {
    pub name_hash: u64,
    pub display_name: &'a str,
    pub password_hash: &'a str,
    pub staff_mod_level: i16,
    pub registration_ip: Option<IpAddr>,
}

impl Db {
    /// Looks up a player's credentials by base37 name. `None` if no such player.
    pub async fn find_credentials(&self, name_hash: u64) -> Result<Option<Credentials>, Error> {
        sqlx::query_as(
            "SELECT password_hash, staff_mod_level,
                    coalesce(banned_until > now(), false) AS banned
             FROM players WHERE name_hash = $1",
        )
        .bind(name_hash as i64)
        .fetch_optional(&self.pool)
        .await
    }

    /// Registers a player. Returns `false` if the name is already taken.
    pub async fn create_player(&self, player: &NewPlayer<'_>) -> Result<bool, Error> {
        let result = sqlx::query(
            "INSERT INTO players
                 (name_hash, display_name, password_hash, staff_mod_level, registration_ip)
             VALUES ($1, $2, $3, $4, $5::inet)
             ON CONFLICT (name_hash) DO NOTHING",
        )
        .bind(player.name_hash as i64)
        .bind(player.display_name)
        .bind(player.password_hash)
        .bind(player.staff_mod_level)
        .bind(player.registration_ip.map(|ip| ip.to_string()))
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn name_taken(&self, name_hash: u64) -> Result<bool, Error> {
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM players WHERE name_hash = $1)")
            .bind(name_hash as i64)
            .fetch_one(&self.pool)
            .await
    }

    /// How many players were registered from `ip` in the last `hours`.
    pub async fn recent_registrations(&self, ip: IpAddr, hours: i32) -> Result<i64, Error> {
        sqlx::query_scalar(
            "SELECT count(*) FROM players
             WHERE registration_ip = $1::inet
               AND created_at > now() - make_interval(hours => $2)",
        )
        .bind(ip.to_string())
        .bind(hours)
        .fetch_one(&self.pool)
        .await
    }
}
