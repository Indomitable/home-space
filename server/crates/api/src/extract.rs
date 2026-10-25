use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header::AUTHORIZATION, request::Parts},
};
use serde::Deserialize;

use db::UserId;

use crate::{ApiError, AppState};

const BEARER_PREFIX: &str = "Bearer ";

/// The authenticated user, taken from a valid `Authorization: Bearer` access token.
#[derive(Clone, Copy, Debug)]
pub struct AuthUser {
    pub id: UserId,
    /// Unix timestamp at which the access token expires.
    pub token_exp: i64,
}

/// Query carrying a short-lived ticket for requests that cannot send headers.
#[derive(Debug, Deserialize)]
pub struct TicketQuery {
    pub ticket: Option<String>,
}

impl AuthUser {
    /// Authenticates from the `Authorization: Bearer` header.
    pub fn from_headers(headers: &HeaderMap, state: &AppState) -> Result<Self, ApiError> {
        let token = headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix(BEARER_PREFIX))
            .ok_or(ApiError::Unauthorized)?;
        let claims = state
            .auth
            .verify_access(token)
            .map_err(|_| ApiError::Unauthorized)?;
        Ok(Self {
            id: claims.sub,
            token_exp: claims.exp,
        })
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Self::from_headers(&parts.headers, state)
    }
}
