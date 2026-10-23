use crate::{Db, Error};
use std::collections::{HashMap, HashSet};

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// An address shared by more accounts than this is a public one (a school,
/// a mobile carrier, a VPN exit), and links nobody.
pub const CROWDED_ADDRESS: i64 = 25;
/// A browser (or desktop client install) more accounts than this logged in
/// with links nobody either: a bot farm's, or one that never got an id of
/// its own.
pub const CROWDED_BROWSER: i64 = 25;

/// The accounts that logged in with `$value` in `$column` (`ip` or
/// `uid`), as an array: `$cap` + 1 of them at most, read along the index on
/// (`$column`, player). More than `$cap` is crowded.
macro_rules! accounts_on {
    ($column:literal, $value:literal, $cap:literal) => {
        concat!(
            "ARRAY(SELECT DISTINCT c.player_id FROM login_logs c WHERE c.",
            $column,
            " = ",
            $value,
            " LIMIT ",
            $cap,
            " + 1)"
        )
    };
}

/// Whether at most `$cap` accounts logged in with `$value` in `$column`
/// (see [`accounts_on!`]).
macro_rules! uncrowded {
    ($column:literal, $value:literal, $cap:literal) => {
        concat!(
            "cardinality(",
            $crate::links::accounts_on!($column, $value, $cap),
            ") <= ",
            $cap
        )
    };
}
pub(crate) use {accounts_on, uncrowded};

/// A banned account that shares a browser or an address with a player.
#[derive(Clone, Debug, PartialEq, sqlx::FromRow)]
pub struct BannedLink {
    pub name: String,
    pub via: String,
    pub banned_ms: Option<i64>,
    pub after_ban: bool,
}

/// One account a chain of shared browsers and addresses reaches.
#[derive(Clone, Debug, PartialEq)]
pub struct ChainLink {
    pub name: String,
    pub staff_mod_level: i16,
    pub depth: u8,
    pub via: String,
    pub link: String,
    pub muted_until_ms: Option<i64>,
    pub banned_until_ms: Option<i64>,
    pub last_ms: Option<i64>,
}

/// Logins from a banned account's browser or address by another account,
/// after the ban.
#[derive(Clone, Debug, PartialEq, sqlx::FromRow)]
pub struct EvasionRecord {
    pub name: String,
    pub staff_mod_level: i16,
    pub banned: String,
    /// `browser` or `address`.
    pub via: String,
    pub banned_ms: i64,
    pub logins: i64,
    pub first_ms: i64,
    pub last_ms: i64,
}

#[derive(sqlx::FromRow)]
struct Hop {
    player_id: i64,
    via_id: i64,
    link: String,
}

/// A staff account: its name and game staff level.
#[derive(Clone, Debug, PartialEq, sqlx::FromRow)]
pub struct StaffAccount {
    pub name: String,
    pub staff_mod_level: i16,
}

#[derive(sqlx::FromRow)]
struct Account {
    id: i64,
    display_name: String,
    staff_mod_level: i16,
    muted_until_ms: Option<i64>,
    banned_until_ms: Option<i64>,
    last_ms: Option<i64>,
}

