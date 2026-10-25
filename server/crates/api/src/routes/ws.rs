use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::HeaderMap,
    response::Response,
    routing::get,
};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use notifications::Event;
use protocol::TicketRequest;

use crate::{
    ApiError, AppState,
    extract::{AuthUser, TicketQuery},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(upgrade))
}

async fn upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TicketQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let (user_id, lifetime) = match query.ticket {
        Some(ticket) => (
            state.auth.verify_ticket(&ticket, &TicketRequest::Events)?,
            state.auth.access_ttl().unsigned_abs(),
        ),
        None => {
            let user = AuthUser::from_headers(&headers, &state)?;
            (user.id, remaining_lifetime(user.token_exp))
        }
    };
    let events = state.hub.subscribe(user_id);
    Ok(ws.on_upgrade(move |socket| session(socket, events, lifetime)))
}

fn remaining_lifetime(token_exp: i64) -> Duration {
    let expires_at = UNIX_EPOCH + Duration::from_secs(token_exp.unsigned_abs());
    expires_at
        .duration_since(SystemTime::now())
        .unwrap_or(Duration::ZERO)
}

async fn session(mut socket: WebSocket, mut events: Receiver<Event>, lifetime: Duration) {
    let expiry = tokio::time::sleep(lifetime);
    tokio::pin!(expiry);
    loop {
        let outgoing = tokio::select! {
            () = &mut expiry => break,
            event = events.recv() => match event {
                Ok(event) => serde_json::to_string(&event).ok(),
                Err(RecvError::Lagged(_)) => serde_json::to_string(&Event::Resync).ok(),
                Err(RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                Some(Ok(_)) => None,
            },
        };
        if let Some(text) = outgoing
            && socket.send(Message::text(text)).await.is_err()
        {
            break;
        }
    }
}
