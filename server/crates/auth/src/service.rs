use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use db::{Db, DbError, User, UserId};
use protocol::{NodeId, TicketRequest, TicketResponse, TokenPair};

use crate::{
    AuthError, Claims, password,
    token::{
        decode_access, decode_ticket, encode_access, encode_ticket, hash_refresh_token,
        new_refresh_token,
    },
};

const MAX_USERNAME_LEN: usize = 64;
const MIN_PASSWORD_LEN: usize = 8;
const TICKET_TTL_SECS: i64 = 60;
const SCOPE_EVENTS: &str = "events";
const SCOPE_DOWNLOAD_PREFIX: &str = "download:";
const DUMMY_PASSWORD: &str = "dummy-password-for-timing";

/// Token settings.
#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub jwt_secret: Vec<u8>,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

/// Login, refresh, logout and user management.
#[derive(Clone, Debug)]
pub struct AuthService {
    db: Db,
    config: AuthConfig,
    dummy_hash: String,
}

impl AuthService {
    /// Creates the service; fails only if the password hasher cannot run.
    pub fn new(db: Db, config: AuthConfig) -> Result<Self, AuthError> {
        let dummy_hash = password::hash(DUMMY_PASSWORD)?;
        Ok(Self {
            db,
            config,
            dummy_hash,
        })
    }

    /// Creates a user after validating username and password.
    pub async fn create_user(&self, username: &str, password: &str) -> Result<User, AuthError> {
        if username.is_empty() || username.len() > MAX_USERNAME_LEN {
            Err(AuthError::InvalidInput("username must be 1-64 characters"))
        } else if password.len() < MIN_PASSWORD_LEN {
            Err(AuthError::InvalidInput(
                "password must be at least 8 characters",
            ))
        } else {
            let hashed = self.hash_blocking(password).await?;
            self.db
                .create_user(username, &hashed)
                .await
                .map_err(|err| match err {
                    DbError::Conflict => AuthError::UsernameTaken,
                    other => other.into(),
                })
        }
    }

    /// Changes a user's password and revokes their sessions.
    pub async fn set_password(&self, user_id: UserId, password: &str) -> Result<(), AuthError> {
        if password.len() < MIN_PASSWORD_LEN {
            Err(AuthError::InvalidInput(
                "password must be at least 8 characters",
            ))
        } else {
            let hashed = self.hash_blocking(password).await?;
            Ok(self.db.set_password_hash(user_id, &hashed).await?)
        }
    }

    /// Verifies credentials and issues a new token family.
    pub async fn login(&self, username: &str, password: &str) -> Result<TokenPair, AuthError> {
        let user = self.db.find_user_by_username(username).await?;
        let hash = user.as_ref().map_or_else(
            || self.dummy_hash.clone(),
            |user| user.password_hash.clone(),
        );
        let candidate = password.to_owned();
        let verified =
            tokio::task::spawn_blocking(move || password::verify(&candidate, &hash)).await?;
        match user {
            Some(user) if verified => self.issue(user.id, Uuid::now_v7()).await,
            _ => Err(AuthError::InvalidCredentials),
        }
    }

