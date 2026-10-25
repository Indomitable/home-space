use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use auth::AuthError;
use files::FilesError;
use protocol::{ErrorBody, ErrorCode};

/// Error returned by handlers, rendered as `{ "error": code, "message": text }`.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("authentication required")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Files(#[from] FilesError),
    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    fn status_and_code(&self) -> (StatusCode, ErrorCode) {
        match self {
            Self::Unauthorized | Self::Auth(AuthError::InvalidCredentials) => {
                (StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized)
            }
            Self::Auth(AuthError::UsernameTaken) => (StatusCode::CONFLICT, ErrorCode::Conflict),
            Self::Auth(AuthError::InvalidInput(_)) => {
                (StatusCode::BAD_REQUEST, ErrorCode::BadRequest)
            }
            Self::NotFound | Self::Files(FilesError::NotFound) => {
                (StatusCode::NOT_FOUND, ErrorCode::NotFound)
            }
            Self::Files(FilesError::Conflict) => (StatusCode::CONFLICT, ErrorCode::Conflict),
            Self::Files(FilesError::InvalidName | FilesError::Cycle | FilesError::NotAFile) => {
                (StatusCode::BAD_REQUEST, ErrorCode::BadRequest)
            }
            Self::Files(FilesError::TooLarge { .. }) => {
                (StatusCode::PAYLOAD_TOO_LARGE, ErrorCode::PayloadTooLarge)
            }
            Self::Auth(_) | Self::Files(_) | Self::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::Internal)
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %self, "request failed");
            "internal server error".to_owned()
        } else {
            self.to_string()
        };
        (
            status,
            Json(ErrorBody {
                error: code,
                message,
            }),
        )
            .into_response()
    }
}
