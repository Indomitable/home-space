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
    #[error("password hashing failed: {0}")]
    Hashing(String),
    #[error("token encoding failed: {0}")]
    Token(#[from] jsonwebtoken::errors::Error),
    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Db(#[from] db::DbError),
}
