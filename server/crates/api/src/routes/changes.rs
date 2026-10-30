use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};

use db::DbError;
use protocol::{Change, ChangesQuery, ChangesResponse, Node};

use crate::{ApiError, AppState, extract::AuthUser};

const DEFAULT_LIMIT: u32 = 500;
const MAX_LIMIT: u32 = 1000;

pub fn routes() -> Router<AppState> {
    Router::new().route("/changes", get(changes))
}

async fn changes(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ChangesQuery>,
) -> Result<Json<ChangesResponse>, ApiError> {
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let batch = state
        .db
        .changes(user.id, query.since, i64::from(limit))
        .await
        .map_err(|err| match err {
            DbError::Pruned => ApiError::Gone,
            other => ApiError::Internal(other.to_string()),
        })?;
    Ok(Json(ChangesResponse {
        changes: batch
            .changes
            .into_iter()
            .map(|change| match change.node {
                Some(node) => Change::Upserted {
                    seq: change.seq,
                    node: Node::from(node),
                },
                None => Change::Deleted {
                    seq: change.seq,
                    id: change.id,
                },
            })
            .collect(),
        cursor: batch.cursor,
        more: batch.more,
    }))
}
