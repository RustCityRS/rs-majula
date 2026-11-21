use crate::device::DeviceCheck;
use crate::links::{CROWDED_BROWSER, uncrowded};
use crate::save::PlayerSave;
use crate::{ADDRESS_LOCK, Db, Error, xact_lock};
use rs_util::net::limit_address;
use sqlx::{FromRow, query, query_as, query_scalar};
use std::net::IpAddr;

/// The outcome of [`Db::claim_session`].
#[derive(Debug, PartialEq)]
pub enum Claim {
    Claimed {
        save: Option<Box<PlayerSave>>,
        took_over: Option<i16>,
        chat_filters: [i16; 3],
    },
    Busy {
        node: i16,
    },
    Unregistered,
    IpLimit,
    BadSession,
}

#[derive(Copy, Clone, Debug)]
pub struct LoginAdmission {
    pub ip: IpAddr,
    pub max_per_ip: u8,
    pub allow_direct: bool,
    pub visit: Option<LobbyVisit>,
    pub lobby_instance: Option<i64>,
}

#[derive(Copy, Clone, Debug)]
pub struct LobbyVisit {
    pub client_session: i64,
    pub revision: i32,
}

#[derive(FromRow)]
struct StoredSession {
    id: i64,
    owner: Option<i16>,
    handoff: bool,
    target: Option<i16>,
}

/// Who owns a player's session, from [`Db::session_owner`].
#[derive(Debug, PartialEq, FromRow)]
pub struct SessionOwner {
    pub node: i16,
    pub since_ms: Option<i64>,
    /// How long ago the owning world's heartbeat was, `None` if it has none.
    pub heartbeat_age_ms: Option<i64>,
}

/// What a login's client said about the device it ran on (see
/// `0019_login_devices.sql`). The desktop client says nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoginDevice {
    /// The web client, rather than the desktop one.
    pub web: bool,
    pub user_agent: Option<String>,
    /// The browser's platform hint (`Sec-CH-UA-Platform`), e.g. `Windows`.
    pub platform: Option<String>,
    /// The browser's first language, e.g. `en-US`.
    pub language: Option<String>,
    /// Whether the browser said it's a phone or tablet.
    pub touch: bool,
    /// The hash of what the client page measured about the device (see
    /// `0042_multilogging.sql`), if it sent one.
    pub fingerprint: Option<i64>,
    /// What else the page measured: its rendering engine,
    /// `navigator.platform` and graphics card.
    pub engine: Option<String>,
    pub js_platform: Option<String>,
    pub gpu: Option<String>,
    /// How the page's check came back; `None` from a client without one.
    pub check: Option<DeviceCheck>,
    /// The page's `navigator.userAgent` isn't the one the browser sent.
    pub agent_differs: bool,
}

/// Why a login with the right password was turned away (`login_logs.refused`).
pub mod refusal {
    /// Another account was online from the same browser.
    pub const MULTILOG: &str = "multilog";
}

/// What [`Db::reconcile_sessions`] corrected.
#[derive(Debug, Default, PartialEq)]
pub struct Reconciled {
    pub released: Vec<u64>,
    pub lost: Vec<u64>,
}

