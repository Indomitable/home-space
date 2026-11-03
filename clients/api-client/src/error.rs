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

impl ApiError {
    /// Whether the same request may succeed when sent again: the connection failed, a proxy
    /// answered instead of the server, a resumable upload's offset moved on, or the server
    /// hit a passing error.
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Transport(_) | Self::Decode(_) => true,
            Self::Api { code, .. } => matches!(code, ErrorCode::Conflict | ErrorCode::Internal),
            Self::Unauthorized => false,
        }
    }
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
