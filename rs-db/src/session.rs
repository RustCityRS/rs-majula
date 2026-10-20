use crate::save::PlayerSave;
use crate::{Db, Error};
use std::net::IpAddr;

/// The outcome of [`Db::claim_session`].
#[derive(Debug, PartialEq)]
pub enum Claim {
    Claimed {
        save: Option<Box<PlayerSave>>,
        took_over: Option<i16>,
    },
    Busy {
        node: i16,
    },
    Unregistered,
}

/// Who owns a player's session, from [`Db::session_owner`].
#[derive(Debug, PartialEq, sqlx::FromRow)]
pub struct SessionOwner {
    pub node: i16,
    pub since_ms: Option<i64>,
    /// How long ago the owning world's heartbeat was, `None` if it has none.
    pub heartbeat_age_ms: Option<i64>,
}

/// What [`Db::reconcile_sessions`] corrected.
#[derive(Debug, Default, PartialEq)]
pub struct Reconciled {
    pub released: Vec<u64>,
    pub lost: Vec<u64>,
}

impl Db {
    /// Claims a player's session for world `node_id` and loads their save,
    /// atomically: the save returned is the one the claim protects.
    ///
    /// The claim is refused while another world owns the session, unless
    /// that world's heartbeat is older than `takeover_after_secs` (it is dead,
    /// and its saves will be refused from now on). Claiming a session this
    /// world already owns succeeds.
    pub async fn claim_session(
        &self,
        name_hash: u64,
        node_id: i16,
        takeover_after_secs: i32,
    ) -> Result<Claim, Error> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(i64, Option<i16>, bool)> = sqlx::query_as(
            "SELECT p.id, p.online_node,
                    coalesce(w.updated_at < now() - make_interval(secs => $2), true)
             FROM players p
             LEFT JOIN world_status w ON w.node_id = p.online_node
             WHERE p.name_hash = $1
             FOR UPDATE OF p",
        )
        .bind(name_hash as i64)
        .bind(takeover_after_secs)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((id, owner, owner_dead)) = row else {
            return Ok(Claim::Unregistered);
        };
        let took_over = owner.filter(|&owner| owner != node_id);
        if let Some(node) = took_over
            && !owner_dead
        {
            return Ok(Claim::Busy { node });
        }

