use crate::logs::transfer;
use crate::{Db, Error};
use std::collections::{HashMap, HashSet, VecDeque};

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// How long a dropped item can lie before a pickup of it is still matched
/// to the drop: longer than any drop stays on the floor.
const DROP_MATCH_MINS: i32 = 10;

/// The content's `^wealth_*` types that bring an item into the game for a
/// player: a loc pickup (5), a shop buy (6) and the party room (10).
const ORIGINS: [i16; 3] = [5, 6, 10];
/// And those that turn another item into coins: a shop sale (7), low and
/// high alchemy (8, 9).
const COIN_ORIGINS: [i16; 3] = [7, 8, 9];

/// One way a player came by some of an item: handed it by another player
/// (`event_type` a handover), picked it up (from whoever dropped it there,
/// if anyone), or from the game itself (a shop, a spawn, alchemy).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ItemSource {
    pub id: i64,
    pub event_type: i16,
    pub at_ms: i64,
    pub count: i64,
    pub value: i64,
    pub node_id: i16,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub from_hash: Option<i64>,
    pub from_name: Option<String>,
    pub from_staff_level: Option<i16>,
    pub detail: Option<String>,
}

/// A step back along where a player's item came from.
#[derive(Clone, Debug)]
pub struct TraceStep {
    pub holder: String,
    pub depth: u8,
    pub parent: Option<usize>,
    pub source: ItemSource,
}

/// An account holding an item, as of their last save.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ItemHolder {
    pub name: String,
    pub staff_level: i16,
    pub total: i64,
    pub inv_types: Vec<i32>,
    pub inv_counts: Vec<i64>,
    pub saved_ms: i64,
    pub online_node: Option<i16>,
    pub banned_until_ms: Option<i64>,
}

/// How much of an item every save holds between them.
#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct ItemTotal {
    pub obj: i32,
    pub total: i64,
    pub holders: i64,
}

/// An account on a trade graph.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct GraphAccount {
    pub name: String,
    pub staff_level: i16,
    pub online_node: Option<i16>,
    pub banned_until_ms: Option<i64>,
}

/// Everything handed over one way, from one player to another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeEdge {
    pub from: String,
    pub to: String,
    pub value: i64,
    pub handovers: i64,
    pub last_ms: i64,
    pub items: Vec<(String, i64)>,
}

/// Who a player hands items to and gets them from, and what passes between
/// those players too.
#[derive(Clone, Debug)]
pub struct TradeGraph {
    pub accounts: Vec<GraphAccount>,
    pub edges: Vec<TradeEdge>,
}

/// An account others hand items to.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct TopReceiver {
    pub name: String,
    pub staff_level: i16,
    pub givers: i64,
    pub handovers: i64,
    pub received: i64,
    pub given: i64,
    pub last_ms: i64,
    pub online_node: Option<i16>,
    pub banned_until_ms: Option<i64>,
}

/// Unix ms as the seconds `to_timestamp` takes.
fn secs(ms: i64) -> f64 {
    ms as f64 / 1000.0
}

/// The most items an edge lists.
const EDGE_ITEMS: usize = 5;

