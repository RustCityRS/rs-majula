use crate::{Db, Error};
use rs_stat::get_level_by_exp;
use serde::Serialize;
use sqlx::PgConnection;

/// The hiscore tables, in display order: 0 is Overall, any other is a stat id
/// + 1. Stats 18 and 19 are unused, so there are no tables 19 and 20.
pub const HISCORE_CATEGORIES: [i16; 20] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 21,
];

/// A row of a hiscore table.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct HiscoreEntry {
    pub rank: i64,
    pub name: String,
    pub level: i16,
    /// Real XP (the save stores 10x).
    pub xp: i64,
}

/// A player's standing in one hiscore table.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct HiscoreRank {
    pub category: i16,
    pub rank: i64,
    pub level: i16,
    /// Real XP (the save stores 10x).
    pub xp: i64,
}

/// A player's standing in every table they are ranked in.
#[derive(Clone, Debug, Serialize)]
pub struct PlayerHiscores {
    pub name: String,
    pub ranks: Vec<HiscoreRank>,
}

impl PlayerHiscores {
    pub fn rank(&self, category: i16) -> Option<&HiscoreRank> {
        self.ranks.iter().find(|r| r.category == category)
    }
}

impl Db {
    /// `limit` rows of table `category`, starting after the first `offset`.
    pub async fn hiscores_page(
        &self,
        category: i16,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<HiscoreEntry>, Error> {
        let rows: Vec<(String, i16, i64)> = sqlx::query_as(
            "SELECT display_name, level, xp / 10 FROM ranked_hiscores
             WHERE category = $1
             ORDER BY -level, -xp, reached_at, player_id
             OFFSET $2 LIMIT $3",
        )
        .bind(category)
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .zip(offset + 1..)
            .map(|((name, level, xp), rank)| HiscoreEntry {
                rank,
                name,
                level,
                xp,
            })
            .collect())
    }

    /// How many players table `category` ranks.
    pub async fn hiscores_count(&self, category: i16) -> Result<i64, Error> {
        sqlx::query_scalar("SELECT count(*) FROM ranked_hiscores WHERE category = $1")
            .bind(category)
            .fetch_one(&self.pool)
            .await
    }

    /// A player's ranks by base37 name. `None` if they aren't on the
    /// hiscores at all.
    pub async fn player_hiscores(&self, name_hash: u64) -> Result<Option<PlayerHiscores>, Error> {
        let rows: Vec<(String, i16, i64, i16, i64)> = sqlx::query_as(
            "SELECT me.display_name, me.category, 1 + (
                        SELECT count(*) FROM ranked_hiscores o
                        WHERE o.category = me.category
                          AND (-o.level, -o.xp, o.reached_at, o.player_id)
                            < (-me.level, -me.xp, me.reached_at, me.player_id)
                    ), me.level, me.xp / 10
             FROM ranked_hiscores me
             WHERE me.name_hash = $1
             ORDER BY me.category",
        )
        .bind(name_hash as i64)
        .fetch_all(&self.pool)
        .await?;
        let Some(name) = rows.first().map(|row| row.0.clone()) else {
            return Ok(None);
        };
        let ranks = rows
            .into_iter()
            .map(|(_, category, rank, level, xp)| HiscoreRank {
                category,
                rank,
                level,
                xp,
            })
            .collect();
        Ok(Some(PlayerHiscores { name, ranks }))
    }
}

/// Rewrites a player's hiscore rows from their saved stats. A row whose XP
/// hasn't changed is left alone, so it keeps its `reached_at`.
pub(crate) async fn update_hiscores(
    tx: &mut PgConnection,
    player_id: i64,
    stats: &[i32],
) -> Result<(), Error> {
    let mut levels = vec![0i16; HISCORE_CATEGORIES.len()];
    let mut xps = vec![0i64; HISCORE_CATEGORIES.len()];
    for (i, &category) in HISCORE_CATEGORIES.iter().enumerate().skip(1) {
        let xp = stats
            .get(category as usize - 1)
            .copied()
            .unwrap_or(0)
            .max(0);
        levels[i] = get_level_by_exp(xp) as i16;
        xps[i] = xp as i64;
        levels[0] += levels[i];
        xps[0] += xps[i];
    }
    sqlx::query(
        "INSERT INTO hiscores (player_id, category, level, xp)
         SELECT $1, * FROM unnest($2::smallint[], $3::smallint[], $4::bigint[])
         ON CONFLICT (player_id, category) DO UPDATE
             SET level = EXCLUDED.level, xp = EXCLUDED.xp, reached_at = now()
             WHERE hiscores.xp <> EXCLUDED.xp",
    )
    .bind(player_id)
    .bind(&HISCORE_CATEGORIES[..])
    .bind(&levels)
    .bind(&xps)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rs_stat::get_exp_by_level;

    /// The migration's backfill carries its own copy of the XP table.
    #[test]
    fn backfill_thresholds_match_rs_stat() {
        let sql = include_str!("../migrations/0005_hiscores.sql");
        let start = sql.find("unnest('{").expect("thresholds") + "unnest('{".len();
        let end = start + sql[start..].find('}').expect("end of thresholds");
        let thresholds: Vec<i32> = sql[start..end]
            .split(',')
            .map(|v| v.trim().parse().expect("threshold"))
            .collect();
        let expected: Vec<i32> = (1..=99).map(get_exp_by_level).collect();
        assert_eq!(thresholds, expected);
    }
}