impl Db {
    /// Every account with a staff level.
    pub async fn staff_accounts(&self) -> Result<Vec<StaffAccount>, Error> {
        sqlx::query_as(
            "SELECT display_name AS name, staff_mod_level FROM players WHERE staff_mod_level > 0 ORDER BY display_name",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// How `a` and `b` are linked, if they are: `browser` when they logged
    /// in with the same one, else `address` when they share an (uncrowded)
    /// address, by login or registration.
    pub async fn link_between(&self, a: u64, b: u64) -> Result<Option<String>, Error> {
        let row: Option<(String,)> = sqlx::query_as(concat!(
            "WITH pa AS (SELECT id, registration_ip FROM players WHERE name_hash = $1), ",
            "pb AS (SELECT id, registration_ip FROM players WHERE name_hash = $2), ",
            "ips AS (SELECT l.player_id, l.ip FROM login_logs l WHERE l.player_id IN ((SELECT id FROM pa), (SELECT id FROM pb)) ",
            "UNION SELECT id, registration_ip FROM pa WHERE registration_ip IS NOT NULL ",
            "UNION SELECT id, registration_ip FROM pb WHERE registration_ip IS NOT NULL), ",
            "a_uids AS (SELECT DISTINCT uid FROM login_logs ",
            "WHERE player_id = (SELECT id FROM pa) AND uid IS NOT NULL) ",
            "SELECT 'browser'::text FROM a_uids u WHERE EXISTS (SELECT 1 FROM login_logs y ",
            "WHERE y.player_id = (SELECT id FROM pb) AND y.uid = u.uid) AND ",
            uncrowded!("uid", "u.uid", "$4"),
            " UNION ALL SELECT 'address'::text FROM ips x JOIN ips y ON x.ip = y.ip ",
            "WHERE x.player_id = (SELECT id FROM pa) AND y.player_id = (SELECT id FROM pb) AND ",
            uncrowded!("ip", "x.ip", "$3"),
            " LIMIT 1"
        ))
        .bind(a as i64)
        .bind(b as i64)
        .bind(CROWDED_ADDRESS)
        .bind(CROWDED_BROWSER)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(via,)| via))
    }

    /// The banned accounts that logged in with one of `name_hash`'s
    /// browsers, or from one of their (uncrowded) addresses.
    pub async fn banned_links(&self, name_hash: u64) -> Result<Vec<BannedLink>, Error> {
        sqlx::query_as(concat!(
            "WITH me AS (SELECT id FROM players WHERE name_hash = $1), ",
            "mine AS (SELECT uid, ip, created_at FROM login_logs WHERE player_id = (SELECT id FROM me)), ",
            "uids AS (SELECT u.uid, ",
            accounts_on!("uid", "u.uid", "$3"),
            " AS ids FROM (SELECT DISTINCT uid FROM mine WHERE uid IS NOT NULL) u), ",
            "ips AS (SELECT a.ip, ",
            accounts_on!("ip", "a.ip", "$2"),
            " AS ids FROM (SELECT DISTINCT ip FROM mine) a), ",
            "shared AS (SELECT * FROM (",
            "SELECT unnest(ids) AS player_id, 'browser'::text AS via, uid, NULL::inet AS ip ",
            "FROM uids WHERE cardinality(ids) <= $3 ",
            "UNION ALL SELECT unnest(ids), 'address'::text, NULL::int, ip FROM ips ",
            "WHERE cardinality(ids) <= $2) s WHERE player_id <> (SELECT id FROM me)) ",
            "SELECT p.display_name AS name, s.via, ",
            ms!("ban.at"),
            " AS banned_ms, bool_or(EXISTS (SELECT 1 FROM mine m WHERE ",
            "(m.uid = s.uid OR m.ip = s.ip) AND m.created_at > ban.at)) AS after_ban ",
            "FROM shared s JOIN players p ON p.id = s.player_id AND p.banned_until > now() ",
            "LEFT JOIN LATERAL (SELECT max(created_at) AS at FROM moderation_logs ",
            "WHERE target_id = p.id AND action = 'ban') ban ON true ",
            "GROUP BY p.display_name, s.via, ban.at ",
            "ORDER BY after_ban DESC, via, name LIMIT 50"
        ))
        .bind(name_hash as i64)
        .bind(CROWDED_ADDRESS)
        .bind(CROWDED_BROWSER)
        .fetch_all(&self.pool)
        .await
    }

