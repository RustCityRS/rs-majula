use crate::recovery::RESET_LOCKED;
use crate::{Db, Error};
use sqlx::PgConnection;
use std::net::IpAddr;

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// The most requests one address may send in a day.
pub const RECOVERIES_PER_ADDRESS: i64 = 3;
/// The most requests an account may have waiting on a decision at once,
/// from where it was never played: a request from an address or browser
/// it logged in from is always taken, so strangers can't crowd out its
/// owner.
pub const RECOVERIES_WAITING: i64 = 5;

/// What a player answered about their account. Each is `None` if they
/// didn't know, and free text is as they typed it.
#[derive(Clone, Debug, Default, PartialEq, Eq, sqlx::FromRow)]
pub struct RecoveryAnswers {
    pub created_month: Option<String>,
    pub last_played: Option<String>,
    pub device: Option<String>,
    pub items: Option<String>,
    pub location: Option<String>,
    pub friends: Option<String>,
    pub traded_with: Option<String>,
    pub other_accounts: Option<String>,
}

/// A recovery request to file.
#[derive(Debug)]
pub struct NewRecovery<'a> {
    pub name_hash: u64,
    pub key_hash: &'a str,
    pub reason: &'a str,
    pub details: &'a str,
    pub passwords_given: i16,
    pub password_matched: bool,
    pub answers: &'a RecoveryAnswers,
    pub ip: IpAddr,
    pub uids: &'a [i32],
    pub user_agent: Option<&'a str>,
    pub platform: Option<&'a str>,
    pub language: Option<&'a str>,
}

/// How filing a recovery request went.
#[derive(Debug, PartialEq, Eq)]
pub enum RecoveryFiled {
    Filed { id: i64 },
    NoSuchPlayer,
    TooManyFromAddress,
    TooManyWaiting,
}

/// How deciding a recovery request went.
#[derive(Debug, PartialEq, Eq)]
pub enum RecoveryDecided {
    Decided,
    NotFound,
    AlreadyApproved,
}

/// A recovery request, as the website and the console see it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct RecoveryRecord {
    pub id: i64,
    pub player: String,
    pub name_hash: i64,
    pub reason: String,
    pub details: String,
    pub passwords_given: i16,
    pub password_matched: bool,
    #[sqlx(flatten)]
    pub answers: RecoveryAnswers,
    pub ip: String,
    pub uids: Vec<i32>,
    pub user_agent: Option<String>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub created_ms: i64,
    pub case_id: Option<i64>,
    pub decision: Option<String>,
    pub decision_note: Option<String>,
    pub decided_by: Option<String>,
    pub decided_ms: Option<i64>,
    pub filed_date: String,
    pub decided_date: Option<String>,
    pub key_hash: String,
    pub reset_until_ms: Option<i64>,
}

/// The class of the advisory locks taken per address on filing.
const RECOVERY_LOCK: i32 = 0x5245_4356;

/// The account a request for `name_hash` from `ip` and `uids` is for, or
/// why it can't be filed. `lock` holds the account's row to the end of
/// the transaction.
async fn refusal(
    conn: &mut PgConnection,
    name_hash: u64,
    ip: IpAddr,
    uids: &[i32],
    lock: bool,
) -> Result<Result<i64, RecoveryFiled>, Error> {
    let sql = if lock {
        "SELECT id FROM players WHERE name_hash = $1 FOR UPDATE"
    } else {
        "SELECT id FROM players WHERE name_hash = $1"
    };
    let player: Option<i64> = sqlx::query_scalar(sql)
        .bind(name_hash as i64)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(player_id) = player else {
        return Ok(Err(RecoveryFiled::NoSuchPlayer));
    };
    let (from_address, waiting, played_here): (i64, i64, bool) = sqlx::query_as(
        "SELECT
             (SELECT count(*) FROM recovery_requests
              WHERE ip = $2::inet AND created_at > now() - interval '1 day'),
             (SELECT count(*) FROM recovery_requests
              WHERE player_id = $1 AND decision IS NULL),
             EXISTS (SELECT 1 FROM login_logs WHERE ip = $2::inet AND player_id = $1)
                 OR EXISTS (SELECT 1 FROM login_logs WHERE uid = ANY($3) AND player_id = $1)",
    )
    .bind(player_id)
    .bind(ip.to_string())
    .bind(uids)
    .fetch_one(&mut *conn)
    .await?;
    Ok(if from_address >= RECOVERIES_PER_ADDRESS {
        Err(RecoveryFiled::TooManyFromAddress)
    } else if waiting >= RECOVERIES_WAITING && !played_here {
        Err(RecoveryFiled::TooManyWaiting)
    } else {
        Ok(player_id)
    })
}

