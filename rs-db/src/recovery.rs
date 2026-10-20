use crate::{Db, Error};

/// What a locked account's password is set to: no password verifies against
/// it (it isn't a PHC string), so only the reset code gets them back in.
pub const RESET_LOCKED: &str = "!locked for a password reset";

/// A password reset waiting to be redeemed.
#[derive(Debug, sqlx::FromRow)]
pub struct ResetCode {
    pub display_name: String,
    /// The code, hashed as passwords are.
    pub code_hash: String,
}

impl Db {
    /// Issues a password reset: the player can set a new password with the
    /// code hashed as `code_hash` for the next `hours`. With `lock`, their
    /// current password stops working now. A reset issued before is
    /// replaced. Returns `false` if no such player.
    pub async fn issue_password_reset(
        &self,
        name_hash: u64,
        code_hash: &str,
        hours: i32,
        lock: bool,
    ) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE players
             SET reset_code_hash = $2,
                 reset_code_until = now() + make_interval(hours => $3),
                 password_hash = CASE WHEN $4 THEN $5 ELSE password_hash END
             WHERE name_hash = $1",
        )
        .bind(name_hash as i64)
        .bind(code_hash)
        .bind(hours)
        .bind(lock)
        .bind(RESET_LOCKED)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// The player's password reset, while it stands.
    pub async fn password_reset(&self, name_hash: u64) -> Result<Option<ResetCode>, Error> {
        sqlx::query_as(
            "SELECT display_name, reset_code_hash AS code_hash FROM players
             WHERE name_hash = $1 AND reset_code_hash IS NOT NULL AND reset_code_until > now()",
        )
        .bind(name_hash as i64)
        .fetch_optional(&self.pool)
        .await
    }

    /// Sets the player's new password with the reset they were issued, and
    /// ends the reset: `code_hash` is the one [`Db::password_reset`] gave,
    /// so a code is only ever redeemed once. Returns whether it took.
    pub async fn redeem_password_reset(
        &self,
        name_hash: u64,
        code_hash: &str,
        password_hash: &str,
    ) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE players
             SET password_hash = $3, reset_code_hash = NULL, reset_code_until = NULL
             WHERE name_hash = $1 AND reset_code_hash = $2 AND reset_code_until > now()",
        )
        .bind(name_hash as i64)
        .bind(code_hash)
        .bind(password_hash)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
