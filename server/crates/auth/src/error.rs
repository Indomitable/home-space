/// Errors returned by the authentication layer.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Unknown user, wrong password or invalid/expired/revoked token.
    #[error("invalid credentials")]
    InvalidCredentials,
    /// The username is already taken.
    #[error("username already exists")]
    UsernameTaken,
    /// The username or password does not meet the requirements.
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    /// Single sign-on is not configured on this server.
    #[error("single sign-on is not enabled")]
    OidcDisabled,
    /// The identity provider authenticated the user but access is restricted to a group.
    #[error("access denied")]
    OidcDenied,
    /// The identity provider could not be reached or sent something invalid.
    #[error("identity provider error: {0}")]
    Oidc(String),
    #[error("password hashing failed: {0}")]
    Hashing(String),
    #[error("token encoding failed: {0}")]
    Token(#[from] jsonwebtoken::errors::Error),
    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Db(#[from] db::DbError),
}
