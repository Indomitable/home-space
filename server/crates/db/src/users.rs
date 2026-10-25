use crate::{Db, DbError, User, UserId};

macro_rules! user_columns {
    () => {
        "id, username, password_hash, created_at"
    };
}

impl Db {
    /// Inserts a new user; `Conflict` if the username is taken.
    pub async fn create_user(&self, username: &str, password_hash: &str) -> Result<User, DbError> {
        let user = sqlx::query_as::<_, User>(concat!(
            "INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3) \
             RETURNING ",
            user_columns!(),
            ""
        ))
        .bind(UserId::new())
        .bind(username)
        .bind(password_hash)
        .fetch_one(self.pool())
        .await?;
        Ok(user)
    }

    /// Finds a user by username.
    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, DbError> {
        let user = sqlx::query_as::<_, User>(concat!(
            "SELECT ",
            user_columns!(),
            " FROM users WHERE username = $1"
        ))
        .bind(username)
        .fetch_optional(self.pool())
        .await?;
        Ok(user)
    }

    /// Finds a user by id.
    pub async fn find_user(&self, id: UserId) -> Result<Option<User>, DbError> {
        let user = sqlx::query_as::<_, User>(concat!(
            "SELECT ",
            user_columns!(),
            " FROM users WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        Ok(user)
    }

    /// Lists all users ordered by username.
    pub async fn list_users(&self) -> Result<Vec<User>, DbError> {
        let users = sqlx::query_as::<_, User>(concat!(
            "SELECT ",
            user_columns!(),
            " FROM users ORDER BY username"
        ))
        .fetch_all(self.pool())
        .await?;
        Ok(users)
    }

    /// Replaces a user's password hash and revokes all their refresh tokens.
    pub async fn set_password_hash(&self, id: UserId, password_hash: &str) -> Result<(), DbError> {
        let mut tx = self.pool().begin().await?;
        let updated = sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
            .bind(id)
            .bind(password_hash)
            .execute(&mut *tx)
            .await?;
        if updated.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now() \
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
