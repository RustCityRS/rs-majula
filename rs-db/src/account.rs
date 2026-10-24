use crate::{Db, Error};
use rs_util::base37::{to_raw_username, to_screen_name};
use sqlx::AssertSqlSafe;

/// A player's own account, as the website's account page shows it to them.
#[derive(Debug, sqlx::FromRow)]
pub struct Account {
    pub display_name: String,
    pub password_hash: String,
    /// When it was made, as `23 October 2026`.
    pub created_date: String,
    pub staff: bool,
    /// A world has the player logged in.
    pub online: bool,
    pub banned: bool,
    pub muted: bool,
}

/// One of a player's logins, as they see it.
#[derive(Debug, sqlx::FromRow)]
pub struct AccountLogin {
    /// As `23 Oct 2026 18:04` (UTC).
    pub at: String,
    /// The game server's node id: the website names it as its world.
    pub node: i16,
    pub ip: String,
    pub web: Option<bool>,
    pub user_agent: Option<String>,
    pub platform: Option<String>,
}

/// What came of a player deleting their account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountDeleted {
    Deleted,
    /// Not while a world has them logged in.
    Online,
    /// Not while banned or muted: the sanction would go with it.
    Sanctioned,
    /// Staff accounts are taken down by another member of staff.
    Staff,
    Gone,
}

/// Everything kept about a player, as JSON, for [`Db::export_account`]
/// (worlds by the number players know, world N being node N + 9): the
/// account (no password or reset code), the save and earlier ones, the
/// hiscores, logins, chat, game events, item events, recordings (when and
/// where, not the anti-cheat checks' verdicts), the reports they made, the
/// sanctions given them, their appeals and recovery requests, and the
/// messages staff sent them (never naming who sent them). `$2` is their
/// friends list and `$3` their ignore list, as names.
const EXPORT: &str = "
SELECT jsonb_build_object(
  'exported_at', now(),
  'account', to_jsonb(p) - ARRAY['id', 'name_hash', 'password_hash', 'reset_code_hash', 'reset_code_until'],
  'save', (SELECT to_jsonb(s) - 'player_id' FROM player_saves s WHERE s.player_id = p.id),
  'earlier_saves', (SELECT coalesce(jsonb_agg((to_jsonb(h) - ARRAY['id', 'player_id', 'node_id']) || jsonb_build_object('world', h.node_id - 9) ORDER BY h.saved_at), '[]')
                    FROM save_history h WHERE h.player_id = p.id),
  'hiscores', (SELECT coalesce(jsonb_agg(to_jsonb(s) - 'player_id' ORDER BY s.category), '[]')
               FROM hiscores s WHERE s.player_id = p.id),
  'friends', to_jsonb($2::text[]),
  'ignores', to_jsonb($3::text[]),
  'logins', (SELECT coalesce(jsonb_agg((to_jsonb(l) - ARRAY['id', 'player_id', 'node_id']) || jsonb_build_object('world', l.node_id - 9) ORDER BY l.created_at), '[]')
             FROM login_logs l WHERE l.player_id = p.id),
  'chat', (SELECT coalesce(jsonb_agg((to_jsonb(c) - ARRAY['id', 'player_id', 'to_name_hash', 'node_id'])
                                     || jsonb_build_object('to', t.display_name, 'world', c.node_id - 9) ORDER BY c.created_at), '[]')
           FROM chat_logs c LEFT JOIN players t ON t.name_hash = c.to_name_hash WHERE c.player_id = p.id),
  'game_events', (SELECT coalesce(jsonb_agg((to_jsonb(e) - ARRAY['id', 'player_id', 'node_id']) || jsonb_build_object('world', e.node_id - 9) ORDER BY e.created_at), '[]')
                  FROM session_logs e WHERE e.player_id = p.id),
  'item_events', (SELECT coalesce(jsonb_agg((to_jsonb(w) - ARRAY['id', 'player_id', 'counterpart_id', 'node_id'])
                                            || jsonb_build_object('with', o.display_name, 'world', w.node_id - 9) ORDER BY w.created_at), '[]')
                  FROM wealth_events w LEFT JOIN players o ON o.id = w.counterpart_id WHERE w.player_id = p.id),
  'recordings', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                     'id', r.id::text, 'world', r.node_id - 9, 'started_at', r.started_at,
                     'ended_at', r.ended_at, 'duration_ms', r.duration_ms) ORDER BY r.started_at), '[]')
                 FROM replays r WHERE r.player_id = p.id),
  'reports_made', (SELECT coalesce(jsonb_agg((to_jsonb(s) - ARRAY['id', 'reporter_id', 'offender_name_hash', 'node_id'])
                                             || jsonb_build_object('reported', o.display_name, 'world', s.node_id - 9) ORDER BY s.created_at), '[]')
                   FROM snapshots s LEFT JOIN players o ON o.name_hash = s.offender_name_hash
                   WHERE s.reporter_id = p.id),
  'sanctions', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                    'action', m.action, 'hours', m.hours, 'reason', m.reason, 'world', m.node_id - 9,
                    'at', m.created_at) ORDER BY m.created_at), '[]')
                FROM moderation_logs m WHERE m.target_id = p.id),
  'appeals', (SELECT coalesce(jsonb_agg(to_jsonb(a) - ARRAY['player_id', 'moderation_log_id', 'case_id', 'decided_by']
                                        ORDER BY a.created_at), '[]')
              FROM appeals a WHERE a.player_id = p.id),
  'recovery_requests', (SELECT coalesce(jsonb_agg(to_jsonb(r) - ARRAY['player_id', 'key_hash', 'case_id', 'decided_by']
                                                  ORDER BY r.created_at), '[]')
                        FROM recovery_requests r WHERE r.player_id = p.id),
  'messages', (SELECT coalesce(jsonb_agg(jsonb_build_object(
                   'subject', m.subject, 'text', m.body, 'sent_at', m.created_at, 'read_at', m.read_at)
                   ORDER BY m.id), '[]')
               FROM staff_messages m WHERE m.player_id = p.id)
)::text
FROM players p WHERE p.name_hash = $1";

