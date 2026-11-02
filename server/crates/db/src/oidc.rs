use sqlx::FromRow;
use time::OffsetDateTime;

use crate::{Db, DbError, UserId};

/// A sign-in sent to the identity provider that has not come back yet.
#[derive(Clone, Debug, FromRow)]
pub struct OidcPending {
    pub nonce: String,
    pub idp_pkce_verifier: String,
    /// Where the client wants to be sent once the sign-in is done.
    pub client_redirect: String,
    /// S256 challenge of the verifier the client will present when exchanging its code.
    pub client_challenge: String,
}

/// A one-time code issued to a client after a successful sign-in.
#[derive(Clone, Debug, FromRow)]
pub struct OidcCode {
    pub user_id: UserId,
    pub client_challenge: String,
}

impl Db {
    /// Remembers a sign-in until the provider sends the user back.
    pub async fn insert_oidc_pending(
        &self,
        state_hash: &[u8],
        pending: &OidcPending,
        expires_at: OffsetDateTime,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO oidc_pending \
             (state_hash, nonce, idp_pkce_verifier, client_redirect, client_challenge, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(state_hash)
        .bind(&pending.nonce)
        .bind(&pending.idp_pkce_verifier)
        .bind(&pending.client_redirect)
        .bind(&pending.client_challenge)
        .bind(expires_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Removes and returns an unexpired pending sign-in, so a `state` works only once.
    pub async fn take_oidc_pending(
        &self,
        state_hash: &[u8],
    ) -> Result<Option<OidcPending>, DbError> {
        let pending = sqlx::query_as::<_, OidcPending>(
            "DELETE FROM oidc_pending WHERE state_hash = $1 AND expires_at > now() \
             RETURNING nonce, idp_pkce_verifier, client_redirect, client_challenge",
        )
        .bind(state_hash)
        .fetch_optional(self.pool())
        .await?;
        Ok(pending)
    }

    /// Stores the one-time code a client exchanges for tokens.
    pub async fn insert_oidc_code(
        &self,
        code_hash: &[u8],
        user_id: UserId,
        client_challenge: &str,
        expires_at: OffsetDateTime,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO oidc_codes (code_hash, user_id, client_challenge, expires_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(code_hash)
        .bind(user_id)
        .bind(client_challenge)
        .bind(expires_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Removes and returns an unexpired login code, so a code works only once.
    pub async fn take_oidc_code(&self, code_hash: &[u8]) -> Result<Option<OidcCode>, DbError> {
        let code = sqlx::query_as::<_, OidcCode>(
            "DELETE FROM oidc_codes WHERE code_hash = $1 AND expires_at > now() \
             RETURNING user_id, client_challenge",
        )
        .bind(code_hash)
        .fetch_optional(self.pool())
        .await?;
        Ok(code)
    }

    /// Deletes expired pending sign-ins and login codes, returning how many were removed.
    pub async fn prune_oidc(&self) -> Result<u64, DbError> {
        let pending = sqlx::query("DELETE FROM oidc_pending WHERE expires_at <= now()")
            .execute(self.pool())
            .await?;
        let codes = sqlx::query("DELETE FROM oidc_codes WHERE expires_at <= now()")
            .execute(self.pool())
            .await?;
        Ok(pending.rows_affected() + codes.rows_affected())
    }
}
