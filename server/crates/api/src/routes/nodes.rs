use std::io;

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::Response,
    routing::{get, post, put},
};
use futures_util::TryStreamExt;
use tokio_util::io::ReaderStream;

use db::NodeUpdate;
use notifications::Event;
use protocol::{
    CreateFolderRequest, ListQuery, Node, NodeId, TicketRequest, UpdateRequest, UploadQuery,
};

use crate::{
    ApiError, AppState,
    extract::{AuthUser, TicketQuery},
};

const FALLBACK_MIME: &str = "application/octet-stream";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nodes", get(list))
        .route("/nodes/{id}", get(get_node).patch(update).delete(delete))
        .route("/nodes/{id}/ancestors", get(ancestors))
        .route("/folders", post(create_folder))
        .route("/files", post(upload).layer(DefaultBodyLimit::disable()))
        .route(
            "/files/{id}/content",
            put(replace_content)
                .layer(DefaultBodyLimit::disable())
                .get(download),
        )
}

fn body_stream(body: Body) -> impl futures_util::Stream<Item = io::Result<axum::body::Bytes>> {
    body.into_data_stream().map_err(io::Error::other)
}

fn declared_type(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
}

async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Node>>, ApiError> {
    let nodes = state.files.list(user.id, query.parent_id).await?;
    Ok(Json(nodes.into_iter().map(Node::from).collect()))
}

async fn get_node(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<NodeId>,
) -> Result<Json<Node>, ApiError> {
    Ok(Json(state.files.get(user.id, id).await?.into()))
}

async fn ancestors(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<NodeId>,
) -> Result<Json<Vec<Node>>, ApiError> {
    let nodes = state.files.ancestors(user.id, id).await?;
    Ok(Json(nodes.into_iter().map(Node::from).collect()))
}

async fn create_folder(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateFolderRequest>,
) -> Result<(StatusCode, Json<Node>), ApiError> {
    let node = state
        .files
        .create_folder(user.id, request.parent_id, &request.name)
        .await?;
    state.index_name(&node).await;
    let node = Node::from(node);
    state
        .hub
        .publish(user.id, Event::NodeCreated { node: node.clone() });
    Ok((StatusCode::CREATED, Json(node)))
}

async fn upload(
    State(state): State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
    Query(query): Query<UploadQuery>,
    body: Body,
) -> Result<(StatusCode, Json<Node>), ApiError> {
    let node = state
        .files
        .upload(
            user.id,
            query.parent_id,
            &query.name,
            declared_type(&headers),
            body_stream(body),
        )
        .await?;
    state.index_name(&node).await;
    state.search.enqueue_content(user.id, node.id);
    let node = Node::from(node);
    state
        .hub
        .publish(user.id, Event::NodeCreated { node: node.clone() });
    Ok((StatusCode::CREATED, Json(node)))
}

async fn replace_content(
    State(state): State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
    Path(id): Path<NodeId>,
    body: Body,
) -> Result<Json<Node>, ApiError> {
    let node = state
        .files
        .replace_content(user.id, id, declared_type(&headers), body_stream(body))
        .await?;
    state.search.enqueue_content(user.id, node.id);
    let node = Node::from(node);
    state
        .hub
        .publish(user.id, Event::NodeUpdated { node: node.clone() });
    Ok(Json(node))
}

async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TicketQuery>,
    Path(id): Path<NodeId>,
) -> Result<Response, ApiError> {
    let owner = match query.ticket {
        Some(ticket) => state
            .auth
            .verify_ticket(&ticket, &TicketRequest::Download { node_id: id })?,
        None => AuthUser::from_headers(&headers, &state)?.id,
    };
    let (node, file) = state.files.download(owner, id).await?;
    let content_type = node.mime.as_deref().unwrap_or(FALLBACK_MIME);
    let disposition = format!(
        "attachment; filename*=UTF-8''{}",
        percent_encode(&node.name)
    );
    let mut response = Response::new(Body::from_stream(ReaderStream::new(file)));
    let headers = response.headers_mut();
    headers.insert(CONTENT_LENGTH, HeaderValue::from(node.size.unsigned_abs()));
    if let Ok(value) = HeaderValue::from_str(content_type) {
        headers.insert(CONTENT_TYPE, value);
    }
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<NodeId>,
    Json(request): Json<UpdateRequest>,
) -> Result<Json<Node>, ApiError> {
    let changes = NodeUpdate {
        name: request.name,
        parent_id: request.parent_id,
    };
    let node = state.files.update(user.id, id, changes).await?;
    state.index_name(&node).await;
    let node = Node::from(node);
    state
        .hub
        .publish(user.id, Event::NodeUpdated { node: node.clone() });
    Ok(Json(node))
}

async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<NodeId>,
) -> Result<StatusCode, ApiError> {
    let node = state.files.delete(user.id, id).await?;
    state.hub.publish(
        user.id,
        Event::NodeDeleted {
            id: node.id,
            parent_id: node.parent_id,
        },
    );
    Ok(StatusCode::NO_CONTENT)
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}