        sqlx::query("UPDATE players SET online_node = $2, online_since = now() WHERE id = $1")
            .bind(id)
            .bind(node_id)
            .execute(&mut *tx)
            .await?;
        let save: Option<PlayerSave> =
            sqlx::query_as("SELECT * FROM player_saves WHERE player_id = $1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok(Claim::Claimed {
            save: save.map(Box::new),
            took_over,
        })
    }

    /// Releases world `node_id`'s claim on a player without a logout (a login
    /// that claimed but never entered the world). Returns whether it held one.
    pub async fn release_session(&self, name_hash: u64, node_id: i16) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE players SET online_node = NULL, online_since = NULL
             WHERE name_hash = $1 AND online_node = $2",
        )
        .bind(name_hash as i64)
        .bind(node_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// The world that owns the player's session, if one does.
    pub async fn session_owner(&self, name_hash: u64) -> Result<Option<SessionOwner>, Error> {
        sqlx::query_as(
            "SELECT p.online_node AS node,
                    (extract(epoch FROM p.online_since) * 1000)::bigint AS since_ms,
                    (extract(epoch FROM now() - w.updated_at) * 1000)::bigint AS heartbeat_age_ms
             FROM players p
             LEFT JOIN world_status w ON w.node_id = p.online_node
             WHERE p.name_hash = $1 AND p.online_node IS NOT NULL",
        )
        .bind(name_hash as i64)
        .fetch_optional(&self.pool)
        .await
    }

    /// Releases a player's session from a world that has stopped: one whose
    /// heartbeat is older than `takeover_after_secs`, as a login elsewhere
    /// would take it over. Returns the world it was released from, `None`
    /// if nothing was (no claim, or its world is alive).
    pub async fn release_dead_session(
        &self,
        name_hash: u64,
        takeover_after_secs: i32,
    ) -> Result<Option<i16>, Error> {
        sqlx::query_scalar(
            "WITH old AS (
                 SELECT id, online_node FROM players WHERE name_hash = $1 FOR UPDATE
             )
             UPDATE players p SET online_node = NULL, online_since = NULL
             FROM old
             WHERE p.id = old.id AND old.online_node IS NOT NULL
               AND NOT EXISTS (
                   SELECT 1 FROM world_status w
                   WHERE w.node_id = old.online_node
                     AND w.updated_at >= now() - make_interval(secs => $2))
             RETURNING old.online_node",
        )
        .bind(name_hash as i64)
        .bind(takeover_after_secs)
        .fetch_optional(&self.pool)
        .await
    }

    /// Ends every session world `node_id` owns, as a logout, except those in
    /// `except` (players whose newest data hasn't reached the database yet).
    /// Called when the world starts, for what a previous run left claimed,
    /// and when it shuts down cleanly.
    pub async fn release_node_sessions(&self, node_id: i16, except: &[u64]) -> Result<u64, Error> {
        let except: Vec<i64> = except.iter().map(|&h| h as i64).collect();
        let result = sqlx::query(
            "UPDATE players SET online_node = NULL, online_since = NULL,
                                last_logout_node = $1, last_logout_at = now()
             WHERE online_node = $1 AND name_hash <> ALL($2)",
        )
        .bind(node_id)
        .bind(&except)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Squares world `node_id`'s claims with what it actually has: releases
    /// claims on players it neither has `online` nor `held` (mid-login or
    /// mid-logout), and reports `online` players it no longer owns.
    pub async fn reconcile_sessions(
        &self,
        node_id: i16,
        online: &[u64],
        held: &[u64],
    ) -> Result<Reconciled, Error> {
        let online: Vec<i64> = online.iter().map(|&h| h as i64).collect();
        let keep: Vec<i64> = online
            .iter()
            .copied()
            .chain(held.iter().map(|&h| h as i64))
            .collect();

        let mut tx = self.pool.begin().await?;
        let released: Vec<i64> = sqlx::query_scalar(
            "UPDATE players SET online_node = NULL, online_since = NULL
             WHERE online_node = $1 AND name_hash <> ALL($2)
             RETURNING name_hash",
        )
        .bind(node_id)
        .bind(&keep)
        .fetch_all(&mut *tx)
        .await?;
        let lost: Vec<i64> = sqlx::query_scalar(
            "SELECT name_hash FROM players
             WHERE name_hash = ANY($2) AND online_node IS DISTINCT FROM $1",
        )
        .bind(node_id)
        .bind(&online)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(Reconciled {
            released: released.into_iter().map(|h| h as u64).collect(),
            lost: lost.into_iter().map(|h| h as u64).collect(),
        })
    }

    /// Counts a wrong-password login from `ip`.
    pub async fn record_login_failure(&self, ip: IpAddr) -> Result<(), Error> {
        sqlx::query("INSERT INTO login_failures (ip) VALUES ($1::inet)")
            .bind(ip.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Wrong-password logins from `ip` in the last `secs`, across all worlds.
    pub async fn recent_login_failures(&self, ip: IpAddr, secs: i32) -> Result<i64, Error> {
        sqlx::query_scalar(
            "SELECT count(*) FROM login_failures
             WHERE ip = $1::inet AND at > now() - make_interval(secs => $2)",
        )
        .bind(ip.to_string())
        .bind(secs)
        .fetch_one(&self.pool)
        .await
    }

    /// Logs that `name_hash` authenticated on world `node_id` from `ip`.
    pub async fn record_login(
        &self,
        name_hash: u64,
        node_id: i16,
        ip: IpAddr,
    ) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO login_logs (player_id, node_id, ip)
             SELECT id, $2, $3::inet FROM players WHERE name_hash = $1",
        )
        .bind(name_hash as i64)
        .bind(node_id)
        .bind(ip.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Forgets logins older than `days`.
    pub async fn prune_login_logs(&self, days: i32) -> Result<(), Error> {
        sqlx::query("DELETE FROM login_logs WHERE created_at < now() - make_interval(days => $1)")
            .bind(days)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Forgets failures older than `secs`.
    pub async fn prune_login_failures(&self, secs: i32) -> Result<(), Error> {
        sqlx::query("DELETE FROM login_failures WHERE at < now() - make_interval(secs => $1)")
            .bind(secs)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
