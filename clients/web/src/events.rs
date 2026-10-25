use std::{cell::Cell, cell::RefCell, rc::Rc};

use gloo_timers::future::TimeoutFuture;
use tokio::sync::oneshot;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{MessageEvent, WebSocket};

use api_client::ApiError;
use protocol::{Event, TicketRequest};

use crate::session;

const MIN_RETRY_MS: u32 = 1_000;
const MAX_RETRY_MS: u32 = 30_000;

type Handler = Rc<dyn Fn(Event)>;

/// Handle to the live-events connection; stop it when the page goes away.
pub struct Connection {
    stopped: Rc<Cell<bool>>,
    socket: Rc<RefCell<Option<WebSocket>>>,
}

impl Connection {
    /// Closes the socket and ends reconnecting.
    pub fn stop(&self) {
        self.stopped.set(true);
        if let Some(socket) = self.socket.borrow().as_ref() {
            let _ = socket.close();
        }
    }
}

struct Open {
    closed: oneshot::Receiver<()>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_close: Closure<dyn FnMut()>,
}

/// Keeps a websocket to the server open, reconnecting with backoff, and passes every
/// event to `on_event`. After a reconnect it sends [`Event::Resync`] since events may
/// have been missed meanwhile.
pub fn connect(on_event: impl Fn(Event) + 'static) -> Connection {
    let connection = Connection {
        stopped: Rc::new(Cell::new(false)),
        socket: Rc::new(RefCell::new(None)),
    };
    let handler: Handler = Rc::new(on_event);
    wasm_bindgen_futures::spawn_local(run(
        Rc::clone(&connection.stopped),
        Rc::clone(&connection.socket),
        handler,
    ));
    connection
}

async fn run(stopped: Rc<Cell<bool>>, socket: Rc<RefCell<Option<WebSocket>>>, handler: Handler) {
    let client = session::client();
    let mut delay = MIN_RETRY_MS;
    let mut reconnecting = false;
    while !stopped.get() {
        match client.ticket(&TicketRequest::Events).await {
            Ok(ticket) => {
                if let Some(open) = open(&client.events_url(&ticket.ticket), &handler, &socket) {
                    if reconnecting {
                        handler(Event::Resync);
                    }
                    reconnecting = true;
                    delay = MIN_RETRY_MS;
                    let _ = open.closed.await;
                }
            }
            Err(ApiError::Unauthorized) => break,
            Err(_) => {}
        }
        if stopped.get() {
            break;
        }
        TimeoutFuture::new(delay).await;
        delay = (delay * 2).min(MAX_RETRY_MS);
    }
}

fn open(url: &str, handler: &Handler, socket: &Rc<RefCell<Option<WebSocket>>>) -> Option<Open> {
    let ws = WebSocket::new(url).ok()?;
    let (closed_tx, closed) = oneshot::channel();
    let closed_tx = RefCell::new(Some(closed_tx));
    let handler = Rc::clone(handler);
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
        if let Some(text) = message.data().as_string()
            && let Ok(event) = serde_json::from_str::<Event>(&text)
        {
            handler(event);
        }
    });
    let on_close = Closure::<dyn FnMut()>::new(move || {
        if let Some(tx) = closed_tx.borrow_mut().take() {
            let _ = tx.send(());
        }
    });
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    *socket.borrow_mut() = Some(ws);
    Some(Open {
        closed,
        _on_message: on_message,
        _on_close: on_close,
    })
}
