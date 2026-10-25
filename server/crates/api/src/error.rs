use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use auth::AuthError;
use files::FilesError;

const CODE_UNAUTHORIZED: &str = "unauthorized";
const CODE_NOT_FOUND: &str = "not_found";
const CODE_CONFLICT: &str = "conflict";
const CODE_BAD_REQUEST: &str = "bad_request";
const CODE_TOO_LARGE: &str = "payload_too_large";
const CODE_INTERNAL: &str = "internal";

/// Error returned by handlers, rendered as `{ "error": code, "message": text }`.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("authentication required")]
    Unauthorized,
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Files(#[from] FilesError),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
    message: String,
}

impl ApiError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Unauthorized | Self::Auth(AuthError::InvalidCredentials) => {
                (StatusCode::UNAUTHORIZED, CODE_UNAUTHORIZED)
            }
            Self::Auth(AuthError::UsernameTaken) => (StatusCode::CONFLICT, CODE_CONFLICT),
            Self::Auth(AuthError::InvalidInput(_)) => (StatusCode::BAD_REQUEST, CODE_BAD_REQUEST),
            Self::Files(FilesError::NotFound) => (StatusCode::NOT_FOUND, CODE_NOT_FOUND),
            Self::Files(FilesError::Conflict) => (StatusCode::CONFLICT, CODE_CONFLICT),
            Self::Files(FilesError::InvalidName | FilesError::Cycle | FilesError::NotAFile) => {
                (StatusCode::BAD_REQUEST, CODE_BAD_REQUEST)
            }
            Self::Files(FilesError::TooLarge { .. }) => {
                (StatusCode::PAYLOAD_TOO_LARGE, CODE_TOO_LARGE)
            }
            Self::Auth(_) | Self::Files(_) | Self::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, CODE_INTERNAL)
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
