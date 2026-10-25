mod auth;
mod nodes;
mod ws;

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .merge(auth::routes())
        .merge(nodes::routes())
        .merge(ws::routes())
}

async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<Json<Value>, ApiError> {
    state
        .db
        .ping()
        .await
        .map_err(|err| ApiError::Internal(err.to_string()))?;
    Ok(Json(json!({ "status": "ok" })))
}
