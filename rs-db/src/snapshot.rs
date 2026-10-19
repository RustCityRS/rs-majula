use crate::{Db, Error};

/// An abuse report snapshot to record.
#[derive(Debug)]
pub struct NewSnapshot {
    pub reporter_name_hash: u64,
    pub offender_name_hash: u64,
    pub reason: i16,
    pub node_id: i16,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub muted: bool,
}

impl Db {
    /// Records an abuse report snapshot. Returns `false` if the reporter isn't a
    /// registered player.
    pub async fn record_snapshot(&self, snapshot: &NewSnapshot) -> Result<bool, Error> {
        let result = sqlx::query(
            "INSERT INTO snapshots
                 (reporter_id, offender_name_hash, reason, node_id, x, z, y, muted)
             SELECT id, $2, $3, $4, $5, $6, $7, $8 FROM players WHERE name_hash = $1",
        )
        .bind(snapshot.reporter_name_hash as i64)
        .bind(snapshot.offender_name_hash as i64)
        .bind(snapshot.reason)
        .bind(snapshot.node_id)
        .bind(snapshot.x)
        .bind(snapshot.z)
        .bind(snapshot.y)
        .bind(snapshot.muted)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