const RECORD: &str = concat!(
    "SELECT r.id, p.display_name AS player, p.name_hash, r.reason, r.details, ",
    "r.passwords_given, r.password_matched, ",
    "to_char(r.created_month, 'YYYY-MM') AS created_month, ",
    "to_char(r.last_played, 'YYYY-MM-DD') AS last_played, r.device, r.items, ",
    "r.location, r.friends, r.traded_with, r.other_accounts, host(r.ip) AS ip, r.uids, ",
    "r.user_agent, r.platform, r.language, ",
    ms!("r.created_at"),
    " AS created_ms, r.case_id, r.decision, r.decision_note, r.decided_by, ",
    ms!("r.decided_at"),
    " AS decided_ms, to_char(r.created_at, 'FMDDth FMMonth YYYY') AS filed_date, ",
    "to_char(r.decided_at, 'FMDDth FMMonth YYYY') AS decided_date, r.key_hash, ",
    "CASE WHEN p.reset_code_hash = r.key_hash AND p.reset_code_until > now() THEN ",
    ms!("p.reset_code_until"),
    " END AS reset_until_ms ",
    "FROM recovery_requests r JOIN players p ON p.id = r.player_id"
);

/// What the game's records say about the account a recovery request is
/// for, measured against where the request came from.
#[derive(Clone, Debug, Default, PartialEq, Eq, sqlx::FromRow)]
pub struct RecoveryFacts {
    pub registered_ms: i64,
    pub registered_from_ip: bool,
    pub logins: i64,
    pub last_login_ms: Option<i64>,
    pub from_ip: i64,
    pub last_from_ip_ms: Option<i64>,
    pub from_network: i64,
    pub from_browser: i64,
    pub last_from_browser_ms: Option<i64>,
    pub same_agent: i64,
    pub same_platform: i64,
    pub same_language: i64,
    pub web_logins: i64,
    pub phone_logins: i64,
    pub client_logins: i64,
    pub logins_since: i64,
    pub addresses_since: i64,
    pub new_addresses_since: i64,
    pub other_requests: i64,
    pub other_requests_elsewhere: i64,
}

