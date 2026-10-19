use crate::{Db, Error};

/// A public or private chat message to log.
#[derive(Debug)]
pub struct ChatLog {
    pub name_hash: u64,
    /// The recipient of a private message, `None` for public chat.
    pub to_name_hash: Option<u64>,
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub message: String,
}

impl Db {
    /// Logs chat sent on world `node_id`, in order. Messages from players who
    /// aren't registered are skipped.
    pub async fn record_chat(&self, node_id: i16, logs: &[ChatLog]) -> Result<(), Error> {
        let name_hashes: Vec<i64> = logs.iter().map(|l| l.name_hash as i64).collect();
        let to_name_hashes: Vec<Option<i64>> = logs
            .iter()
            .map(|l| l.to_name_hash.map(|h| h as i64))
            .collect();
        let xs: Vec<i16> = logs.iter().map(|l| l.x).collect();
        let zs: Vec<i16> = logs.iter().map(|l| l.z).collect();
        let ys: Vec<i16> = logs.iter().map(|l| l.y).collect();
        let messages: Vec<&str> = logs.iter().map(|l| l.message.as_str()).collect();
        sqlx::query(
            "INSERT INTO chat_logs (player_id, to_name_hash, node_id, x, z, y, message)
             SELECT p.id, c.to_name_hash, $1, c.x, c.z, c.y, c.message
             FROM unnest($2::bigint[], $3::bigint[], $4::smallint[], $5::smallint[],
                         $6::smallint[], $7::text[])
                  WITH ORDINALITY AS c (name_hash, to_name_hash, x, z, y, message, ord)
             JOIN players p ON p.name_hash = c.name_hash
             ORDER BY c.ord",
        )
        .bind(node_id)
        .bind(&name_hashes)
        .bind(&to_name_hashes)
        .bind(&xs)
        .bind(&zs)
        .bind(&ys)
        .bind(&messages)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
