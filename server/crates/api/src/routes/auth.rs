use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use protocol::{LoginRequest, Me, RefreshRequest, TicketRequest, TicketResponse, TokenPair};

use crate::{ApiError, AppState, extract::AuthUser};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
        .route("/tickets", post(ticket))
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

async fn me(State(state): State<AppState>, user: AuthUser) -> Result<Json<Me>, ApiError> {
    let found = state
        .db
        .find_user(user.id)
        .await
        .map_err(|err| ApiError::Internal(err.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(Me {
        id: found.id,
        username: found.username,
    }))
}

async fn ticket(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<TicketRequest>,
) -> Result<Json<TicketResponse>, ApiError> {
    if let TicketRequest::Download { node_id } = request {
        state.files.get(user.id, node_id).await?;
    }
    Ok(Json(state.auth.issue_ticket(user.id, &request)?))
}