    /// Every account `name_hash` reaches through shared browsers and
    /// (uncrowded) addresses, up to `depth` links away and at most `limit`
    /// of them, nearest first. A browser link is preferred over an address
    /// one when an account is reached both ways.
    pub async fn link_chain(
        &self,
        name_hash: u64,
        depth: u8,
        limit: usize,
    ) -> Result<Vec<ChainLink>, Error> {
        let Some((start,)) =
            sqlx::query_as::<_, (i64,)>("SELECT id FROM players WHERE name_hash = $1")
                .bind(name_hash as i64)
                .fetch_optional(&self.pool)
                .await?
        else {
            return Ok(Vec::new());
        };
        let mut seen: HashSet<i64> = HashSet::from([start]);
        // Each account reached: its depth, who it was reached from, and how.
        let mut reached: Vec<(i64, u8, i64, String)> = Vec::new();
        let mut frontier = vec![start];
        for hop in 1..=depth {
            if frontier.is_empty() || reached.len() >= limit {
                break;
            }
            // Each browser and address the frontier used, through one of them.
            let hops: Vec<Hop> = sqlx::query_as(concat!(
                "WITH mine AS (SELECT DISTINCT player_id, ip, uid FROM login_logs WHERE player_id = ANY($1)), ",
                "uids AS (SELECT DISTINCT ON (uid) uid, player_id FROM mine WHERE uid IS NOT NULL ",
                "ORDER BY uid, player_id), ",
                "ips AS (SELECT DISTINCT ON (ip) ip, player_id FROM mine ORDER BY ip, player_id) ",
                "SELECT * FROM (SELECT unnest(a.ids) AS player_id, a.via_id, a.link FROM (",
                "SELECT f.player_id AS via_id, 'browser'::text AS link, ",
                accounts_on!("uid", "f.uid", "$4"),
                " AS ids, $4 AS cap FROM uids f ",
                "UNION ALL SELECT f.player_id, 'address'::text, ",
                accounts_on!("ip", "f.ip", "$3"),
                ", $3 FROM ips f) a WHERE cardinality(a.ids) <= a.cap) o ",
                "WHERE o.player_id <> ALL($2)"
            ))
            .bind(&frontier)
            .bind(seen.iter().copied().collect::<Vec<_>>())
            .bind(CROWDED_ADDRESS)
            .bind(CROWDED_BROWSER)
            .fetch_all(&self.pool)
            .await?;
            let mut next: HashMap<i64, (i64, String)> = HashMap::new();
            for h in hops {
                let entry = next
                    .entry(h.player_id)
                    .or_insert((h.via_id, h.link.clone()));
                if h.link == "browser" {
                    *entry = (h.via_id, h.link);
                }
            }
            let mut next: Vec<(i64, (i64, String))> = next.into_iter().collect();
            next.sort_by_key(|(id, (_, link))| (link != "browser", *id));
            next.truncate(limit - reached.len());
            frontier = next.iter().map(|(id, _)| *id).collect();
            for (id, (via, link)) in next {
                seen.insert(id);
                reached.push((id, hop, via, link));
            }
        }

        let mut ids: Vec<i64> = reached.iter().map(|r| r.0).collect();
        ids.push(start);
        let accounts: Vec<Account> = sqlx::query_as(concat!(
            "SELECT p.id, p.display_name, p.staff_mod_level, ",
            "CASE WHEN p.muted_until > now() THEN ",
            ms!("p.muted_until"),
            " END AS muted_until_ms, CASE WHEN p.banned_until > now() THEN ",
            ms!("p.banned_until"),
            " END AS banned_until_ms, (SELECT ",
            ms!("max(l.created_at)"),
            " FROM login_logs l WHERE l.player_id = p.id) AS last_ms ",
            "FROM players p WHERE p.id = ANY($1)"
        ))
        .bind(&ids)
        .fetch_all(&self.pool)
        .await?;
        let by_id: HashMap<i64, &Account> = accounts.iter().map(|a| (a.id, a)).collect();
        Ok(reached
            .into_iter()
            .filter_map(|(id, depth, via, link)| {
                let account = by_id.get(&id)?;
                Some(ChainLink {
                    name: account.display_name.clone(),
                    staff_mod_level: account.staff_mod_level,
                    depth,
                    via: by_id.get(&via)?.display_name.clone(),
                    link,
                    muted_until_ms: account.muted_until_ms,
                    banned_until_ms: account.banned_until_ms,
                    last_ms: account.last_ms,
                })
            })
            .collect())
    }

