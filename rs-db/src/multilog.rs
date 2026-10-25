use crate::links::{CROWDED_ADDRESS, CROWDED_BROWSER, CROWDED_FINGERPRINT, uncrowded};
use crate::logs::transfer;
use crate::replay::ACTIVITY_STEP_MS;
use crate::{Db, Error};
use std::collections::HashSet;

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// The pairs of logins from the same `$column` value (`uid`, `fingerprint`
/// or `ip`, one an uncrowded `$ok` CTE holds), by different accounts:
/// `$rank` says how strong that link is (see [`crate::links::link_rank`]).
macro_rules! login_pairs {
    ($column:literal, $ok:literal, $rank:literal) => {
        concat!(
            "SELECT x.id AS xi, y.id AS yi, x.player_id AS xp, y.player_id AS yp, ",
            "x.a AS xa, x.b AS xb, y.a AS ya, y.b AS yb, x.refused AS xr, y.refused AS yr, ",
            "x.live AND y.live AS live, x.node_id AS xn, y.node_id AS yn, ",
            $rank,
            " AS rank FROM s x JOIN ",
            $ok,
            " o ON o.v = x.",
            $column,
            " JOIN s y ON y.",
            $column,
            " = x.",
            $column,
            " AND y.player_id <> x.player_id"
        )
    };
}

/// How long a pickup or a shop buy is matched back to the drop or the sale
/// it took, as the economy's item sources do.
const MATCH_MINS: i32 = 10;
/// How near a shop buy is to the sale it bought back: the same shop, so the
/// same room.
const SHOP_TILES: i32 = 12;
/// A hand-off by timing alone: the dropper logs out within this long of the
/// drop, and the other account logs in within this long of that.
const HANDOFF_SECS: i32 = 180;
/// The content's `^wealth_shop_buy` and `^wealth_shop_sell`.
const SHOP_BUY: i16 = 6;
const SHOP_SELL: i16 = 7;

/// Two accounts played at the same time from the same browser, device or
/// address on one UTC day, or one tried to log in while the other was on
/// (and was refused). `name` is the account with the lower id.
#[derive(Clone, Debug, PartialEq, sqlx::FromRow)]
pub struct MultilogRecord {
    pub player_id: i64,
    pub other_id: i64,
    pub name: String,
    pub staff_mod_level: i16,
    pub created_ms: i64,
    pub other: String,
    pub other_staff_mod_level: i16,
    pub other_created_ms: i64,
    pub via: String,
    pub day_ms: i64,
    pub sessions: i64,
    pub overlap_ms: i64,
    pub attempts: i64,
    pub first_ms: i64,
    pub last_ms: i64,
    pub live: bool,
    pub nodes: Vec<i16>,
}

/// Expected moments both acted, for two people, before taking turns is
/// judged at all: fewer is too little play to tell.
const MIN_EXPECTED_BOTH: f64 = 10.0;
/// One person on two accounts acts on one at a time: both act in the same
/// moment at most this share of what two people playing would.
const TURNS_RATIO: f64 = 0.25;

/// How two accounts' play lined up while both were online, in moments of
/// [`ACTIVITY_STEP_MS`]: how many there were, in how many each chose an
/// action, and in how many both did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Turns {
    pub moments: i64,
    pub active: i64,
    pub other_active: i64,
    pub both: i64,
}

impl Turns {
    /// How many moments two people, each playing as they did, would both
    /// act in.
    pub fn expected_both(&self) -> f64 {
        if self.moments == 0 {
            return 0.0;
        }
        (self.active * self.other_active) as f64 / self.moments as f64
    }

    /// Whether they took turns: enough play to tell, and far fewer moments
    /// both acted in than two people would have.
    pub fn alternating(&self) -> bool {
        let expected = self.expected_both();
        expected >= MIN_EXPECTED_BOTH && self.both as f64 <= expected * TURNS_RATIO
    }
}

/// One recording's online and active moments, by absolute moment number.
#[derive(sqlx::FromRow)]
struct Played {
    player_id: i64,
    started_ms: i64,
    duration_ms: i32,
    activity: Vec<u8>,
}

