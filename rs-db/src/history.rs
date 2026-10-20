use crate::save::PlayerSave;
use crate::{Db, Error};
use sqlx::PgConnection;

/// A timestamp column as Unix milliseconds.
macro_rules! ms {
    ($column:literal) => {
        concat!("(extract(epoch FROM ", $column, ") * 1000)::bigint")
    };
}

/// A save's columns, in `player_saves` and `save_history` alike.
macro_rules! columns {
    () => {
        "x, z, y, body, colors, gender, runenergy, playtime, stats, levels, \
         afk_zones, last_afk_zone, public_chat, private_chat, trade_chat, \
         last_date, varp_ids, varp_values, inv_types, inv_slots, inv_objs, inv_counts"
    };
}

/// The least time between two points kept at logout, and between two kept
/// through a session. Within it the older point stays: it's from before
/// whatever the player did since, which a rollback wants.
const LOGOUT_GAP_MINS: i32 = 15;
const SESSION_GAP_MINS: i32 = 60;

/// One earlier copy of a player's save.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct SavePoint {
    pub id: i64,
    pub kind: String,
    pub node_id: i16,
    pub note: Option<String>,
    pub saved_ms: i64,
}

/// Keeps the save just stored for `player_id` as a point, unless an
/// automatic point was kept less than `gap_mins` ago.
async fn keep_point(
    conn: &mut PgConnection,
    player_id: i64,
    node_id: i16,
    kind: &str,
    gap_mins: i32,
) -> Result<(), Error> {
    sqlx::query(concat!(
        "INSERT INTO save_history (player_id, kind, node_id, ",
        columns!(),
        ") SELECT player_id, $2, $3, ",
        columns!(),
        " FROM player_saves WHERE player_id = $1 AND NOT EXISTS (
             SELECT 1 FROM save_history
             WHERE player_id = $1 AND kind <> 'staff'
               AND saved_at > now() - make_interval(mins => $4))"
    ))
    .bind(player_id)
    .bind(kind)
    .bind(node_id)
    .bind(gap_mins)
    .execute(conn)
    .await?;
    Ok(())
}

/// After a save through a session.
pub(crate) async fn keep_session_point(
    conn: &mut PgConnection,
    player_id: i64,
    node_id: i16,
) -> Result<(), Error> {
    keep_point(conn, player_id, node_id, "session", SESSION_GAP_MINS).await
}

/// After a logout's save.
pub(crate) async fn keep_logout_point(
    conn: &mut PgConnection,
    player_id: i64,
    node_id: i16,
) -> Result<(), Error> {
    keep_point(conn, player_id, node_id, "logout", LOGOUT_GAP_MINS).await
}

/// Before staff overwrite `player_id`'s save: keeps it as it is, if they
/// have one, so the change can be undone.
pub(crate) async fn keep_staff_point(
    conn: &mut PgConnection,
    player_id: i64,
    node_id: i16,
    note: &str,
) -> Result<(), Error> {
    sqlx::query(concat!(
        "INSERT INTO save_history (player_id, kind, node_id, note, ",
        columns!(),
        ") SELECT player_id, 'staff', $2, $3, ",
        columns!(),
        " FROM player_saves WHERE player_id = $1"
    ))
    .bind(player_id)
    .bind(node_id)
    .bind(note)
    .execute(conn)
    .await?;
    Ok(())
}

impl Db {
    /// A player's save points, newest first, older than point `before` when
    /// paging.
    pub async fn save_points(
        &self,
        name_hash: u64,
        before: Option<i64>,
        limit: i64,
    ) -> Result<Vec<SavePoint>, Error> {
        sqlx::query_as(concat!(
            "SELECT h.id, h.kind, h.node_id, h.note, ",
            ms!("h.saved_at"),
            " AS saved_ms FROM save_history h
             JOIN players p ON p.id = h.player_id
             WHERE p.name_hash = $1 AND ($2::bigint IS NULL OR h.id < $2)
             ORDER BY h.id DESC LIMIT $3"
        ))
        .bind(name_hash as i64)
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// One of a player's save points, and the save it kept. `None` if it
    /// isn't theirs (or was pruned).
    pub async fn save_point(
        &self,
        name_hash: u64,
        id: i64,
    ) -> Result<Option<(SavePoint, PlayerSave)>, Error> {
        let point: Option<SavePoint> = sqlx::query_as(concat!(
            "SELECT h.id, h.kind, h.node_id, h.note, ",
            ms!("h.saved_at"),
            " AS saved_ms FROM save_history h
             JOIN players p ON p.id = h.player_id
             WHERE p.name_hash = $1 AND h.id = $2"
        ))
        .bind(name_hash as i64)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(point) = point else {
            return Ok(None);
        };
        let save: PlayerSave = sqlx::query_as(concat!(
            "SELECT ",
            columns!(),
            " FROM save_history WHERE id = $1"
        ))
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(Some((point, save)))
    }

    /// Forgets save points older than `days`, but never a player's newest
    /// `keep`.
    pub async fn prune_save_history(&self, days: i32, keep: i64) -> Result<u64, Error> {
        let result = sqlx::query(
            "DELETE FROM save_history h
             WHERE h.saved_at < now() - make_interval(days => $1)
               AND (SELECT count(*) FROM save_history n
                    WHERE n.player_id = h.player_id AND n.id > h.id) >= $2",
        )
        .bind(days)
        .bind(keep)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}
