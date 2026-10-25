use protocol::ErrorCode;

/// Failure of an API call.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ApiError {
    #[error("not signed in")]
    Unauthorized,
    #[error("{message}")]
    Api { code: ErrorCode, message: String },
    #[error("request failed: {0}")]
    Transport(String),
    #[error("unexpected response: {0}")]
    Decode(String),
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_decode() {
            Self::Decode(err.to_string())
        } else {
            Self::Transport(err.to_string())
        }
    }
}
