use uuid::Uuid;

use crate::{Db, DbError, RefreshToken};

macro_rules! token_columns {
    () => {
        "id, user_id, family_id, token_hash, expires_at, revoked_at"
    };
}

impl Db {
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
}
