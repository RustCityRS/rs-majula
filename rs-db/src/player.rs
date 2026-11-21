use crate::{Db, Error, NAME_LOCK, xact_lock};
use rs_util::base37::to_userhash;
use sqlx::PgConnection;
use std::net::IpAddr;
use std::time::Duration;

/// A millisecond count from the database, such as a mute's, as a
/// [`Duration`], clamping negatives to zero.
pub fn millis(ms: i64) -> Duration {
    Duration::from_millis(ms.max(0) as u64)
}

/// What the game login needs to authenticate a player.
#[derive(Debug, sqlx::FromRow)]
pub struct Credentials {
    pub password_hash: String,
    pub staff_mod_level: i16,
    pub banned: bool,
    pub muted_ms: Option<i64>,
    pub left_other_world_ms: Option<i64>,
    pub age_ms: i64,
    pub unread_messages: i64,
    pub last_login_ms: Option<i64>,
    pub last_login_ipv4: Option<i64>,
    pub member: bool,
    pub jcoins: i32,
}

/// A player's current mute and ban, from [`Db::sanctions`].
#[derive(Debug, PartialEq, Eq)]
pub struct Sanctions {
    pub name_hash: u64,
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

/// Whether player `o` holds `$name` (lowercase compared) as their display name,
/// or as a previous display name still held (`display_name_hold_start`).
macro_rules! holds_name {
    ($name:literal) => {
        concat!(
            "(lower(o.display_name) = lower(",
            $name,
            ") OR (lower(o.previous_display_name) = lower(",
            $name,
            ") AND o.display_name_changed_at > display_name_hold_start()))"
        )
    };
}

/// Holds `name`, whatever its case, against every other registration and
/// display name claim until the transaction on `conn` ends, so a check that it
/// is free stays true until the write commits.
async fn lock_name(conn: &mut PgConnection, name: &str) -> Result<(), Error> {
    xact_lock(conn, NAME_LOCK, &name.to_lowercase()).await
}

impl Db {
    /// Looks up a player's credentials by base37 name, for a login to world
    /// `node_id`, with their membership, JCoins and last admitted login
    /// before this one. A world login skips lobby logins: its lobby visit
    /// came just before it. `None` if no such player.
    pub async fn find_credentials(
        &self,
        name_hash: u64,
        node_id: i16,
    ) -> Result<Option<Credentials>, Error> {
        sqlx::query_as(
            concat!("select * from (SELECT players.password_hash, players.staff_mod_level,
                    coalesce(players.banned_until > now(), false) AS banned,
                    CASE WHEN players.muted_until > now()
                         THEN ", ms!("players.muted_until - now()"), "
                    END AS muted_ms,
                    CASE WHEN players.last_logout_node <> $2
                         THEN ", ms!("now() - players.last_logout_at"), "
                    END AS left_other_world_ms,
                    ", ms!("now() - players.created_at"), " AS age_ms,
                    (SELECT count(*) FROM staff_messages m
                     WHERE m.player_id = players.id AND m.read_at IS NULL) AS unread_messages,
                    ", ms!("prev.created_at"), " AS last_login_ms,
                    CASE WHEN family(prev.ip) = 4 THEN prev.ip - '0.0.0.0'::inet END AS last_login_ipv4,
                    coalesce(d.subscribed OR d.members_until > now(), false) AS member,
                    coalesce(d.jcoins, 0) AS jcoins
             FROM players
             LEFT JOIN account_details d ON d.player_id = players.id
             LEFT JOIN LATERAL (SELECT l.created_at, l.ip FROM login_logs l
                                WHERE l.player_id = players.id AND l.refused IS NULL
                                  AND (l.node_id >= 0 OR $2 < 0)
                                ORDER BY l.created_at DESC LIMIT 1) prev ON true
             WHERE players.name_hash = $1) p"),
        )
        .bind(name_hash as i64)
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Makes an account a subscribed member, on its first login to a members
    /// world.
    pub async fn grant_membership(&self, name_hash: u64) -> Result<(), Error> {
        sqlx::query(
            "UPDATE account_details SET subscribed = true
             WHERE player_id = (SELECT id FROM players WHERE name_hash = $1) AND NOT subscribed",
        )
        .bind(name_hash as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Registers a player. Returns `false` if the name is already taken, as a
    /// name or as another player's display name or held previous display name,
    /// or if the display name is another player's name.
    pub async fn create_player(&self, player: &NewPlayer<'_>) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;
        lock_name(&mut tx, player.display_name).await?;
        let result = sqlx::query(concat!(
            "INSERT INTO players
                 (name_hash, display_name, password_hash, staff_mod_level, registration_ip)
             SELECT $1, $2, $3, $4, $5::inet
             WHERE NOT EXISTS (SELECT 1 FROM players o WHERE o.name_hash = $6 OR ",
            holds_name!("$2"),
            ")
             ON CONFLICT (name_hash) DO NOTHING"
        ))
        .bind(player.name_hash as i64)
        .bind(player.display_name)
        .bind(player.password_hash)
        .bind(player.staff_mod_level)
        .bind(player.registration_ip.map(|ip| ip.to_string()))
        .bind(to_userhash(player.display_name) as i64)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
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
        sqlx::query_scalar(concat!(
            "UPDATE players
             SET muted_until = greatest(muted_until, now() + make_interval(hours => $2))
             WHERE name_hash = $1 AND staff_mod_level < $3
             RETURNING ",
            ms!("muted_until - now()")
        ))
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
        let rows: Vec<(i64, Option<i64>, bool)> = sqlx::query_as(concat!(
            "SELECT name_hash,
                    CASE WHEN muted_until > now()
                         THEN ",
            ms!("muted_until - now()"),
            "
                    END,
                    coalesce(banned_until > now(), false)
             FROM players WHERE name_hash = ANY($1)"
        ))
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

    /// Whether `name` is registered, or is another player's display name or
    /// held previous display name.
    pub async fn name_taken(&self, name: &crate::PlayerName) -> Result<bool, Error> {
        sqlx::query_scalar(concat!(
            "SELECT EXISTS (SELECT 1 FROM players o WHERE o.name_hash = $1 OR ",
            holds_name!("$2"),
            ")"
        ))
        .bind(name.hash as i64)
        .bind(&name.display)
        .fetch_one(&self.pool)
        .await
    }

    /// How many players were registered from `ip` in the last `hours`. An
    /// IPv6 address counts with the rest of its /64, which one household or
    /// phone is handed whole.
    pub async fn recent_registrations(&self, ip: IpAddr, hours: i32) -> Result<i64, Error> {
        sqlx::query_scalar(
            "SELECT count(*) FROM players
             WHERE registration_ip <<= network(set_masklen($1::inet,
                     CASE family($1::inet) WHEN 6 THEN 64 ELSE 32 END))
               AND created_at > now() - make_interval(hours => $2)",
        )
        .bind(ip.to_string())
        .bind(hours)
        .fetch_one(&self.pool)
        .await
    }
}

/// The display names a rev-647 player can claim (`p_setdisplayname`): a name
/// is free unless another player has it as their name or display name.
impl Db {
    /// Whether no player but `name_hash` has `name` as their name or display
    /// name.
    pub async fn display_name_free(
        &self,
        name_hash: u64,
        name: &crate::PlayerName,
    ) -> Result<bool, Error> {
        let taken: bool = sqlx::query_scalar(concat!(
            "SELECT EXISTS (SELECT 1 FROM players o
             WHERE o.name_hash <> $1 AND (o.name_hash = $2 OR ",
            holds_name!("$3"),
            "))"
        ))
        .bind(name_hash as i64)
        .bind(name.hash as i64)
        .bind(&name.display)
        .fetch_one(&self.pool)
        .await?;
        Ok(!taken)
    }

    /// Sets the player `name_hash`'s display name to `name` if it is free (see
    /// [`display_name_free`](Self::display_name_free)), in one statement.
    /// Returns whether it took.
    pub async fn claim_display_name(
        &self,
        name_hash: u64,
        name: &crate::PlayerName,
    ) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;
        lock_name(&mut tx, &name.display).await?;
        let result = sqlx::query(concat!(
            "UPDATE players SET
                 previous_display_name = CASE WHEN lower(display_name) = lower($3)
                     THEN previous_display_name ELSE display_name END,
                 display_name_changed_at = CASE WHEN lower(display_name) = lower($3)
                     THEN display_name_changed_at ELSE now() END,
                 display_name = $3
             WHERE name_hash = $1 AND NOT EXISTS (
                 SELECT 1 FROM players o
                 WHERE o.name_hash <> $1 AND (o.name_hash = $2 OR ",
            holds_name!("$3"),
            "))"
        ))
        .bind(name_hash as i64)
        .bind(name.hash as i64)
        .bind(&name.display)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(result.rows_affected() == 1)
    }

    /// The player `name_hash`'s display name, if registered.
    pub async fn display_name(&self, name_hash: u64) -> Result<Option<String>, Error> {
        sqlx::query_scalar("SELECT display_name FROM players WHERE name_hash = $1")
            .bind(name_hash as i64)
            .fetch_optional(&self.pool)
            .await
    }
}
