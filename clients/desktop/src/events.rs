use std::{
    hash::{Hash, Hasher},
    time::Duration,
};

use iced::{
    Subscription,
    futures::{SinkExt, Stream, StreamExt, channel::mpsc::Sender},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite};

use api_client::{ApiError, Client};
use protocol::{Event, TicketRequest};

const MIN_RETRY: Duration = Duration::from_secs(1);
const MAX_RETRY: Duration = Duration::from_secs(30);
const BUFFER: usize = 64;

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct Connection(Client);

impl Hash for Connection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.url("").hash(state);
    }
}

/// Live events from the server of `client`, reconnecting with backoff.
///
/// Every connection after the first attempt is preceded by [`Event::Resync`], since events
/// may have been missed meanwhile. The stream ends once the session is no longer valid.
pub fn subscription(client: &Client) -> Subscription<Event> {
    Subscription::run_with(Connection(client.clone()), |connection| {
        listen(connection.0.clone())
    })
}

fn listen(client: Client) -> impl Stream<Item = Event> {
    iced::stream::channel(BUFFER, async move |mut output: Sender<Event>| {
        let mut delay = MIN_RETRY;
        let mut reconnecting = false;
        loop {
            match client.ticket(&TicketRequest::Events).await {
                Ok(ticket) => {
                    if let Ok((socket, _)) = connect_async(client.events_url(&ticket.ticket)).await
                    {
                        delay = MIN_RETRY;
                        let resynced = !reconnecting || output.send(Event::Resync).await.is_ok();
                        if !resynced || !forward(socket, &mut output).await {
                            break;
                        }
                    }
                }
                Err(ApiError::Unauthorized) => break,
                Err(_) => {}
            }
            reconnecting = true;
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(MAX_RETRY);
        }
    })
}

/// Passes events on until the socket closes; `false` once nobody listens anymore.
async fn forward(mut socket: Socket, output: &mut Sender<Event>) -> bool {
    while let Some(Ok(message)) = socket.next().await {
        if let tungstenite::Message::Text(text) = message
            && let Ok(event) = serde_json::from_str::<Event>(&text)
            && output.send(event).await.is_err()
        {
            return false;
        }
    }
    true
}