impl Db {
    /// Files a recovery request for the account `name_hash`, unless the
    /// address or the account has sent too many already.
    pub async fn file_recovery(&self, request: &NewRecovery<'_>) -> Result<RecoveryFiled, Error> {
        let mut tx = self.pool.begin().await?;
        // One at a time from an address, so its count holds.
        sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2))")
            .bind(RECOVERY_LOCK)
            .bind(request.ip.to_string())
            .execute(&mut *tx)
            .await?;
        let player_id =
            match refusal(&mut tx, request.name_hash, request.ip, request.uids, true).await? {
                Ok(id) => id,
                Err(refused) => return Ok(refused),
            };
        let answers = request.answers;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO recovery_requests
                 (player_id, key_hash, reason, details, passwords_given, password_matched,
                  created_month, last_played, device, items, location, friends,
                  traded_with, other_accounts, ip, uids, user_agent, platform, language)
             VALUES ($1, $2, $3, $4, $5, $6, ($7 || '-01')::date, $8::date, $9, $10, $11, $12,
                     $13, $14, $15::inet, $16, $17, $18, $19)
             RETURNING id",
        )
        .bind(player_id)
        .bind(request.key_hash)
        .bind(request.reason)
        .bind(request.details)
        .bind(request.passwords_given)
        .bind(request.password_matched)
        .bind(&answers.created_month)
        .bind(&answers.last_played)
        .bind(&answers.device)
        .bind(&answers.items)
        .bind(&answers.location)
        .bind(&answers.friends)
        .bind(&answers.traded_with)
        .bind(&answers.other_accounts)
        .bind(request.ip.to_string())
        .bind(request.uids)
        .bind(request.user_agent)
        .bind(request.platform)
        .bind(request.language)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(RecoveryFiled::Filed { id })
    }

    /// Why [`Db::file_recovery`] would refuse a request for `name_hash`
    /// from `ip` and `uids` now, if it would: asked first, before the work
    /// of making one.
    pub async fn recovery_refusal(
        &self,
        name_hash: u64,
        ip: IpAddr,
        uids: &[i32],
    ) -> Result<Option<RecoveryFiled>, Error> {
        let mut conn = self.pool.acquire().await?;
        Ok(refusal(&mut conn, name_hash, ip, uids, false).await?.err())
    }

    /// Recovery request `id`.
    pub async fn recovery(&self, id: i64) -> Result<Option<RecoveryRecord>, Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!("{RECORD} WHERE r.id = $1")))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Recovery requests the console hasn't taken into a case yet, oldest
    /// first.
    pub async fn waiting_recoveries(&self, limit: i64) -> Result<Vec<RecoveryRecord>, Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{RECORD} WHERE r.case_id IS NULL ORDER BY r.id LIMIT $1"
        )))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Notes the case recovery request `id` was taken into. `false` if
    /// there's no such request.
    pub async fn link_recovery(&self, id: i64, case_id: i64) -> Result<bool, Error> {
        let result = sqlx::query("UPDATE recovery_requests SET case_id = $2 WHERE id = $1")
            .bind(id)
            .bind(case_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Records the decision on recovery request `id`. Approving issues the
    /// account a password reset its recovery key redeems for the next
    /// `hours`, and stops its current password working; denying one that
    /// was approved takes back a reset still unredeemed.
    pub async fn decide_recovery(
        &self,
        id: i64,
        approve: bool,
        note: &str,
        by: &str,
        hours: i32,
    ) -> Result<RecoveryDecided, Error> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(i64, String, Option<String>)> = sqlx::query_as(
            "SELECT player_id, key_hash, decision FROM recovery_requests WHERE id = $1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((player_id, key_hash, decision)) = row else {
            return Ok(RecoveryDecided::NotFound);
        };
        if approve && decision.as_deref() == Some("approved") {
            return Ok(RecoveryDecided::AlreadyApproved);
        }
        sqlx::query(
            "UPDATE recovery_requests
             SET decision = $2, decision_note = $3, decided_by = $4, decided_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(if approve { "approved" } else { "denied" })
        .bind(note)
        .bind(by)
        .execute(&mut *tx)
        .await?;
        if approve {
            sqlx::query(
                "UPDATE players
                 SET reset_code_hash = $2,
                     reset_code_until = now() + make_interval(hours => $3),
                     password_hash = $4
                 WHERE id = $1",
            )
            .bind(player_id)
            .bind(&key_hash)
            .bind(hours)
            .bind(RESET_LOCKED)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "UPDATE players SET reset_code_hash = NULL, reset_code_until = NULL
                 WHERE id = $1 AND reset_code_hash = $2",
            )
            .bind(player_id)
            .bind(&key_hash)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(RecoveryDecided::Decided)
    }

    /// What the account `name_hash`'s logins say about recovery request
    /// `id`, sent from `ip` (and the browser `uids`, saying `user_agent`,
    /// `platform` and `language`) by someone who says they last played on
    /// `last_played` (`2026-10-18`). `None` if there's no such account.
    #[allow(clippy::too_many_arguments)]
    pub async fn recovery_facts(
        &self,
        id: i64,
        name_hash: u64,
        ip: &str,
        uids: &[i32],
        user_agent: Option<&str>,
        platform: Option<&str>,
        language: Option<&str>,
        last_played: Option<&str>,
    ) -> Result<Option<RecoveryFacts>, Error> {
        sqlx::query_as(concat!(
            "WITH p AS (SELECT id, created_at, registration_ip FROM players WHERE name_hash = $1), ",
            "l AS (SELECT * FROM login_logs WHERE player_id = (SELECT id FROM p)), ",
            "since AS (SELECT * FROM l WHERE created_at >= $8::date + 1) ",
            "SELECT ",
            ms!("p.created_at"),
            " AS registered_ms, coalesce(p.registration_ip = $2::inet, false) AS registered_from_ip, ",
            "(SELECT count(*) FROM l) AS logins, ",
            "(SELECT ",
            ms!("max(created_at)"),
            " FROM l) AS last_login_ms, ",
            "(SELECT count(*) FROM l WHERE ip = $2::inet) AS from_ip, ",
            "(SELECT ",
            ms!("max(created_at)"),
            " FROM l WHERE ip = $2::inet) AS last_from_ip_ms, ",
            "(SELECT count(*) FROM l WHERE ip <<= network(set_masklen($2::inet, ",
            "CASE family($2::inet) WHEN 4 THEN 24 ELSE 48 END))) AS from_network, ",
            "(SELECT count(*) FROM l WHERE uid = ANY($3)) AS from_browser, ",
            "(SELECT ",
            ms!("max(created_at)"),
            " FROM l WHERE uid = ANY($3)) AS last_from_browser_ms, ",
            "(SELECT count(*) FROM l WHERE user_agent = $4) AS same_agent, ",
            "(SELECT count(*) FROM l WHERE platform = $5) AS same_platform, ",
            "(SELECT count(*) FROM l WHERE language = $6) AS same_language, ",
            "(SELECT count(*) FROM l WHERE web AND NOT touch) AS web_logins, ",
            "(SELECT count(*) FROM l WHERE web AND touch) AS phone_logins, ",
            "(SELECT count(*) FROM l WHERE NOT web) AS client_logins, ",
            "(SELECT count(*) FROM since) AS logins_since, ",
            "(SELECT count(DISTINCT ip) FROM since) AS addresses_since, ",
            "(SELECT count(DISTINCT ip) FROM since WHERE ip NOT IN ",
            "(SELECT ip FROM l WHERE created_at < $8::date + 1)) AS new_addresses_since, ",
            "(SELECT count(*) FROM recovery_requests WHERE player_id = p.id AND id <> $7) ",
            "AS other_requests, ",
            "(SELECT count(*) FROM recovery_requests WHERE player_id = p.id AND id <> $7 ",
            "AND ip <> $2::inet) AS other_requests_elsewhere ",
            "FROM p"
        ))
        .bind(name_hash as i64)
        .bind(ip)
        .bind(uids)
        .bind(user_agent)
        .bind(platform)
        .bind(language)
        .bind(id)
        .bind(last_played)
        .fetch_optional(&self.pool)
        .await
    }

    /// Other accounts that logged in from `ip` or the browser `uids`, by
    /// name (at most `limit`).
    pub async fn accounts_seen_from(
        &self,
        name_hash: u64,
        ip: &str,
        uids: &[i32],
        limit: i64,
    ) -> Result<Vec<String>, Error> {
        sqlx::query_scalar(
            "SELECT p.display_name FROM players p
             WHERE p.name_hash <> $1 AND EXISTS (
                 SELECT 1 FROM login_logs l
                 WHERE l.player_id = p.id AND (l.ip = $2::inet OR l.uid = ANY($3)))
             ORDER BY p.display_name LIMIT $4",
        )
        .bind(name_hash as i64)
        .bind(ip)
        .bind(uids)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// The name hashes of the players on `name_hash`'s friend list (the
    /// ether service's table).
    pub async fn friend_hashes(&self, name_hash: u64) -> Result<Vec<u64>, Error> {
        let hashes: Option<Vec<i64>> =
            sqlx::query_scalar("SELECT friend_hashes FROM friends WHERE owner_hash = $1")
                .bind(name_hash as i64)
                .fetch_optional(&self.pool)
                .await?;
        Ok(hashes
            .unwrap_or_default()
            .into_iter()
            .map(|h| h as u64)
            .collect())
    }

    /// The name hashes of the players `name_hash` passed items to or got
    /// them from (trades, stakes and the like), as long as those are kept.
    pub async fn trade_partner_hashes(&self, name_hash: u64) -> Result<Vec<u64>, Error> {
        let hashes: Vec<i64> = sqlx::query_scalar(
            "SELECT DISTINCT cp.name_hash FROM wealth_events w
             JOIN players cp ON cp.id = w.counterpart_id
             WHERE w.player_id = (SELECT id FROM players WHERE name_hash = $1)",
        )
        .bind(name_hash as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(hashes.into_iter().map(|h| h as u64).collect())
    }
}
