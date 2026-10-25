use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
    routing::get,
};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use notifications::Event;

use crate::{AppState, extract::AuthUser};

const RESYNC_MESSAGE: &str = r#"{"type":"resync"}"#;

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(upgrade))
}

async fn upgrade(State(state): State<AppState>, user: AuthUser, ws: WebSocketUpgrade) -> Response {
    let events = state.hub.subscribe(user.id);
    let lifetime = remaining_lifetime(user.token_exp);
    ws.on_upgrade(move |socket| session(socket, events, lifetime))
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
                Err(RecvError::Lagged(_)) => Some(RESYNC_MESSAGE.to_owned()),
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
