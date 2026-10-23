use crate::{Db, Error};

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// A script error as staff see it: every row of it, summed.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ScriptErrorGroup {
    pub script: String,
    pub line: i32,
    pub message: String,
    pub file: String,
    pub trace: String,
    pub count: i64,
    pub first_ms: i64,
    pub last_ms: i64,
    pub players: i64,
    pub npcs: Vec<String>,
    pub nodes: Vec<i16>,
}

/// One row of a script error: whom it happened to, how often, and where
/// the first was.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ScriptErrorRecord {
    pub node_id: i16,
    pub first_ms: i64,
    pub last_ms: i64,
    pub count: i32,
    pub player: Option<String>,
    pub npc: Option<String>,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub replay_id: Option<i64>,
    pub replay_ms: Option<i32>,
    pub trace: String,
}

impl Db {
    /// The script errors seen since `since_ms` (Unix ms), the latest first.
    pub async fn script_error_groups(
        &self,
        since_ms: i64,
        limit: i64,
    ) -> Result<Vec<ScriptErrorGroup>, Error> {
        // The file and trace are the latest row's, looked up only for the
        // groups shown.
        let sql = concat!(
            "SELECT g.script, g.line, g.message, l.file, l.trace, g.count, ",
            ms!("g.first_at"),
            " AS first_ms, ",
            ms!("g.last_at"),
            " AS last_ms, ",
            "g.players, g.npcs, g.nodes FROM (",
            "SELECT script, line, message, sum(count)::bigint AS count, ",
            "min(first_at) AS first_at, max(last_at) AS last_at, ",
            "count(DISTINCT player_id) AS players, ",
            "coalesce(array_agg(DISTINCT npc) FILTER (WHERE npc IS NOT NULL), '{}') AS npcs, ",
            "array_agg(DISTINCT node_id) AS nodes ",
            "FROM script_errors WHERE last_at >= to_timestamp($1 / 1000.0) ",
            "GROUP BY script, line, message ORDER BY max(last_at) DESC LIMIT $2) g ",
            "CROSS JOIN LATERAL (SELECT file, trace FROM script_errors e ",
            "WHERE e.script = g.script AND e.line = g.line AND e.message = g.message ",
            "ORDER BY e.last_at DESC, e.id DESC LIMIT 1) l ",
            "ORDER BY g.last_at DESC"
        );
        sqlx::query_as(sql)
            .bind(since_ms)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// The rows of one script error, the latest first.
    pub async fn script_error_records(
        &self,
        script: &str,
        line: i32,
        message: &str,
        limit: i64,
    ) -> Result<Vec<ScriptErrorRecord>, Error> {
        let sql = concat!(
            "SELECT e.node_id, ",
            ms!("e.first_at"),
            " AS first_ms, ",
            ms!("e.last_at"),
            " AS last_ms, ",
            "e.count, p.display_name AS player, e.npc, e.x, e.z, e.y, e.replay_id, e.replay_ms, ",
            "e.trace FROM script_errors e LEFT JOIN players p ON p.id = e.player_id ",
            "WHERE e.script = $1 AND e.line = $2 AND e.message = $3 ",
            "ORDER BY e.last_at DESC, e.id DESC LIMIT $4"
        );
        sqlx::query_as(sql)
            .bind(script)
            .bind(line)
            .bind(message)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// Deletes script errors last seen over `days` ago.
    pub async fn prune_script_errors(&self, days: i32) -> Result<(), Error> {
        sqlx::query("DELETE FROM script_errors WHERE last_at < now() - make_interval(days => $1)")
            .bind(days)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
