use crate::{Db, Error};
use std::net::IpAddr;

/// What the game login needs to authenticate a player.
#[derive(Debug, sqlx::FromRow)]
pub struct Credentials {
    pub password_hash: String,
    pub staff_mod_level: i16,
    pub banned: bool,
    pub muted_ms: Option<i64>,
    pub left_other_world_ms: Option<i64>,
}

/// A player's current mute and ban, from [`Db::sanctions`].
#[derive(Debug, PartialEq, Eq)]
pub struct Sanctions {
    pub name_hash: u64,
    /// Milliseconds left on a chat mute, `None` while not muted.
    pub muted_ms: Option<i64>,
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
    /// Looks up a player's credentials by base37 name, for a login to world
    /// `node_id`. `None` if no such player.
    pub async fn find_credentials(
        &self,
        name_hash: u64,
        node_id: i16,
    ) -> Result<Option<Credentials>, Error> {
        sqlx::query_as(
            "select * from (SELECT password_hash, staff_mod_level,
                    coalesce(banned_until > now(), false) AS banned,
                    CASE WHEN muted_until > now()
                         THEN (extract(epoch FROM muted_until - now()) * 1000)::bigint
                    END AS muted_ms,
                    CASE WHEN last_logout_node <> $2
                         THEN (extract(epoch FROM now() - last_logout_at) * 1000)::bigint
                    END AS left_other_world_ms
             FROM players WHERE name_hash = $1) p",
        )
        .bind(name_hash as i64)
        .bind(node_id)
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

    /// Mutes a player for `hours`, unless they are staff of `by_level` or
    /// above. A longer mute already in place is kept. Returns the milliseconds
    /// left on the mute, or `None` if no such player or they outrank the
    /// moderator.
    pub async fn mute_player(
        &self,
        name_hash: u64,
        hours: i32,
        by_level: i16,
    ) -> Result<Option<i64>, Error> {
        sqlx::query_scalar(
            "UPDATE players
             SET muted_until = greatest(muted_until, now() + make_interval(hours => $2))
             WHERE name_hash = $1 AND staff_mod_level < $3
             RETURNING (extract(epoch FROM muted_until - now()) * 1000)::bigint",
        )
        .bind(name_hash as i64)
        .bind(hours)
        .bind(by_level)
        .fetch_optional(&self.pool)
        .await
    }

    /// Bans a player for `hours`. A longer ban already in place is kept.
    /// Returns `false` if no such player.
    pub async fn ban_player(&self, name_hash: u64, hours: i32) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE players
             SET banned_until = greatest(banned_until, now() + make_interval(hours => $2))
             WHERE name_hash = $1",
        )
        .bind(name_hash as i64)
        .bind(hours)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// The current mute and ban of each of `names` that is registered.
    pub async fn sanctions(&self, names: &[u64]) -> Result<Vec<Sanctions>, Error> {
        let names: Vec<i64> = names.iter().map(|&h| h as i64).collect();
        let rows: Vec<(i64, Option<i64>, bool)> = sqlx::query_as(
            "SELECT name_hash,
                    CASE WHEN muted_until > now()
                         THEN (extract(epoch FROM muted_until - now()) * 1000)::bigint
                    END,
                    coalesce(banned_until > now(), false)
             FROM players WHERE name_hash = ANY($1)",
        )
        .bind(&names)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(h, muted_ms, banned)| Sanctions {
                name_hash: h as u64,
                muted_ms,
                banned,
            })
            .collect())
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
