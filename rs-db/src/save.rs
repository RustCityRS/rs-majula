use crate::{Db, Error};

#[derive(Clone, Debug, Default, PartialEq, sqlx::FromRow)]
pub struct PlayerSave {
    pub x: i16,
    pub z: i16,
    pub y: i16,
    pub body: Vec<i16>,
    pub colors: Vec<i16>,
    pub gender: i16,
    pub runenergy: i32,
    pub playtime: i32,
    pub stats: Vec<i32>,
    pub levels: Vec<i16>,
    pub afk_zones: Vec<i32>,
    pub last_afk_zone: i16,
    pub public_chat: i16,
    pub private_chat: i16,
    pub trade_chat: i16,
    pub last_date: i64,
    pub varp_ids: Vec<i32>,
    pub varp_values: Vec<i32>,
    pub inv_types: Vec<i32>,
    pub inv_slots: Vec<i16>,
    pub inv_objs: Vec<i32>,
    pub inv_counts: Vec<i32>,
}

impl Db {
    /// Loads a player's save by base37 name. `None` if the player doesn't
    /// exist or has never been saved.
    pub async fn load_save(&self, name_hash: u64) -> Result<Option<PlayerSave>, Error> {
        sqlx::query_as(
            "SELECT s.* FROM player_saves s
             JOIN players p ON p.id = s.player_id
             WHERE p.name_hash = $1",
        )
        .bind(name_hash as i64)
        .fetch_optional(&self.pool)
        .await
    }

    /// Writes a player's save, creating the row on first save. Returns `false`
    /// if no player with that name exists.
    pub async fn store_save(&self, name_hash: u64, save: &PlayerSave) -> Result<bool, Error> {
        let result = sqlx::query(
            "INSERT INTO player_saves (
                 player_id, x, z, y, body, colors, gender, runenergy, playtime,
                 stats, levels, afk_zones, last_afk_zone,
                 public_chat, private_chat, trade_chat, last_date,
                 varp_ids, varp_values, inv_types, inv_slots, inv_objs, inv_counts
             )
             SELECT id, $2, $3, $4, $5, $6, $7, $8, $9,
                    $10, $11, $12, $13,
                    $14, $15, $16, $17,
                    $18, $19, $20, $21, $22, $23
             FROM players WHERE name_hash = $1
             ON CONFLICT (player_id) DO UPDATE SET
                 x = EXCLUDED.x, z = EXCLUDED.z, y = EXCLUDED.y,
                 body = EXCLUDED.body, colors = EXCLUDED.colors,
                 gender = EXCLUDED.gender, runenergy = EXCLUDED.runenergy,
                 playtime = EXCLUDED.playtime, stats = EXCLUDED.stats,
                 levels = EXCLUDED.levels, afk_zones = EXCLUDED.afk_zones,
                 last_afk_zone = EXCLUDED.last_afk_zone,
                 public_chat = EXCLUDED.public_chat,
                 private_chat = EXCLUDED.private_chat,
                 trade_chat = EXCLUDED.trade_chat,
                 last_date = EXCLUDED.last_date,
                 varp_ids = EXCLUDED.varp_ids, varp_values = EXCLUDED.varp_values,
                 inv_types = EXCLUDED.inv_types, inv_slots = EXCLUDED.inv_slots,
                 inv_objs = EXCLUDED.inv_objs, inv_counts = EXCLUDED.inv_counts,
                 updated_at = now()",
        )
        .bind(name_hash as i64)
        .bind(save.x)
        .bind(save.z)
        .bind(save.y)
        .bind(&save.body)
        .bind(&save.colors)
        .bind(save.gender)
        .bind(save.runenergy)
        .bind(save.playtime)
        .bind(&save.stats)
        .bind(&save.levels)
        .bind(&save.afk_zones)
        .bind(save.last_afk_zone)
        .bind(save.public_chat)
        .bind(save.private_chat)
        .bind(save.trade_chat)
        .bind(save.last_date)
        .bind(&save.varp_ids)
        .bind(&save.varp_values)
        .bind(&save.inv_types)
        .bind(&save.inv_slots)
        .bind(&save.inv_objs)
        .bind(&save.inv_counts)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