    /// Exchanges a refresh token for a new pair, revoking the old token.
    ///
    /// Presenting an already-used token revokes the entire family.
    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        let stored = self
            .db
            .find_refresh_token(&hash_refresh_token(refresh_token))
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        if stored.revoked_at.is_some() {
            self.db.revoke_refresh_family(stored.family_id).await?;
            Err(AuthError::InvalidCredentials)
        } else if stored.expires_at <= OffsetDateTime::now_utc() {
            Err(AuthError::InvalidCredentials)
        } else if self.db.revoke_refresh_token(stored.id).await? {
            self.issue(stored.user_id, stored.family_id).await
        } else {
            self.db.revoke_refresh_family(stored.family_id).await?;
            Err(AuthError::InvalidCredentials)
        }
    }

    /// Revokes the token family of `refresh_token`; unknown tokens are ignored.
    pub async fn logout(&self, refresh_token: &str) -> Result<(), AuthError> {
        if let Some(stored) = self
            .db
            .find_refresh_token(&hash_refresh_token(refresh_token))
            .await?
        {
            self.db.revoke_refresh_family(stored.family_id).await?;
        }
        Ok(())
    }

    /// Validates an access token.
    pub fn verify_access(&self, token: &str) -> Result<Claims, AuthError> {
        decode_access(&self.config.jwt_secret, token)
    }

    /// Lifetime of access tokens.
    pub fn access_ttl(&self) -> Duration {
        self.config.access_ttl
    }

    /// Issues a short-lived ticket limited to `request`'s scope.
    pub fn issue_ticket(
        &self,
        user_id: UserId,
        request: &TicketRequest,
    ) -> Result<TicketResponse, AuthError> {
        let ttl = Duration::seconds(TICKET_TTL_SECS);
        Ok(TicketResponse {
            ticket: encode_ticket(&self.config.jwt_secret, user_id, &scope_of(request), ttl)?,
            expires_in: TICKET_TTL_SECS,
        })
    }

    /// Validates a ticket for `request`'s scope and returns its user.
    pub fn verify_ticket(
        &self,
        ticket: &str,
        request: &TicketRequest,
    ) -> Result<UserId, AuthError> {
        decode_ticket(&self.config.jwt_secret, ticket, &scope_of(request))
    }

    async fn issue(&self, user_id: UserId, family_id: Uuid) -> Result<TokenPair, AuthError> {
        let refresh_token = new_refresh_token();
        self.db
            .insert_refresh_token(
                user_id,
                family_id,
                &hash_refresh_token(&refresh_token),
                OffsetDateTime::now_utc() + self.config.refresh_ttl,
            )
            .await?;
        Ok(TokenPair {
            access_token: encode_access(&self.config.jwt_secret, user_id, self.config.access_ttl)?,
            refresh_token,
            expires_in: self.config.access_ttl.whole_seconds(),
        })
    }

    async fn hash_blocking(&self, password: &str) -> Result<String, AuthError> {
        let password = password.to_owned();
        tokio::task::spawn_blocking(move || password::hash(&password)).await?
    }
}

fn scope_of(request: &TicketRequest) -> String {
    match request {
        TicketRequest::Events => SCOPE_EVENTS.to_owned(),
        TicketRequest::Download { node_id } => download_scope(*node_id),
        TicketRequest::DownloadVersion { node_id, version } => {
            format!("{}:v{version}", download_scope(*node_id))
        }
    }
}

fn download_scope(node_id: NodeId) -> String {
    format!("{SCOPE_DOWNLOAD_PREFIX}{node_id}")
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    fn service(pool: PgPool) -> AuthService {
        let config = AuthConfig {
            jwt_secret: b"test-secret".to_vec(),
            access_ttl: Duration::minutes(15),
            refresh_ttl: Duration::days(30),
        };
        AuthService::new(Db::from_pool(pool), config).expect("service")
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn login_succeeds_with_correct_password(pool: PgPool) {
        let auth = service(pool);
        auth.create_user("alice", "password-1").await.expect("user");
        let pair = auth.login("alice", "password-1").await.expect("login");
        assert!(auth.verify_access(&pair.access_token).is_ok());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn login_fails_with_wrong_password(pool: PgPool) {
        let auth = service(pool);
        auth.create_user("alice", "password-1").await.expect("user");
        let result = auth.login("alice", "password-2").await;
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn login_fails_for_unknown_user(pool: PgPool) {
        let result = service(pool).login("nobody", "password-1").await;
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn duplicate_username_is_rejected(pool: PgPool) {
        let auth = service(pool);
        auth.create_user("alice", "password-1").await.expect("user");
        let result = auth.create_user("alice", "password-2").await;
        assert!(matches!(result, Err(AuthError::UsernameTaken)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn refresh_rotates_the_token(pool: PgPool) {
        let auth = service(pool);
        auth.create_user("alice", "password-1").await.expect("user");
        let first = auth.login("alice", "password-1").await.expect("login");
        let second = auth.refresh(&first.refresh_token).await.expect("refresh");
        assert_ne!(first.refresh_token, second.refresh_token);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn reusing_a_refresh_token_revokes_the_family(pool: PgPool) {
        let auth = service(pool);
        auth.create_user("alice", "password-1").await.expect("user");
        let first = auth.login("alice", "password-1").await.expect("login");
        let second = auth.refresh(&first.refresh_token).await.expect("refresh");
        assert!(auth.refresh(&first.refresh_token).await.is_err());
        assert!(auth.refresh(&second.refresh_token).await.is_err());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn logout_revokes_the_session(pool: PgPool) {
        let auth = service(pool);
        auth.create_user("alice", "password-1").await.expect("user");
        let pair = auth.login("alice", "password-1").await.expect("login");
        auth.logout(&pair.refresh_token).await.expect("logout");
        assert!(auth.refresh(&pair.refresh_token).await.is_err());
    }
}