impl Db {
    /// The player `name_hash`'s own account, if registered.
    pub async fn account(&self, name_hash: u64) -> Result<Option<Account>, Error> {
        sqlx::query_as(
            "SELECT display_name, password_hash,
                    to_char(created_at, 'FMDD FMMonth YYYY') AS created_date,
                    staff_mod_level > 0 AS staff, online_node IS NOT NULL AS online,
                    coalesce(banned_until > now(), false) AS banned,
                    coalesce(muted_until > now(), false) AS muted
             FROM players WHERE name_hash = $1",
        )
        .bind(name_hash as i64)
        .fetch_optional(&self.pool)
        .await
    }

    /// The player's `limit` latest logins, newest first.
    pub async fn account_logins(
        &self,
        name_hash: u64,
        limit: i64,
    ) -> Result<Vec<AccountLogin>, Error> {
        sqlx::query_as(
            "SELECT to_char(l.created_at AT TIME ZONE 'UTC', 'DD Mon YYYY HH24:MI') AS at,
                    l.node_id AS node, host(l.ip) AS ip, l.web, l.user_agent, l.platform
             FROM login_logs l JOIN players p ON p.id = l.player_id
             WHERE p.name_hash = $1 ORDER BY l.created_at DESC LIMIT $2",
        )
        .bind(name_hash as i64)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Sets the player's password to `password_hash` if it's still the one
    /// hashed as `current_hash` (so two changes at once can't both take),
    /// and ends any password reset they were issued. Returns whether it took.
    pub async fn change_password(
        &self,
        name_hash: u64,
        current_hash: &str,
        password_hash: &str,
    ) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE players
             SET password_hash = $3, reset_code_hash = NULL, reset_code_until = NULL
             WHERE name_hash = $1 AND password_hash = $2",
        )
        .bind(name_hash as i64)
        .bind(current_hash)
        .bind(password_hash)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Everything kept about the player in the database, as JSON (see
    /// `EXPORT`). `None` if not registered.
    pub async fn export_account(&self, name_hash: u64) -> Result<Option<String>, Error> {
        let friends = self
            .social_names("friends", "friend_hashes", name_hash)
            .await?;
        let ignores = self
            .social_names("ignores", "ignore_hashes", name_hash)
            .await?;
        sqlx::query_scalar(EXPORT)
            .bind(name_hash as i64)
            .bind(friends)
            .bind(ignores)
            .fetch_optional(&self.pool)
            .await
    }

    /// Deletes the player's account and everything kept with it (every
    /// table here cascades), with their friends and ignore lists, unless
    /// they're logged in, banned, muted or staff.
    pub async fn delete_account(&self, name_hash: u64) -> Result<AccountDeleted, Error> {
        let mut tx = self.pool.begin().await?;
        // In the one statement, so a world logging them in either beats it
        // (and nothing's deleted) or finds no one to log in.
        let deleted = sqlx::query(
            "DELETE FROM players WHERE name_hash = $1 AND online_node IS NULL
               AND staff_mod_level = 0
               AND NOT coalesce(banned_until > now(), false)
               AND NOT coalesce(muted_until > now(), false)",
        )
        .bind(name_hash as i64)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if deleted == 0 {
            let why = self.account(name_hash).await?;
            return Ok(match why {
                None => AccountDeleted::Gone,
                Some(a) if a.staff => AccountDeleted::Staff,
                Some(a) if a.online => AccountDeleted::Online,
                Some(_) => AccountDeleted::Sanctioned,
            });
        }
        // The social server's lists, where it keeps them in this database.
        for table in ["friends", "ignores"] {
            if self.has_table(table).await? {
                sqlx::query(AssertSqlSafe(format!(
                    "DELETE FROM {table} WHERE owner_hash = $1"
                )))
                .bind(name_hash as i64)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(AccountDeleted::Deleted)
    }

    /// The names on the player's list in the social server's `table`
    /// (`friends` or `ignores`), empty where it doesn't keep them here.
    async fn social_names(
        &self,
        table: &'static str,
        column: &'static str,
        name_hash: u64,
    ) -> Result<Vec<String>, Error> {
        if !self.has_table(table).await? {
            return Ok(Vec::new());
        }
        let hashes: Option<Vec<i64>> = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT {column} FROM {table} WHERE owner_hash = $1"
        )))
        .bind(name_hash as i64)
        .fetch_optional(&self.pool)
        .await?;
        Ok(hashes
            .unwrap_or_default()
            .into_iter()
            .map(|h| to_screen_name(&to_raw_username(h as u64)))
            .collect())
    }

    async fn has_table(&self, table: &str) -> Result<bool, Error> {
        sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
            .bind(table)
            .fetch_one(&self.pool)
            .await
    }
}
