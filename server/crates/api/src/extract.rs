use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

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

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
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
