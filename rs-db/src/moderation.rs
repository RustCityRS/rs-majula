use crate::{Db, Error};

/// A staff moderation command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModAction {
    Mute { hours: i32 },
    Unmute,
    Ban { hours: i32 },
    Unban,
    Kick,
}

impl ModAction {
    /// Its name in `moderation_logs.action`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mute { .. } => "mute",
            Self::Unmute => "unmute",
            Self::Ban { .. } => "ban",
            Self::Unban => "unban",
            Self::Kick => "kick",
        }
    }

    pub const fn hours(self) -> Option<i32> {
        match self {
            Self::Mute { hours } | Self::Ban { hours } => Some(hours),
            _ => None,
        }
    }
}

impl Db {
    /// Applies `action` to `target` for a moderator of `by_level`, and logs it.
    /// Refused (`false`, nothing changed or logged) if the target isn't a
    /// registered player, or is staff of `by_level` or above.
    pub async fn moderate(
        &self,
        moderator: u64,
        by_level: i16,
        target: u64,
        action: ModAction,
        node_id: i16,
    ) -> Result<bool, Error> {
        // Each is rank-checked and returns the target's id.
        let query = match action {
            ModAction::Mute { .. } => {
                "UPDATE players SET muted_until = now() + make_interval(hours => $3)
                 WHERE name_hash = $1 AND staff_mod_level < $2 RETURNING id"
            }
            ModAction::Unmute => {
                "UPDATE players SET muted_until = NULL
                 WHERE name_hash = $1 AND staff_mod_level < $2 RETURNING id"
            }
            ModAction::Ban { .. } => {
                "UPDATE players SET banned_until = now() + make_interval(hours => $3)
                 WHERE name_hash = $1 AND staff_mod_level < $2 RETURNING id"
            }
            ModAction::Unban => {
                "UPDATE players SET banned_until = NULL
                 WHERE name_hash = $1 AND staff_mod_level < $2 RETURNING id"
            }
            ModAction::Kick => {
                "SELECT id FROM players WHERE name_hash = $1 AND staff_mod_level < $2"
            }
        };
        let mut tx = self.pool.begin().await?;
        let mut find = sqlx::query_scalar(query).bind(target as i64).bind(by_level);
        if let Some(hours) = action.hours() {
            find = find.bind(hours);
        }
        let target_id: Option<i64> = find.fetch_optional(&mut *tx).await?;
        let Some(target_id) = target_id else {
            return Ok(false);
        };
        sqlx::query(
            "INSERT INTO moderation_logs (moderator_id, target_id, action, hours, node_id)
             SELECT id, $2, $3, $4, $5 FROM players WHERE name_hash = $1",
        )
        .bind(moderator as i64)
        .bind(target_id)
        .bind(action.name())
        .bind(action.hours())
        .bind(node_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
    }
}
