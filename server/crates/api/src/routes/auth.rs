use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use auth::TokenPair;
use db::UserId;

use crate::{ApiError, AppState, extract::AuthUser};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Serialize)]
struct MeResponse {
    id: UserId,
    username: String,
}

async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<TokenPair>, ApiError> {
    Ok(Json(
        state
            .auth
            .login(&request.username, &request.password)
            .await?,
    ))
}

async fn refresh(
    State(state): State<AppState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<TokenPair>, ApiError> {
    Ok(Json(state.auth.refresh(&request.refresh_token).await?))
}

async fn logout(
    State(state): State<AppState>,
    Json(request): Json<RefreshRequest>,
) -> Result<StatusCode, ApiError> {
    state.auth.logout(&request.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn me(State(state): State<AppState>, user: AuthUser) -> Result<Json<MeResponse>, ApiError> {
    let found = state
        .db
        .find_user(user.id)
        .await
        .map_err(|err| ApiError::Internal(err.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(MeResponse {
        id: found.id,
        username: found.username,
    }))
}