impl Db {
    /// How the player came by an item before `before_ms` (now if `None`),
    /// newest first. `items` are its forms' debug names (an item and its
    /// noted form, say).
    pub async fn item_sources(
        &self,
        name_hash: u64,
        items: &[String],
        before_ms: Option<i64>,
        limit: i64,
    ) -> Result<Vec<ItemSource>, Error> {
        sqlx::query_as(concat!(
            "WITH me AS (SELECT id FROM players WHERE name_hash = $1), ",
            "b AS (SELECT coalesce(to_timestamp($3::float8), now()) AS t) ",
            "SELECT * FROM (",
            // Handed to them.
            "(SELECT e.id, e.event_type, ",
            ms!("e.created_at"),
            " AS at_ms, e.count::bigint, e.value::bigint, e.node_id, e.x, e.z, e.y, ",
            "g.name_hash AS from_hash, g.display_name AS from_name, ",
            "g.staff_mod_level AS from_staff_level, NULL::text AS detail ",
            "FROM wealth_events e JOIN players g ON g.id = e.player_id ",
            "WHERE e.counterpart_id = (SELECT id FROM me) AND e.item = ANY($2) ",
            "AND e.event_type = ANY($5) AND e.created_at < (SELECT t FROM b) ",
            "ORDER BY e.created_at DESC LIMIT $4) ",
            // Picked up, with the drop it was if one's logged.
            "UNION ALL (SELECT e.id, e.event_type, ",
            ms!("e.created_at"),
            ", e.count::bigint, e.value::bigint, e.node_id, e.x, e.z, e.y, ",
            "d.name_hash, d.display_name, d.staff_mod_level, NULL::text FROM wealth_events e ",
            "LEFT JOIN LATERAL (SELECT dp.name_hash, dp.display_name, dp.staff_mod_level ",
            "FROM wealth_events dr ",
            "JOIN players dp ON dp.id = dr.player_id WHERE dr.event_type = $6 ",
            "AND dr.node_id = e.node_id AND dr.x = e.x AND dr.z = e.z AND dr.y = e.y ",
            "AND dr.item = e.item AND dr.created_at <= e.created_at ",
            "AND dr.created_at > e.created_at - make_interval(mins => $7) ",
            "ORDER BY dr.created_at DESC LIMIT 1) d ON true ",
            "WHERE e.player_id = (SELECT id FROM me) AND e.event_type = $8 AND e.item = ANY($2) ",
            "AND e.created_at < (SELECT t FROM b) ORDER BY e.created_at DESC LIMIT $4) ",
            // From the game: for coins, what was sold or alched into them.
            "UNION ALL (SELECT e.id, e.event_type, ",
            ms!("e.created_at"),
            ", CASE WHEN e.item = ANY($2) THEN e.count ELSE e.value END::bigint, e.value::bigint, ",
            "e.node_id, e.x, e.z, e.y, NULL::bigint, NULL::text, NULL::smallint, ",
            "CASE WHEN NOT e.item = ANY($2) THEN e.item END FROM wealth_events e ",
            "WHERE e.player_id = (SELECT id FROM me) AND e.created_at < (SELECT t FROM b) ",
            "AND ((e.item = ANY($2) AND e.event_type = ANY($9)) ",
            "OR ('coins' = ANY($2) AND e.event_type = ANY($10))) ",
            "ORDER BY e.created_at DESC LIMIT $4)",
            ") s ORDER BY at_ms DESC, id DESC LIMIT $4"
        ))
        .bind(name_hash as i64)
        .bind(items)
        .bind(before_ms.map(secs))
        .bind(limit)
        .bind(&transfer::HANDOVERS[..])
        .bind(transfer::DROP)
        .bind(DROP_MATCH_MINS)
        .bind(transfer::TAKE)
        .bind(&ORIGINS[..])
        .bind(&COIN_ORIGINS[..])
        .fetch_all(&self.pool)
        .await
    }

    /// Where the player's item (`items`, its forms) came from, step by step
    /// back through the
    /// players it passed through: each holder's newest `per_step` sources
    /// from before they handed it on, up to `depth` holders back and
    /// `max_steps` in all. Each player is followed back once, and staff
    /// above `max_staff_level` not at all. Empty when they aren't
    /// registered.
    pub async fn item_trace(
        &self,
        name_hash: u64,
        items: &[String],
        depth: u8,
        per_step: i64,
        max_steps: usize,
        max_staff_level: Option<i16>,
    ) -> Result<Vec<TraceStep>, Error> {
        let name: Option<String> =
            sqlx::query_scalar("SELECT display_name FROM players WHERE name_hash = $1")
                .bind(name_hash as i64)
                .fetch_optional(&self.pool)
                .await?;
        let Some(name) = name else {
            return Ok(Vec::new());
        };
        let mut steps: Vec<TraceStep> = Vec::new();
        let mut followed = HashSet::from([name_hash]);
        let mut queue = VecDeque::from([(name_hash, name, None, 0u8, None)]);
        while let Some((holder_hash, holder, before, level, parent)) = queue.pop_front() {
            for source in self
                .item_sources(holder_hash, items, before, per_step)
                .await?
            {
                if steps.len() >= max_steps {
                    return Ok(steps);
                }
                let from = source
                    .from_hash
                    .map(|h| h as u64)
                    .zip(source.from_name.clone())
                    .filter(|_| {
                        max_staff_level
                            .is_none_or(|max| source.from_staff_level.unwrap_or(0) <= max)
                    })
                    .filter(|&(h, _)| level + 1 < depth && followed.insert(h));
                let at_ms = source.at_ms;
                steps.push(TraceStep {
                    holder: holder.clone(),
                    depth: level,
                    parent,
                    source,
                });
                if let Some((h, from_name)) = from {
                    queue.push_back((h, from_name, Some(at_ms), level + 1, Some(steps.len() - 1)));
                }
            }
        }
        Ok(steps)
    }