/// The moments `played` were online and active in, between `from_ms` and
/// `to_ms`.
fn moments(
    played: &[Played],
    player: i64,
    from_ms: i64,
    to_ms: i64,
) -> (HashSet<i64>, HashSet<i64>) {
    let step = i64::from(ACTIVITY_STEP_MS);
    let (lo, hi) = (from_ms / step, to_ms / step);
    let mut online = HashSet::new();
    let mut active = HashSet::new();
    for p in played.iter().filter(|p| p.player_id == player) {
        let slots = (i64::from(p.duration_ms.max(0)) + step - 1) / step;
        for i in 0..slots {
            let at = (p.started_ms + i * step) / step;
            if at < lo || at > hi {
                continue;
            }
            online.insert(at);
            let byte = p.activity.get((i >> 3) as usize).copied().unwrap_or(0);
            if byte >> (i & 7) & 1 == 1 {
                active.insert(at);
            }
        }
    }
    (online, active)
}

/// [`Turns`] from two players' recordings.
fn turns(played: &[Played], a: i64, b: i64, from_ms: i64, to_ms: i64) -> Turns {
    let (online_a, active_a) = moments(played, a, from_ms, to_ms);
    let (online_b, active_b) = moments(played, b, from_ms, to_ms);
    let together: HashSet<i64> = online_a.intersection(&online_b).copied().collect();
    let acted_a: HashSet<i64> = active_a.intersection(&together).copied().collect();
    let acted_b: HashSet<i64> = active_b.intersection(&together).copied().collect();
    Turns {
        moments: together.len() as i64,
        active: acted_a.len() as i64,
        other_active: acted_b.len() as i64,
        both: acted_a.intersection(&acted_b).count() as i64,
    }
}

/// Items one account passed to another linked to it on one UTC day: handed
/// over (a trade, a stake, a give), dropped and picked up, or sold to a
/// shop and bought back from it.
#[derive(Clone, Debug, PartialEq, sqlx::FromRow)]
pub struct TransferRecord {
    pub from: String,
    pub from_staff_mod_level: i16,
    pub to: String,
    pub to_staff_mod_level: i16,
    pub via: String,
    pub day_ms: i64,
    pub transfers: i64,
    pub value: i64,
    pub methods: Vec<String>,
    pub swaps: i64,
    pub items: Vec<String>,
    pub counts: Vec<i64>,
    pub first_ms: i64,
    pub last_ms: i64,
}

