use crate::{Db, Error};
use sqlx::AssertSqlSafe;
use std::net::IpAddr;

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// What the appeal form needs to check who's appealing, and what against.
#[derive(Debug, sqlx::FromRow)]
pub struct AppealLogin {
    pub display_name: String,
    pub password_hash: String,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
}

/// An appeal to file.
#[derive(Debug)]
pub struct NewAppeal<'a> {
    pub name_hash: u64,
    pub sanction: &'a str,
    pub statement: &'a str,
    pub ip: Option<IpAddr>,
}

/// How filing an appeal went.
#[derive(Debug, PartialEq, Eq)]
pub enum AppealFiled {
    Filed { id: i64 },
    NotSanctioned,
    AlreadyOpen { id: i64 },
    AlreadyDecided { id: i64 },
}

/// An appeal, as the website and the console see it.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct AppealRecord {
    pub id: i64,
    pub player: String,
    pub sanction: String,
    pub sanction_until_ms: i64,
    pub issued_by: Option<String>,
    pub reason: Option<String>,
    pub statement: String,
    pub created_ms: i64,
    pub case_id: Option<i64>,
    pub decision: Option<String>,
    pub decision_note: Option<String>,
    pub decided_by: Option<String>,
    pub decided_ms: Option<i64>,
    /// The dates as the website shows them: `20th October 2026`.
    pub filed_date: String,
    pub decided_date: Option<String>,
}

const RECORD: &str = concat!(
    "SELECT a.id, p.display_name AS player, a.sanction, ",
    ms!("a.sanction_until"),
    " AS sanction_until_ms, a.issued_by, m.reason, a.statement, ",
    ms!("a.created_at"),
    " AS created_ms, a.case_id, a.decision, a.decision_note, a.decided_by, ",
    ms!("a.decided_at"),
    " AS decided_ms, to_char(a.created_at, 'FMDDth FMMonth YYYY') AS filed_date, ",
    "to_char(a.decided_at, 'FMDDth FMMonth YYYY') AS decided_date ",
    "FROM appeals a JOIN players p ON p.id = a.player_id ",
    "LEFT JOIN moderation_logs m ON m.id = a.moderation_log_id"
);

impl Db {
    /// The player `name_hash`, for the appeal form, if registered.
    pub async fn appeal_login(&self, name_hash: u64) -> Result<Option<AppealLogin>, Error> {
        sqlx::query_as(concat!(
            "SELECT display_name, password_hash, CASE WHEN muted_until > now() THEN ",
            ms!("muted_until"),
            " END AS muted_until_ms, CASE WHEN banned_until > now() THEN ",
            ms!("banned_until"),
            " END AS banned_until_ms FROM players WHERE name_hash = $1"
        ))
        .bind(name_hash as i64)
        .fetch_optional(&self.pool)
        .await
    }

    /// Files an appeal against the player's current mute or ban, noting the
    /// moderation log entry that set it (and so who), if one did.
    pub async fn file_appeal(&self, appeal: &NewAppeal<'_>) -> Result<AppealFiled, Error> {
        let column = match appeal.sanction {
            "mute" => "muted_until",
            "ban" => "banned_until",
            _ => return Ok(AppealFiled::NotSanctioned),
        };
        // The sanction's end is only ever read from its column, here in
        // SQL, so comparing it is exact.
        let until = format!("(SELECT {column} FROM players WHERE id = $1)");
        let mut tx = self.pool.begin().await?;
        let player: Option<(i64, bool)> = sqlx::query_as(AssertSqlSafe(format!(
            "SELECT id, coalesce({column} > now(), false)
             FROM players WHERE name_hash = $1 FOR UPDATE"
        )))
        .bind(appeal.name_hash as i64)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((player_id, true)) = player else {
            return Ok(AppealFiled::NotSanctioned);
        };

        let earlier: Option<(i64, bool)> = sqlx::query_as(AssertSqlSafe(format!(
            "SELECT id, decision IS NOT NULL FROM appeals
             WHERE player_id = $1 AND sanction = $2
               AND (decision IS NULL OR sanction_until = {until})
             ORDER BY id DESC LIMIT 1"
        )))
        .bind(player_id)
        .bind(appeal.sanction)
        .fetch_optional(&mut *tx)
        .await?;
        match earlier {
            Some((id, false)) => return Ok(AppealFiled::AlreadyOpen { id }),
            Some((id, true)) => return Ok(AppealFiled::AlreadyDecided { id }),
            None => {}
        }

        // The command that set it: its own length lands on this end time.
        // (An abuse report's mute or the automatic ban has none.)
        let source: Option<(i64, String)> = sqlx::query_as(AssertSqlSafe(format!(
            "SELECT m.id, mp.display_name FROM moderation_logs m
             JOIN players mp ON mp.id = m.moderator_id
             WHERE m.target_id = $1 AND m.action = $2
               AND abs(extract(epoch FROM
                       m.created_at + make_interval(hours => m.hours) - {until})) < 300
             ORDER BY m.id DESC LIMIT 1"
        )))
        .bind(player_id)
        .bind(appeal.sanction)
        .fetch_optional(&mut *tx)
        .await?;
        let id: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
            "INSERT INTO appeals
                 (player_id, sanction, sanction_until, moderation_log_id, issued_by, statement, ip)
             SELECT $1, $2, {column}, $3, $4, $5, $6::inet FROM players WHERE id = $1
             RETURNING id"
        )))
        .bind(player_id)
        .bind(appeal.sanction)
        .bind(source.as_ref().map(|(id, _)| *id))
        .bind(source.map(|(_, name)| name))
        .bind(appeal.statement)
        .bind(appeal.ip.map(|ip| ip.to_string()))
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(AppealFiled::Filed { id })
    }

    /// The appeals `name_hash` has filed, newest first.
    pub async fn appeals_of(&self, name_hash: u64) -> Result<Vec<AppealRecord>, Error> {
        sqlx::query_as(AssertSqlSafe(format!(
            "{RECORD} WHERE p.name_hash = $1 ORDER BY a.id DESC LIMIT 50"
        )))
        .bind(name_hash as i64)
        .fetch_all(&self.pool)
        .await
    }

    /// Appeals the console hasn't taken into a case yet, oldest first.
    pub async fn waiting_appeals(&self, limit: i64) -> Result<Vec<AppealRecord>, Error> {
        sqlx::query_as(AssertSqlSafe(format!(
            "{RECORD} WHERE a.case_id IS NULL ORDER BY a.id LIMIT $1"
        )))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Notes the case appeal `id` was taken into. `false` if there's no
    /// such appeal.
    pub async fn link_appeal(&self, id: i64, case_id: i64) -> Result<bool, Error> {
        let result = sqlx::query("UPDATE appeals SET case_id = $2 WHERE id = $1")
            .bind(id)
            .bind(case_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Records the decision on appeal `id` (a later one replaces it).
    /// `false` if there's no such appeal.
    pub async fn decide_appeal(
        &self,
        id: i64,
        decision: &str,
        note: &str,
        by: &str,
    ) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE appeals SET decision = $2, decision_note = $3, decided_by = $4,
                                decided_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(decision)
        .bind(note)
        .bind(by)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