    /// Accounts that logged in, over the last `days`, with a browser a
    /// banned account used, or from one of its (uncrowded) addresses, after
    /// that ban: possible ban evasion. `via` is `browser` when any of those
    /// logins shared the browser, else `address`. Accounts banned themselves
    /// are left out, as are staff above `max_staff_level`. It starts from
    /// the logins over those days, and looks up only the banned accounts
    /// their browsers and addresses lead to.
    pub async fn ban_evasion(
        &self,
        days: i32,
        max_staff_level: i16,
        limit: i64,
    ) -> Result<Vec<EvasionRecord>, Error> {
        sqlx::query_as(concat!(
            "WITH recent AS (SELECT l.id, l.player_id, l.ip, l.uid, l.created_at FROM login_logs l ",
            "JOIN players p ON p.id = l.player_id ",
            "WHERE l.created_at > now() - make_interval(days => $1) ",
            "AND NOT coalesce(p.banned_until > now(), false) AND p.staff_mod_level <= $2), ",
            "uids AS (SELECT u.uid, ",
            accounts_on!("uid", "u.uid", "$5"),
            " AS ids FROM (SELECT DISTINCT uid FROM recent WHERE uid IS NOT NULL) u), ",
            "ips AS (SELECT a.ip, ",
            accounts_on!("ip", "a.ip", "$4"),
            " AS ids FROM (SELECT DISTINCT ip FROM recent) a), ",
            "shared AS (SELECT uid, NULL::inet AS ip, unnest(ids) AS player_id FROM uids ",
            "WHERE cardinality(ids) <= $5 ",
            "UNION ALL SELECT NULL::int, ip, unnest(ids) FROM ips WHERE cardinality(ids) <= $4), ",
            "banned AS (SELECT s.uid, s.ip, b.id, b.display_name, ban.at FROM shared s ",
            "JOIN players b ON b.id = s.player_id AND b.banned_until > now() ",
            "JOIN LATERAL (SELECT max(m.created_at) AS at FROM moderation_logs m ",
            "WHERE m.target_id = b.id AND m.action = 'ban') ban ON ban.at IS NOT NULL), ",
            "hits AS (SELECT r.id, r.player_id, r.created_at, d.display_name, d.at, true AS browser ",
            "FROM banned d JOIN recent r ON r.uid = d.uid AND r.player_id <> d.id ",
            "WHERE r.created_at > d.at ",
            "UNION ALL SELECT r.id, r.player_id, r.created_at, d.display_name, d.at, false ",
            "FROM banned d JOIN recent r ON r.ip = d.ip AND r.player_id <> d.id ",
            "WHERE r.created_at > d.at) ",
            "SELECT p.display_name AS name, p.staff_mod_level, h.display_name AS banned, ",
            "CASE WHEN bool_or(h.browser) THEN 'browser' ELSE 'address' END AS via, ",
            ms!("h.at"),
            " AS banned_ms, count(DISTINCT h.id) AS logins, ",
            ms!("min(h.created_at)"),
            " AS first_ms, ",
            ms!("max(h.created_at)"),
            " AS last_ms FROM hits h JOIN players p ON p.id = h.player_id ",
            "GROUP BY p.display_name, p.staff_mod_level, h.display_name, h.at ",
            "ORDER BY max(h.created_at) DESC LIMIT $3"
        ))
        .bind(days)
        .bind(max_staff_level)
        .bind(limit)
        .bind(CROWDED_ADDRESS)
        .bind(CROWDED_BROWSER)
        .fetch_all(&self.pool)
        .await
    }
}
