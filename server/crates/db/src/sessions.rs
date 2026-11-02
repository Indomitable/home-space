use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Db, DbError, UserId};

impl Db {
    /// Starts a session with its first refresh token.
    pub async fn start_session(
        &self,
        session_id: Uuid,
        user_id: UserId,
        token_hash: &[u8],
        expires_at: OffsetDateTime,
    ) -> Result<(), DbError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3)")
            .bind(session_id)
            .bind(user_id)
            .bind(expires_at)
            .execute(&mut *tx)
            .await?;
        insert_token(&mut tx, session_id, user_id, token_hash, expires_at).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Stores the next refresh token of a session and extends the session to match.
    ///
    /// Returns `false` when the session has ended in the meantime.
    pub async fn continue_session(
        &self,
        session_id: Uuid,
        user_id: UserId,
        token_hash: &[u8],
        expires_at: OffsetDateTime,
    ) -> Result<bool, DbError> {
        let mut tx = self.pool().begin().await?;
        let extended = sqlx::query(
            "UPDATE sessions SET expires_at = $3 WHERE id = $1 AND user_id = $2 AND expires_at > now()",
        )
        .bind(session_id)
        .bind(user_id)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;
        if extended.rows_affected() == 0 {
            return Ok(false);
        }
        insert_token(&mut tx, session_id, user_id, token_hash, expires_at).await?;
        tx.commit().await?;
        Ok(true)
    }

    /// Whether `session_id` is an unexpired session of `user_id`.
    pub async fn session_is_live(
        &self,
        session_id: Uuid,
        user_id: UserId,
    ) -> Result<bool, DbError> {
        let live = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM sessions \
             WHERE id = $1 AND user_id = $2 AND expires_at > now())",
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_one(self.pool())
        .await?;
        Ok(live)
    }

    /// Ends a session and, with it, its refresh tokens.
    pub async fn end_session(&self, session_id: Uuid) -> Result<(), DbError> {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    /// Ends every session of a user.
    pub async fn end_user_sessions(&self, user_id: UserId) -> Result<(), DbError> {
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    /// Deletes expired sessions, returning how many were removed.
    pub async fn prune_sessions(&self) -> Result<u64, DbError> {
        let pruned = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
            .execute(self.pool())
            .await?;
        Ok(pruned.rows_affected())
    }
}

async fn insert_token(
    tx: &mut sqlx::PgConnection,
    session_id: Uuid,
    user_id: UserId,
    token_hash: &[u8],
    expires_at: OffsetDateTime,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, family_id, token_hash, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(session_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    const BEFORE_SESSIONS: [&str; 6] = [
        include_str!("../../../migrations/0001_init.sql"),
        include_str!("../../../migrations/0002_search.sql"),
        include_str!("../../../migrations/0003_versions.sql"),
        include_str!("../../../migrations/0004_trash.sql"),
        include_str!("../../../migrations/0005_changes.sql"),
        include_str!("../../../migrations/0006_oidc.sql"),
    ];
    const SESSIONS: &str = include_str!("../../../migrations/0007_sessions.sql");

    async fn token(pool: &PgPool, user: Uuid, family: Uuid, revoked: bool, days: i32) {
        sqlx::query(
            "INSERT INTO refresh_tokens (id, user_id, family_id, token_hash, expires_at, revoked_at) \
             VALUES ($1, $2, $3, $4, now() + make_interval(days => $5), \
                     CASE WHEN $6 THEN now() END)",
        )
        .bind(Uuid::now_v7())
        .bind(user)
        .bind(family)
        .bind(Uuid::now_v7().as_bytes().to_vec())
        .bind(days)
        .bind(revoked)
        .execute(pool)
        .await
        .expect("token");
    }

    #[sqlx::test(migrations = false)]
    async fn upgrade_keeps_live_logins_and_drops_dead_ones(pool: PgPool) {
        for migration in BEFORE_SESSIONS {
            sqlx::raw_sql(migration)
                .execute(&pool)
                .await
                .expect("migrate");
        }
        let user = Uuid::now_v7();
        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, 'alice', 'x')")
            .bind(user)
            .execute(&pool)
            .await
            .expect("user");
        let (live, logged_out, expired) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
        // A rotated login: its old token is revoked, its current one is usable.
        token(&pool, user, live, true, 30).await;
        token(&pool, user, live, false, 30).await;
        token(&pool, user, logged_out, true, 30).await;
        token(&pool, user, expired, false, -1).await;

        sqlx::raw_sql(SESSIONS)
            .execute(&pool)
            .await
            .expect("sessions");

        let sessions: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM sessions")
            .fetch_all(&pool)
            .await
            .expect("sessions");
        assert_eq!(sessions, vec![live]);
        let tokens: i64 = sqlx::query_scalar("SELECT count(*) FROM refresh_tokens")
            .fetch_one(&pool)
            .await
            .expect("count");
        assert_eq!(tokens, 2);
        let db = Db::from_pool(pool);
        assert!(
            db.session_is_live(live, UserId::from(user))
                .await
                .expect("live")
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn ending_a_session_removes_its_refresh_tokens(pool: PgPool) {
        let user = Uuid::now_v7();
        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, 'alice', 'x')")
            .bind(user)
            .execute(&pool)
            .await
            .expect("user");
        let db = Db::from_pool(pool.clone());
        let session = Uuid::now_v7();
        let expires = OffsetDateTime::now_utc() + time::Duration::days(1);
        db.start_session(session, UserId::from(user), b"one", expires)
            .await
            .expect("start");
        assert!(
            db.continue_session(session, UserId::from(user), b"two", expires)
                .await
                .expect("continue")
        );

        db.end_session(session).await.expect("end");

        let tokens: i64 = sqlx::query_scalar("SELECT count(*) FROM refresh_tokens")
            .fetch_one(&pool)
            .await
            .expect("count");
        assert_eq!(tokens, 0);
        assert!(
            !db.continue_session(session, UserId::from(user), b"three", expires)
                .await
                .expect("continue")
        );
    }
}
