mod auth;
mod capabilities;
mod changes;
mod devices;
mod nodes;
mod search;
mod trash;
mod uploads;
mod ws;

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .merge(auth::routes())
        .merge(capabilities::routes())
        .merge(changes::routes())
        .merge(devices::routes())
        .merge(nodes::routes())
        .merge(trash::routes())
        .merge(uploads::routes())
        .merge(search::routes())
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