impl Db {
    /// How players `a` and `b` (ids) took turns, or didn't, between
    /// `from_ms` and `to_ms`, from their recordings.
    pub async fn turns_between(
        &self,
        a: i64,
        b: i64,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Turns, Error> {
        let played: Vec<Played> = sqlx::query_as(concat!(
            "SELECT player_id, ",
            ms!("started_at"),
            " AS started_ms, duration_ms, activity FROM replays ",
            "WHERE player_id = ANY($1) AND activity IS NOT NULL ",
            "AND started_at < to_timestamp($3) ",
            "AND started_at + make_interval(secs => duration_ms / 1000.0) > to_timestamp($2)"
        ))
        .bind([a, b])
        .bind(from_ms as f64 / 1000.0)
        .bind(to_ms as f64 / 1000.0)
        .fetch_all(&self.pool)
        .await?;
        Ok(turns(&played, a, b, from_ms, to_ms))
    }

    /// Accounts that played at the same time, for `min_overlap_secs` or
    /// more, over the last `days`, from the same browser, device fingerprint
    /// or address (none crowded), and logins refused for it, by pair and
    /// UTC day, newest first: only `player`'s, if given. Staff above
    /// `max_staff_level` are left out.
    pub async fn multilogging(
        &self,
        player: Option<u64>,
        days: i32,
        min_overlap_secs: i32,
        max_staff_level: i16,
        limit: i64,
    ) -> Result<Vec<MultilogRecord>, Error> {
        sqlx::query_as(concat!(
            // Each session over those days, with when it ended: now if it
            // goes on, its start if it never entered the world.
            "WITH s AS (SELECT l.id, l.player_id, l.node_id, l.ip, l.uid, l.fingerprint, ",
            "l.refused IS NOT NULL AS refused, l.created_at AS a, ",
            "CASE WHEN l.ended_at IS NOT NULL THEN l.ended_at WHEN p.online_login = l.id THEN now() ",
            "ELSE l.created_at END AS b, coalesce(p.online_login = l.id, false) AS live ",
            "FROM login_logs l JOIN players p ON p.id = l.player_id ",
            "WHERE l.created_at > now() - make_interval(days => $1) AND p.staff_mod_level <= $3 ",
            "UNION ALL SELECT l.id, l.player_id, l.node_id, l.ip, l.uid, l.fingerprint, false, ",
            "l.created_at, now(), true FROM players p JOIN login_logs l ON l.id = p.online_login ",
            "WHERE p.online_node IS NOT NULL AND p.staff_mod_level <= $3 ",
            "AND l.created_at <= now() - make_interval(days => $1)), ",
            "ok_uid AS (SELECT d.v FROM (SELECT DISTINCT uid AS v FROM s WHERE uid IS NOT NULL) d WHERE ",
            uncrowded!("uid", "d.v", "$6"),
            "), ok_fp AS (SELECT d.v FROM (SELECT DISTINCT fingerprint AS v FROM s ",
            "WHERE fingerprint IS NOT NULL) d WHERE ",
            uncrowded!("fingerprint", "d.v", "$7"),
            "), ok_ip AS (SELECT d.v FROM (SELECT DISTINCT ip AS v FROM s) d WHERE ",
            uncrowded!("ip", "d.v", "$5"),
            "), pairs AS (",
            login_pairs!("uid", "ok_uid", "0"),
            " UNION ALL ",
            login_pairs!("fingerprint", "ok_fp", "1"),
            " UNION ALL ",
            login_pairs!("ip", "ok_ip", "2"),
            // Sessions that overlapped long enough, and refused logins made
            // while the other was on: each pair of logins once, through its
            // strongest link.
            "), hits AS (SELECT DISTINCT ON (least(xi, yi), greatest(xi, yi)) * FROM pairs ",
            "WHERE (NOT xr AND NOT yr AND least(xb, yb) - greatest(xa, ya) >= make_interval(secs => $2)) ",
            "OR (xr AND NOT yr AND ya <= xa AND xa <= yb) ",
            "ORDER BY least(xi, yi), greatest(xi, yi), rank), ",
            "by_day AS (SELECT least(xp, yp) AS p1, greatest(xp, yp) AS p2, ",
            ms!("date_trunc('day', greatest(xa, ya) AT TIME ZONE 'UTC')"),
            " AS day_ms, (ARRAY['browser', 'fingerprint', 'address'])[min(rank) + 1] AS via, ",
            "count(*) FILTER (WHERE NOT xr) AS sessions, ",
            "coalesce(sum(extract(epoch FROM least(xb, yb) - greatest(xa, ya)) * 1000) ",
            "FILTER (WHERE NOT xr), 0)::bigint AS overlap_ms, ",
            "count(*) FILTER (WHERE xr) AS attempts, ",
            ms!("min(greatest(xa, ya))"),
            " AS first_ms, ",
            ms!("max(CASE WHEN xr THEN xa ELSE least(xb, yb) END)"),
            " AS last_ms, bool_or(live AND NOT xr) AS live, ",
            "ARRAY(SELECT DISTINCT unnest(array_agg(xn) || array_agg(yn)) ORDER BY 1) AS nodes ",
            "FROM hits GROUP BY 1, 2, 3) ",
            "SELECT a.id AS player_id, b.id AS other_id, a.display_name AS name, a.staff_mod_level, ",
            ms!("a.created_at"),
            " AS created_ms, b.display_name AS other, b.staff_mod_level AS other_staff_mod_level, ",
            ms!("b.created_at"),
            " AS other_created_ms, d.via, d.day_ms, d.sessions, d.overlap_ms, d.attempts, ",
            "d.first_ms, d.last_ms, d.live, d.nodes FROM by_day d ",
            "JOIN players a ON a.id = d.p1 JOIN players b ON b.id = d.p2 ",
            "WHERE $8::bigint IS NULL OR $8 IN (a.name_hash, b.name_hash) ",
            "ORDER BY d.last_ms DESC LIMIT $4"
        ))
        .bind(days)
        .bind(min_overlap_secs)
        .bind(max_staff_level)
        .bind(limit)
        .bind(CROWDED_ADDRESS)
        .bind(CROWDED_BROWSER)
        .bind(CROWDED_FINGERPRINT)
        .bind(player.map(|h| h as i64))
        .fetch_all(&self.pool)
        .await
    }

    /// Items passed between linked accounts over the last `days` (see
    /// [`TransferRecord`]), by pair and UTC day, newest first, and hand-offs
    /// between unlinked ones worth `timing_value` or more: only `player`'s,
    /// if given. Staff above `max_staff_level` are left out.
    pub async fn linked_transfers(
        &self,
        player: Option<u64>,
        days: i32,
        timing_value: i64,
        max_staff_level: i16,
        limit: i64,
    ) -> Result<Vec<TransferRecord>, Error> {
        sqlx::query_as(concat!(
            "WITH moves AS (",
            // Handed over.
            "SELECT e.player_id AS from_id, e.counterpart_id AS to_id, e.node_id, e.created_at AS at, ",
            "NULL::timestamptz AS left_at, e.item, e.count::bigint AS count, e.value::bigint AS value, ",
            "CASE e.event_type WHEN $5 THEN 'trade' WHEN $6 THEN 'stake' ELSE 'give' END AS method ",
            "FROM wealth_events e WHERE e.event_type = ANY($4) AND e.counterpart_id IS NOT NULL ",
            "AND e.counterpart_id <> e.player_id AND e.created_at > now() - make_interval(days => $1) ",
            // Picked up from another's drop.
            "UNION ALL SELECT d.player_id, e.player_id, e.node_id, e.created_at, d.created_at, e.item, ",
            "e.count::bigint, e.value::bigint, 'drop' FROM wealth_events e ",
            "JOIN LATERAL (SELECT dr.player_id, dr.created_at FROM wealth_events dr WHERE dr.event_type = $7 ",
            "AND dr.node_id = e.node_id AND dr.x = e.x AND dr.z = e.z AND dr.y = e.y AND dr.item = e.item ",
            "AND dr.created_at <= e.created_at AND dr.created_at > e.created_at - make_interval(mins => $9) ",
            "ORDER BY dr.created_at DESC LIMIT 1) d ON d.player_id <> e.player_id ",
            "WHERE e.event_type = $8 AND e.created_at > now() - make_interval(days => $1) ",
            // Bought back from the shop another sold it to.
            "UNION ALL SELECT sl.player_id, e.player_id, e.node_id, e.created_at, sl.created_at, e.item, ",
            "e.count::bigint, e.value::bigint, 'shop' FROM wealth_events e ",
            "JOIN LATERAL (SELECT so.player_id, so.created_at FROM wealth_events so WHERE so.event_type = $11 ",
            "AND so.node_id = e.node_id AND so.item = e.item AND so.y = e.y ",
            "AND abs(so.x - e.x) <= $12 AND abs(so.z - e.z) <= $12 AND so.player_id <> e.player_id ",
            "AND so.created_at <= e.created_at AND so.created_at > e.created_at - make_interval(mins => $9) ",
            "ORDER BY so.created_at DESC LIMIT 1) sl ON true ",
            "WHERE e.event_type = $10 AND e.created_at > now() - make_interval(days => $1)), ",
            "pairs AS (SELECT DISTINCT m.from_id, m.to_id FROM moves m ",
            "JOIN players f ON f.id = m.from_id AND f.staff_mod_level <= $3 ",
            "JOIN players t ON t.id = m.to_id AND t.staff_mod_level <= $3), ",
            "linked AS (SELECT pr.from_id, pr.to_id, CASE ",
            "WHEN EXISTS (SELECT 1 FROM login_uids a JOIN login_uids b ON b.uid = a.uid ",
            "WHERE a.player_id = pr.from_id AND b.player_id = pr.to_id AND ",
            uncrowded!("uid", "a.uid", "$15"),
            ") THEN 'browser' ",
            "WHEN EXISTS (SELECT 1 FROM login_fingerprints a JOIN login_fingerprints b ",
            "ON b.fingerprint = a.fingerprint WHERE a.player_id = pr.from_id AND b.player_id = pr.to_id AND ",
            uncrowded!("fingerprint", "a.fingerprint", "$16"),
            ") THEN 'fingerprint' ",
            "WHEN EXISTS (SELECT 1 FROM login_ips a JOIN login_ips b ON b.ip = a.ip ",
            "WHERE a.player_id = pr.from_id AND b.player_id = pr.to_id AND ",
            uncrowded!("ip", "a.ip", "$14"),
            ") THEN 'address' END AS via FROM pairs pr), ",
            // A drop (or sale) whose dropper logged out of that world before
            // it was taken, and the taker logged in after that.
            "timed AS (SELECT m.*, k.via, o.out_at, i.in_at FROM moves m ",
            "JOIN linked k ON k.from_id = m.from_id AND k.to_id = m.to_id ",
            "LEFT JOIN LATERAL (SELECT min(lf.ended_at) AS out_at FROM login_logs lf ",
            "WHERE m.left_at IS NOT NULL AND lf.player_id = m.from_id AND lf.node_id = m.node_id ",
            "AND lf.ended_at > m.left_at AND lf.ended_at <= m.at) o ON true ",
            "LEFT JOIN LATERAL (SELECT min(lt.created_at) AS in_at FROM login_logs lt ",
            "WHERE o.out_at IS NOT NULL AND lt.player_id = m.to_id AND lt.node_id = m.node_id ",
            "AND lt.refused IS NULL AND lt.created_at >= o.out_at AND lt.created_at <= m.at) i ON true), ",
            "kept AS (SELECT t.*, coalesce(t.via, 'timing') AS link, t.in_at IS NOT NULL AS swap ",
            "FROM timed t WHERE t.via IS NOT NULL OR (t.in_at IS NOT NULL ",
            "AND t.out_at - t.left_at <= make_interval(secs => $13) ",
            "AND t.in_at - t.out_at <= make_interval(secs => $13) AND t.value >= $2)), ",
            "per_day AS (SELECT from_id, to_id, link, ",
            ms!("date_trunc('day', at AT TIME ZONE 'UTC')"),
            " AS day_ms, count(*) AS transfers, sum(value)::bigint AS value, ",
            "array_agg(DISTINCT method ORDER BY method) AS methods, count(*) FILTER (WHERE swap) AS swaps, ",
            ms!("min(at)"),
            " AS first_ms, ",
            ms!("max(at)"),
            " AS last_ms FROM kept GROUP BY 1, 2, 3, 4), ",
            "per_item AS (SELECT from_id, to_id, ",
            ms!("date_trunc('day', at AT TIME ZONE 'UTC')"),
            " AS day_ms, item, sum(count)::bigint AS count, sum(value) AS value FROM kept GROUP BY 1, 2, 3, 4) ",
            "SELECT f.display_name AS \"from\", f.staff_mod_level AS from_staff_mod_level, ",
            "t.display_name AS \"to\", t.staff_mod_level AS to_staff_mod_level, d.link AS via, d.day_ms, ",
            "d.transfers, d.value, d.methods, d.swaps, ",
            "ARRAY(SELECT pi.item FROM per_item pi WHERE pi.from_id = d.from_id AND pi.to_id = d.to_id ",
            "AND pi.day_ms = d.day_ms ORDER BY pi.value DESC, pi.item LIMIT 5) AS items, ",
            "ARRAY(SELECT pi.count FROM per_item pi WHERE pi.from_id = d.from_id AND pi.to_id = d.to_id ",
            "AND pi.day_ms = d.day_ms ORDER BY pi.value DESC, pi.item LIMIT 5) AS counts, ",
            "d.first_ms, d.last_ms FROM per_day d ",
            "JOIN players f ON f.id = d.from_id JOIN players t ON t.id = d.to_id ",
            "WHERE $18::bigint IS NULL OR $18 IN (f.name_hash, t.name_hash) ",
            "ORDER BY d.last_ms DESC LIMIT $17"
        ))
        .bind(days)
        .bind(timing_value)
        .bind(max_staff_level)
        .bind(&transfer::HANDOVERS[..])
        .bind(transfer::TRADE)
        .bind(transfer::STAKE)
        .bind(transfer::DROP)
        .bind(transfer::TAKE)
        .bind(MATCH_MINS)
        .bind(SHOP_BUY)
        .bind(SHOP_SELL)
        .bind(SHOP_TILES)
        .bind(HANDOFF_SECS)
        .bind(CROWDED_ADDRESS)
        .bind(CROWDED_BROWSER)
        .bind(CROWDED_FINGERPRINT)
        .bind(limit)
        .bind(player.map(|h| h as i64))
        .fetch_all(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: i64 = ACTIVITY_STEP_MS as i64;

    /// A recording from `start` (in moments) with activity in `active`
    /// (moments into it).
    fn played(player_id: i64, start: i64, len: i64, active: &[i64]) -> Played {
        let mut activity = vec![0u8; ((len + 7) >> 3) as usize];
        for &i in active {
            activity[(i >> 3) as usize] |= 1 << (i & 7);
        }
        Played {
            player_id,
            started_ms: start * STEP,
            duration_ms: (len * STEP) as i32,
            activity,
        }
    }

    #[test]
    fn one_person_takes_turns_two_people_dont() {
        // An hour together. One person: ten minutes on one, ten on the other.
        let len = 1200;
        let a: Vec<i64> = (0..len)
            .filter(|i| (i / 200) % 2 == 0 && i % 3 == 0)
            .collect();
        let b: Vec<i64> = (0..len)
            .filter(|i| (i / 200) % 2 == 1 && i % 3 == 0)
            .collect();
        let one = [played(1, 1000, len, &a), played(2, 1000, len, &b)];
        let t = turns(&one, 1, 2, 0, i64::MAX / 2);
        assert_eq!((t.moments, t.both), (1200, 0));
        assert!(t.alternating(), "{t:?}");

        // Two people, each acting on their own: every third and every fifth.
        let a: Vec<i64> = (0..len).filter(|i| i % 3 == 0).collect();
        let b: Vec<i64> = (0..len).filter(|i| i % 5 == 0).collect();
        let two = [played(1, 1000, len, &a), played(2, 1000, len, &b)];
        let t = turns(&two, 1, 2, 0, i64::MAX / 2);
        assert_eq!(t.both, 80);
        assert!(!t.alternating(), "{t:?}");

        // Hardly any play: too little to tell.
        let few = [played(1, 1000, len, &[1, 2]), played(2, 1000, len, &[5])];
        assert!(!turns(&few, 1, 2, 0, i64::MAX / 2).alternating());
    }

    #[test]
    fn only_moments_both_were_online_count() {
        // B joins halfway; A's play before then isn't compared.
        let a: Vec<i64> = (0..100).collect();
        let rows = [played(1, 0, 100, &a), played(2, 50, 100, &[10])];
        let t = turns(&rows, 1, 2, 0, i64::MAX / 2);
        assert_eq!(
            (t.moments, t.active, t.other_active, t.both),
            (50, 50, 1, 1)
        );
        // And only within the window asked about.
        let t = turns(&rows, 1, 2, 0, 59 * STEP);
        assert_eq!(t.moments, 10);
    }
}