/// A wrong password counted before it was checked (see
/// [`Db::claim_login_attempt`]).
#[derive(Debug)]
pub struct LoginAttempt {
    ip: IpAddr,
    /// Its row in `login_failures`, which has no id of its own.
    row: String,
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
        self.claim_login(name_hash, node_id, takeover_after_secs, None)
            .await
    }

    /// Claims a player's session for `node_id`, a world or (negative) a
    /// lobby, as [`Db::claim_session`] does, once the login's `admission`
    /// lets it in: a lobby's `lobby_instance` must still hold its live
    /// heartbeat, and the address (an IPv6 one with its /64) may hold at
    /// most `max_per_ip` other live sessions.
    ///
    /// A world login presents the visit of a pending lobby handoff that names
    /// it or no world, and may go without one only when `allow_direct` and no
    /// handoff is pending. A lobby login needs its visit only while a handoff
    /// to a world is pending, and then takes the player back from that world.
    /// A visit matches on its client session, address and revision. The claim
    /// clears the handoff.
    pub async fn claim_login(
        &self,
        name_hash: u64,
        node_id: i16,
        takeover_after_secs: i32,
        admission: Option<LoginAdmission>,
    ) -> Result<Claim, Error> {
        let mut tx = self.pool.begin().await?;
        if let Some(instance) = admission.and_then(|a| a.lobby_instance) {
            let current: Option<i64> = query_scalar(concat!(
                "SELECT lobby_instance FROM world_status WHERE node_id = $1 AND lobby_instance = $2
                 AND ",
                live!("updated_at", "$3"),
                " FOR SHARE"
            ))
            .bind(node_id)
            .bind(instance)
            .bind(takeover_after_secs)
            .fetch_optional(&mut *tx)
            .await?;
            if current.is_none() {
                return Ok(Claim::BadSession);
            }
        }
        let ip = admission.map(|a| limit_address(a.ip).to_string());
        if let Some(ip) = &ip {
            xact_lock(&mut tx, ADDRESS_LOCK, ip).await?;
        }
        let row: Option<StoredSession> = query_as(
            "SELECT id, online_node AS owner, coalesce(lobby_until > now(), false) AS handoff,
                    lobby_target AS target FROM players WHERE name_hash = $1 FOR UPDATE",
        )
        .bind(name_hash as i64)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(StoredSession {
            id,
            owner,
            handoff,
            target,
        }) = row
        else {
            return Ok(Claim::Unregistered);
        };
        let took_over = owner.filter(|&owner| owner != node_id);
        if let Some(node) = took_over {
            let live: bool = query_scalar(concat!(
                "SELECT EXISTS(SELECT 1 FROM world_status WHERE node_id = $1
                AND ",
                live!("updated_at", "$2"),
                ")"
            ))
            .bind(node)
            .bind(takeover_after_secs)
            .fetch_one(&mut *tx)
            .await?;
            if live {
                return Ok(Claim::Busy { node });
            }
        }

        let needs_visit = if node_id < 0 {
            handoff && target.is_some()
        } else {
            handoff || admission.is_some_and(|a| !a.allow_direct)
        };
        if needs_visit && admission.and_then(|a| a.visit).is_none() {
            return Ok(Claim::BadSession);
        }
        if let Some(admission) = admission {
            let count: i64 = query_scalar(concat!(
                "SELECT count(*) FROM players p JOIN world_status w ON w.node_id = p.online_node
                 WHERE p.online_ip = $1::inet AND p.id <> $2
                   AND ",
                live!("w.updated_at", "$3")
            ))
            .bind(&ip)
            .bind(id)
            .bind(takeover_after_secs)
            .fetch_one(&mut *tx)
            .await?;
            if count >= i64::from(admission.max_per_ip) {
                return Ok(Claim::IpLimit);
            }
            if let Some(visit) = admission.visit
                && needs_visit
            {
                let valid: bool = query_scalar(
                    "SELECT coalesce(lobby_session = $2 AND lobby_ip = $3::inet
                       AND lobby_revision = $4 AND lobby_until > now()
                       AND ($5 < 0 OR lobby_target IS NULL OR lobby_target = $5), false)
                     FROM players WHERE id = $1",
                )
                .bind(id)
                .bind(visit.client_session)
                .bind(admission.ip.to_canonical().to_string())
                .bind(visit.revision)
                .bind(node_id)
                .fetch_one(&mut *tx)
                .await?;
                if !valid {
                    return Ok(Claim::BadSession);
                }
            }
        }

        // The session is the login this world just let in: the newest the
        // account made here, moments ago.
        query(
            "UPDATE players SET online_node = $2, online_since = now(), online_ip = $3::inet,
                    lobby_until = NULL, lobby_session = NULL, lobby_ip = NULL,
                    lobby_revision = NULL, lobby_target = NULL,
                    online_login = (SELECT l.id FROM login_logs l
                                    WHERE l.player_id = $1 AND l.node_id = $2 AND l.refused IS NULL
                                      AND l.created_at > now() - interval '5 minutes'
                                    ORDER BY l.id DESC LIMIT 1)
             WHERE id = $1",
        )
        .bind(id)
        .bind(node_id)
        .bind(&ip)
        .execute(&mut *tx)
        .await?;
        let save = if node_id < 0 {
            None
        } else {
            query_as::<_, PlayerSave>(concat!(select_live_save!(), " WHERE s.player_id = $1"))
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
        };
        let chat_filters = match &save {
            Some(save) => [save.public_chat, save.private_chat, save.trade_chat],
            None => {
                let (public, private, trade): (i16, i16, i16) = query_as(
                    "SELECT public_chat, private_chat, trade_chat FROM account_details
                     WHERE player_id = $1",
                )
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
                [public, private, trade]
            }
        };
        tx.commit().await?;
        Ok(Claim::Claimed {
            save: save.map(Box::new),
            took_over,
            chat_filters,
        })
    }

    /// Releases world `node_id`'s claim on a player without a logout (a login
    /// that claimed but never entered the world). Returns whether it held one.
    pub async fn release_session(&self, name_hash: u64, node_id: i16) -> Result<bool, Error> {
        let result = query(
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
        query_as(concat!(
            "SELECT p.online_node AS node,
                    ",
            ms!("p.online_since"),
            " AS since_ms,
                    ",
            ms!("now() - w.updated_at"),
            " AS heartbeat_age_ms
             FROM players p
             LEFT JOIN world_status w ON w.node_id = p.online_node
             WHERE p.name_hash = $1 AND p.online_node IS NOT NULL"
        ))
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
        query_scalar(concat!(
            "WITH old AS (
                 SELECT id, online_node FROM players WHERE name_hash = $1 FOR UPDATE
             )
             UPDATE players p SET online_node = NULL, online_since = NULL
             FROM old
             WHERE p.id = old.id AND old.online_node IS NOT NULL
               AND NOT EXISTS (
                   SELECT 1 FROM world_status w
                   WHERE w.node_id = old.online_node
                     AND ",
            live!("w.updated_at", "$2"),
            ")
             RETURNING old.online_node"
        ))
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
        let result = query(
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
        let released: Vec<i64> = query_scalar(
            "UPDATE players SET online_node = NULL, online_since = NULL
             WHERE online_node = $1 AND name_hash <> ALL($2)
             RETURNING name_hash",
        )
        .bind(node_id)
        .bind(&keep)
        .fetch_all(&mut *tx)
        .await?;
        let lost: Vec<i64> = query_scalar(
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

    /// Counts a wrong-password login from `ip`. Failures are kept by
    /// [`limit_address`], so an IPv6 address counts with its /64.
    pub async fn record_login_failure(&self, ip: IpAddr) -> Result<(), Error> {
        query("INSERT INTO login_failures (ip) VALUES ($1::inet)")
            .bind(limit_address(ip).to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Counts a wrong password from `ip` before it's checked: `None` if
    /// `ip` already gave `max` in the last `secs`. Counted first, requests
    /// sent at once can't all get past the limit while their passwords are
    /// checked. [`Db::forgive_login_attempt`] takes it back when the
    /// password was right.
    pub async fn claim_login_attempt(
        &self,
        ip: IpAddr,
        secs: i32,
        max: i64,
    ) -> Result<Option<LoginAttempt>, Error> {
        let ip = limit_address(ip);
        let row: String =
            query_scalar("INSERT INTO login_failures (ip) VALUES ($1::inet) RETURNING ctid::text")
                .bind(ip.to_string())
                .fetch_one(&self.pool)
                .await?;
        let attempt = LoginAttempt { ip, row };
        // Its own statement, so it sees every attempt counted before this one.
        if self.recent_login_failures(ip, secs).await? > max {
            // A refused one doesn't count: it was never checked.
            self.forgive_login_attempt(attempt).await?;
            return Ok(None);
        }
        Ok(Some(attempt))
    }

    /// Takes back an attempt [`Db::claim_login_attempt`] counted.
    pub async fn forgive_login_attempt(&self, attempt: LoginAttempt) -> Result<(), Error> {
        query("DELETE FROM login_failures WHERE ctid = $1::tid AND ip = $2::inet")
            .bind(attempt.row)
            .bind(attempt.ip.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Wrong-password logins from `ip` in the last `secs`, across all worlds.
    pub async fn recent_login_failures(&self, ip: IpAddr, secs: i32) -> Result<i64, Error> {
        query_scalar(
            "SELECT count(*) FROM login_failures
             WHERE ip = $1::inet AND at > now() - make_interval(secs => $2)",
        )
        .bind(limit_address(ip).to_string())
        .bind(secs)
        .fetch_one(&self.pool)
        .await
    }

    /// Logs that `name_hash` authenticated on world `node_id` from `ip`,
    /// with the client install's `uid` when it sent one worth keeping, and
    /// the `device` it ran on.
    pub async fn record_login(
        &self,
        name_hash: u64,
        node_id: i16,
        ip: IpAddr,
        uid: Option<i32>,
        device: &LoginDevice,
        refused: Option<&str>,
    ) -> Result<(), Error> {
        query(
            "INSERT INTO login_logs (player_id, node_id, ip, uid, web, user_agent, platform, language, touch,
                                     fingerprint, refused, engine, js_platform, gpu, tamper)
             SELECT id, $2, $3::inet, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
             FROM players WHERE name_hash = $1",
        )
        .bind(name_hash as i64)
        .bind(node_id)
        .bind(ip.to_string())
        .bind(uid)
        .bind(device.web)
        .bind(&device.user_agent)
        .bind(&device.platform)
        .bind(&device.language)
        .bind(device.touch)
        .bind(device.fingerprint)
        .bind(refused)
        .bind(&device.engine)
        .bind(&device.js_platform)
        .bind(&device.gpu)
        .bind(device.tamper_column())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Another account online now (on any world) that logged in with
    /// browser `uid`, other than `name_hash`: its name. A browser shared by
    /// more than [`CROWDED_BROWSER`] accounts never counts, as it links
    /// nobody.
    pub async fn online_from_browser(
        &self,
        name_hash: u64,
        uid: i32,
    ) -> Result<Option<String>, Error> {
        query_scalar(concat!(
            "SELECT p.display_name FROM login_logs l JOIN players p ON p.online_login = l.id ",
            "WHERE l.uid = $2 AND p.online_node IS NOT NULL AND p.name_hash <> $1 AND ",
            uncrowded!("uid", "$2", "$3"),
            " LIMIT 1"
        ))
        .bind(name_hash as i64)
        .bind(uid)
        .bind(CROWDED_BROWSER)
        .fetch_optional(&self.pool)
        .await
    }

    /// Forgets failures older than `secs`.
    pub async fn prune_login_failures(&self, secs: i32) -> Result<(), Error> {
        query("DELETE FROM login_failures WHERE at < now() - make_interval(secs => $1)")
            .bind(secs)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
