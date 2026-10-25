//! HTTP and websocket API for home-space.

mod error;
mod extract;
mod routes;
mod state;

pub use error::ApiError;
pub use state::AppState;

use axum::Router;
use tower_http::trace::TraceLayer;

const API_PREFIX: &str = "/api/v1";

/// Builds the application router with all routes mounted under `/api/v1`.
pub fn router(state: AppState) -> Router {
    Router::new()
        .nest(API_PREFIX, routes::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
