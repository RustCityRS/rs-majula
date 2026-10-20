use crate::hiscores::update_hiscores;
use crate::history::{keep_logout_point, keep_session_point, keep_staff_point};
use crate::{Db, Error};
use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::{PgConnection, Postgres};

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

/// Where a save ended up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveResult {
    Stored,
    NotOwner,
    Unregistered,
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

    /// Writes a player's save for world `node_id`, creating the row on first
    /// save. Only the world that owns the player's session may save them.
    /// Hiscores are left alone: they only move on logout. Through a long
    /// session a copy is kept in the save history every so often.
    pub async fn store_save(
        &self,
        name_hash: u64,
        node_id: i16,
        save: &PlayerSave,
    ) -> Result<SaveResult, Error> {
        let mut tx = self.pool.begin().await?;
        let id = match lock_session(&mut tx, name_hash, node_id).await? {
            Ok(id) => id,
            Err(refused) => return Ok(refused),
        };
        upsert_save(id, save).execute(&mut *tx).await?;
        keep_session_point(&mut tx, id, node_id).await?;
        tx.commit().await?;
        Ok(SaveResult::Stored)
    }

    /// Writes a player's final save as they leave world `node_id`, ends their
    /// session and records the logout for the world-hop delay, all in one
    /// transaction: no other world can claim the player until their final
    /// save has landed. This is the only live save that updates hiscores, and
    /// a copy is kept in the save history (unless one was kept just before).
    pub async fn store_logout_save(
        &self,
        name_hash: u64,
        node_id: i16,
        save: &PlayerSave,
    ) -> Result<SaveResult, Error> {
        let mut tx = self.pool.begin().await?;
        let id = match lock_session(&mut tx, name_hash, node_id).await? {
            Ok(id) => id,
            Err(refused) => return Ok(refused),
        };
        upsert_save(id, save).execute(&mut *tx).await?;
        keep_logout_point(&mut tx, id, node_id).await?;
        update_hiscores(&mut tx, id, &save.stats).await?;
        sqlx::query(
            "UPDATE players SET online_node = NULL, online_since = NULL,
                                last_logout_node = $2, last_logout_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(node_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(SaveResult::Stored)
    }

    /// Stores a save made outside a session (staff changing a logged-out
    /// player) and ends world `node_id`'s claim on them. The save it replaces
    /// is kept in the save history, with `note` saying who and why. Unlike a
    /// logout it leaves their last logout as it was.
    pub async fn store_edit_save(
        &self,
        name_hash: u64,
        node_id: i16,
        save: &PlayerSave,
        note: &str,
    ) -> Result<SaveResult, Error> {
        let mut tx = self.pool.begin().await?;
        let id = match lock_session(&mut tx, name_hash, node_id).await? {
            Ok(id) => id,
            Err(refused) => return Ok(refused),
        };
        keep_staff_point(&mut tx, id, node_id, note).await?;
        upsert_save(id, save).execute(&mut *tx).await?;
        update_hiscores(&mut tx, id, &save.stats).await?;
        sqlx::query("UPDATE players SET online_node = NULL, online_since = NULL WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(SaveResult::Stored)
    }

    /// Writes a first save for a player who has none and is not online
    /// anywhere (importing a local save at registration). Returns `false` if
    /// the player doesn't exist, already has a save, or is online.
    pub async fn import_save(&self, name_hash: u64, save: &PlayerSave) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;
        let id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM players p
             WHERE name_hash = $1 AND online_node IS NULL
               AND NOT EXISTS (SELECT 1 FROM player_saves s WHERE s.player_id = p.id)
             FOR UPDATE",
        )
        .bind(name_hash as i64)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(id) = id else {
            return Ok(false);
        };
        upsert_save(id, save).execute(&mut *tx).await?;
        update_hiscores(&mut tx, id, &save.stats).await?;
        tx.commit().await?;
        Ok(true)
    }
}

/// Locks the player's row for the rest of `tx` and checks that world
/// `node_id` owns their session. `Ok(player id)` if it does.
pub(crate) async fn lock_session(
    tx: &mut PgConnection,
    name_hash: u64,
    node_id: i16,
) -> Result<Result<i64, SaveResult>, Error> {
    let row: Option<(i64, Option<i16>)> =
        sqlx::query_as("SELECT id, online_node FROM players WHERE name_hash = $1 FOR UPDATE")
            .bind(name_hash as i64)
            .fetch_optional(&mut *tx)
            .await?;
    Ok(match row {
        None => Err(SaveResult::Unregistered),
        Some((id, Some(owner))) if owner == node_id => Ok(id),
        Some(_) => Err(SaveResult::NotOwner),
    })
}

fn upsert_save(player_id: i64, save: &PlayerSave) -> Query<'_, Postgres, PgArguments> {
    sqlx::query(
        "INSERT INTO player_saves (
                 player_id, x, z, y, body, colors, gender, runenergy, playtime,
                 stats, levels, afk_zones, last_afk_zone,
                 public_chat, private_chat, trade_chat, last_date,
                 varp_ids, varp_values, inv_types, inv_slots, inv_objs, inv_counts
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                     $10, $11, $12, $13,
                     $14, $15, $16, $17,
                     $18, $19, $20, $21, $22, $23)
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
    .bind(player_id)
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
}
