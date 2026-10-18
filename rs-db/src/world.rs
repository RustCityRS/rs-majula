use crate::{Db, Error};

/// A world's latest heartbeat.
#[derive(Clone, Debug, PartialEq, sqlx::FromRow)]
pub struct WorldStatus {
    pub node_id: i16,
    pub players: i32,
}

impl Db {
    /// Records that the world `node_id` is up with `players` online.
    pub async fn report_world_status(&self, node_id: i16, players: i32) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO world_status (node_id, players) VALUES ($1, $2)
             ON CONFLICT (node_id) DO UPDATE
                 SET players = EXCLUDED.players, updated_at = now()",
        )
        .bind(node_id)
        .bind(players)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The worlds that reported within the last `max_age_secs`, judged by the
    /// database's clock so the worlds' own clocks don't matter.
    pub async fn live_worlds(&self, max_age_secs: i32) -> Result<Vec<WorldStatus>, Error> {
        sqlx::query_as(
            "SELECT node_id, players FROM world_status
             WHERE updated_at > now() - make_interval(secs => $1)",
        )
        .bind(max_age_secs)
        .fetch_all(&self.pool)
        .await
    }
}