    /// The accounts whose saves hold at least `min_count` of the items
    /// `objs` between them (an item and its noted form, say), the most first.
    pub async fn item_holders(
        &self,
        objs: &[i32],
        min_count: i64,
        limit: i64,
    ) -> Result<Vec<ItemHolder>, Error> {
        sqlx::query_as(concat!(
            "WITH held AS (SELECT s.player_id, i.inv, sum(i.count::bigint)::bigint AS count ",
            "FROM player_saves s CROSS JOIN LATERAL ",
            "unnest(s.inv_types, s.inv_objs, s.inv_counts) AS i (inv, obj, count) ",
            // `&&` first, along the index on the saves' items.
            "WHERE s.inv_objs && $1 AND i.obj = ANY($1) GROUP BY s.player_id, i.inv), ",
            "totals AS (SELECT player_id, sum(count)::bigint AS total, ",
            "array_agg(inv ORDER BY count DESC) AS inv_types, ",
            "array_agg(count ORDER BY count DESC) AS inv_counts FROM held ",
            "GROUP BY player_id HAVING sum(count) >= $2) ",
            "SELECT p.display_name AS name, p.staff_mod_level AS staff_level, t.total, ",
            "t.inv_types, t.inv_counts, ",
            ms!("s.updated_at"),
            " AS saved_ms, p.online_node, CASE WHEN p.banned_until > now() THEN ",
            ms!("p.banned_until"),
            " END AS banned_until_ms FROM totals t JOIN players p ON p.id = t.player_id ",
            "JOIN player_saves s ON s.player_id = t.player_id ",
            "ORDER BY t.total DESC, p.display_name LIMIT $3"
        ))
        .bind(objs)
        .bind(min_count.max(1))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// How many of each of `objs` every save holds, and in how many saves.
    /// Items nobody holds are left out.
    pub async fn item_totals(&self, objs: &[i32]) -> Result<Vec<ItemTotal>, Error> {
        sqlx::query_as(
            "SELECT i.obj, sum(i.count::bigint)::bigint AS total,
                    count(DISTINCT s.player_id) AS holders
             FROM player_saves s
             CROSS JOIN LATERAL unnest(s.inv_objs, s.inv_counts) AS i (obj, count)
             WHERE s.inv_objs && $1 AND i.obj = ANY($1)
             GROUP BY i.obj ORDER BY i.obj",
        )
        .bind(objs)
        .fetch_all(&self.pool)
        .await
    }

    /// Who the player handed items to or got them from since `since_ms`,
    /// the `partners` they traded the most value with, and everything handed
    /// over between any of them. Empty when they aren't registered.
    pub async fn trade_graph(
        &self,
        name_hash: u64,
        since_ms: i64,
        partners: i64,
    ) -> Result<TradeGraph, Error> {
        let ids: Vec<i64> = sqlx::query_scalar(
            "WITH me AS (SELECT id FROM players WHERE name_hash = $1),
             h AS (SELECT CASE WHEN e.player_id = (SELECT id FROM me)
                               THEN e.counterpart_id ELSE e.player_id END AS other,
                          e.value::bigint AS value
                   FROM wealth_events e
                   WHERE e.event_type = ANY($2)
                     AND e.created_at > to_timestamp($3::float8)
                     AND (e.player_id = (SELECT id FROM me)
                          OR e.counterpart_id = (SELECT id FROM me)))
             SELECT id FROM me
             UNION ALL
             (SELECT other FROM h WHERE other IS NOT NULL
              GROUP BY other ORDER BY sum(value) DESC, other LIMIT $4)",
        )
        .bind(name_hash as i64)
        .bind(&transfer::HANDOVERS[..])
        .bind(secs(since_ms))
        .bind(partners)
        .fetch_all(&self.pool)
        .await?;
        if ids.is_empty() {
            return Ok(TradeGraph {
                accounts: Vec::new(),
                edges: Vec::new(),
            });
        }
        let accounts: Vec<GraphAccount> = sqlx::query_as(concat!(
            "SELECT display_name AS name, staff_mod_level AS staff_level, online_node, ",
            "CASE WHEN banned_until > now() THEN ",
            ms!("banned_until"),
            " END AS banned_until_ms FROM players WHERE id = ANY($1) ",
            "ORDER BY array_position($1, id)"
        ))
        .bind(&ids)
        .fetch_all(&self.pool)
        .await?;
        let rows: Vec<EdgeRow> = sqlx::query_as(concat!(
            "SELECT g.display_name, t.display_name, e.item, sum(e.value::bigint)::bigint, ",
            // A handover's items are logged together, at one time.
            "sum(e.count::bigint)::bigint, ",
            "array_agg(DISTINCT (extract(epoch FROM e.created_at) * 1000000)::bigint), ",
            ms!("max(e.created_at)"),
            " FROM wealth_events e JOIN players g ON g.id = e.player_id ",
            "JOIN players t ON t.id = e.counterpart_id ",
            "WHERE e.event_type = ANY($2) ",
            "AND e.created_at > to_timestamp($3::float8) ",
            "AND e.player_id = ANY($1) AND e.counterpart_id = ANY($1) ",
            "GROUP BY g.display_name, t.display_name, e.item"
        ))
        .bind(&ids)
        .bind(&transfer::HANDOVERS[..])
        .bind(secs(since_ms))
        .fetch_all(&self.pool)
        .await?;
        Ok(TradeGraph {
            accounts,
            edges: edges(rows),
        })
    }

    /// The accounts at least `min_givers` players handed items to since
    /// `since_ms`, those taking in the most more than they gave first: one
    /// account many feed and that feeds nobody back is a mule.
    pub async fn top_receivers(
        &self,
        since_ms: i64,
        min_givers: i64,
        limit: i64,
    ) -> Result<Vec<TopReceiver>, Error> {
        sqlx::query_as(concat!(
            "WITH h AS (SELECT player_id, counterpart_id, value::bigint AS value, created_at ",
            "FROM wealth_events WHERE counterpart_id IS NOT NULL AND event_type = ANY($1) ",
            "AND created_at > to_timestamp($2::float8)), ",
            "inflow AS (SELECT counterpart_id AS id, count(DISTINCT player_id) AS givers, ",
            // A handover's items are logged together, at one time.
            "count(DISTINCT (player_id, created_at)) AS handovers, ",
            "sum(value)::bigint AS received, max(created_at) AS last ",
            "FROM h GROUP BY counterpart_id HAVING count(DISTINCT player_id) >= $3), ",
            "outflow AS (SELECT player_id AS id, sum(value)::bigint AS given FROM h ",
            "GROUP BY player_id) ",
            "SELECT p.display_name AS name, p.staff_mod_level AS staff_level, i.givers, ",
            "i.handovers, i.received, coalesce(o.given, 0)::bigint AS given, ",
            ms!("i.last"),
            " AS last_ms, p.online_node, CASE WHEN p.banned_until > now() THEN ",
            ms!("p.banned_until"),
            " END AS banned_until_ms FROM inflow i JOIN players p ON p.id = i.id ",
            "LEFT JOIN outflow o ON o.id = i.id ",
            "ORDER BY i.received - coalesce(o.given, 0) DESC, p.display_name LIMIT $4"
        ))
        .bind(&transfer::HANDOVERS[..])
        .bind(secs(since_ms))
        .bind(min_givers.max(1))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}

/// What one player handed another of one item: (from, to, item, value,
/// count, when each handover was in microseconds, last).
type EdgeRow = (String, String, String, i64, i64, Vec<i64>, i64);
/// An item on an edge: (item, value, count).
type EdgeItem = (String, i64, i64);
/// An edge being folded: its items, and when each handover was.
type Folding = (TradeEdge, Vec<EdgeItem>, HashSet<i64>);

/// Rows folded into one edge per direction, the most valuable first.
fn edges(rows: Vec<EdgeRow>) -> Vec<TradeEdge> {
    let mut by_pair: HashMap<(String, String), Folding> = HashMap::new();
    for (from, to, item, value, count, times, last_ms) in rows {
        let (edge, items, handovers) =
            by_pair
                .entry((from.clone(), to.clone()))
                .or_insert_with(|| {
                    (
                        TradeEdge {
                            from,
                            to,
                            value: 0,
                            handovers: 0,
                            last_ms,
                            items: Vec::new(),
                        },
                        Vec::new(),
                        HashSet::new(),
                    )
                });
        edge.value += value;
        // One handover of several items counts once.
        handovers.extend(times);
        edge.last_ms = edge.last_ms.max(last_ms);
        items.push((item, value, count));
    }
    let mut edges: Vec<TradeEdge> = by_pair
        .into_values()
        .map(|(mut edge, mut items, handovers)| {
            edge.handovers = handovers.len() as i64;
            items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            edge.items = items
                .into_iter()
                .take(EDGE_ITEMS)
                .map(|(item, _, count)| (item, count))
                .collect();
            edge
        })
        .collect();
    edges.sort_by(|a, b| {
        b.value
            .cmp(&a.value)
            .then_with(|| (&a.from, &a.to).cmp(&(&b.from, &b.to)))
    });
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_fold_items_per_direction() {
        let row = |from: &str, to: &str, item: &str, value, count, times: &[i64]| {
            (
                from.to_owned(),
                to.to_owned(),
                item.to_owned(),
                value,
                count,
                times.to_vec(),
                value,
            )
        };
        // A's coins and axe went over in one trade, then more coins.
        let edges = edges(vec![
            row("A", "B", "coins", 500, 500, &[1, 2]),
            row("A", "B", "rune_axe", 900, 1, &[1]),
            row("B", "A", "lobster", 10, 5, &[3]),
        ]);
        assert_eq!(edges.len(), 2);
        assert_eq!(
            (edges[0].from.as_str(), edges[0].value, edges[0].handovers),
            ("A", 1400, 2)
        );
        assert_eq!(
            edges[0].items,
            [("rune_axe".to_owned(), 1), ("coins".to_owned(), 500)]
        );
        assert_eq!(edges[0].last_ms, 900);
        assert_eq!(edges[1].from, "B");
    }
}
