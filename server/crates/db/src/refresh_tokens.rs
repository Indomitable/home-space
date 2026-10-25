use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Db, DbError, RefreshToken, UserId};

macro_rules! token_columns {
    () => {
        "id, user_id, family_id, token_hash, expires_at, revoked_at"
    };
}

impl Db {
    /// Stores a new refresh token hash in the given family.
    pub async fn insert_refresh_token(
        &self,
        user_id: UserId,
        family_id: Uuid,
        token_hash: &[u8],
        expires_at: OffsetDateTime,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO refresh_tokens (id, user_id, family_id, token_hash, expires_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(family_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Finds a refresh token by its hash.
    pub async fn find_refresh_token(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<RefreshToken>, DbError> {
        let token = sqlx::query_as::<_, RefreshToken>(concat!(
            "SELECT ",
            token_columns!(),
            " FROM refresh_tokens WHERE token_hash = $1"
        ))
        .bind(token_hash)
        .fetch_optional(self.pool())
        .await?;
        Ok(token)
    }

    /// Atomically revokes a token that is still active.
    ///
    /// Returns `false` when it was already revoked, so concurrent refreshes
    /// with the same token cannot both succeed.
    pub async fn revoke_refresh_token(&self, id: Uuid) -> Result<bool, DbError> {
        let updated = sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(updated.rows_affected() == 1)
    }

    /// Revokes every active token of a family.
    pub async fn revoke_refresh_family(&self, family_id: Uuid) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now() \
             WHERE family_id = $1 AND revoked_at IS NULL",
        )
        .bind(family_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }
}
