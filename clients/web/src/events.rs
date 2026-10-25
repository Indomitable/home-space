use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

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

#[derive(Default)]
struct HubState {
    connection: RefCell<Option<Connection>>,
    subscribers: RefCell<Vec<(u64, Handler)>>,
    next_id: Cell<u64>,
}

/// One live-events connection shared by every page; pages come and go via
/// [`Hub::subscribe`] without reconnecting.
#[derive(Clone, Default)]
pub struct Hub(Rc<HubState>);

impl Hub {
    /// Opens the connection unless it is already open.
    pub fn start(&self) {
        if self.0.connection.borrow().is_some() {
            return;
        }
        let state = Rc::downgrade(&self.0);
        let connection = connect(move |event| {
            let Some(state) = state.upgrade() else {
                return;
            };
            let handlers: Vec<Handler> = state
                .subscribers
                .borrow()
                .iter()
                .map(|(_, handler)| Rc::clone(handler))
                .collect();
            for handler in handlers {
                handler(event.clone());
            }
        });
        *self.0.connection.borrow_mut() = Some(connection);
    }

    /// Closes the connection; subscribers stay registered.
    pub fn stop(&self) {
        if let Some(connection) = self.0.connection.borrow_mut().take() {
            connection.stop();
        }
    }

    /// Passes every event to `on_event` until the returned handle is dropped.
    pub fn subscribe(&self, on_event: impl Fn(Event) + 'static) -> Subscription {
        let id = self.0.next_id.get();
        self.0.next_id.set(id + 1);
        self.0
            .subscribers
            .borrow_mut()
            .push((id, Rc::new(on_event)));
        Subscription {
            state: Rc::downgrade(&self.0),
            id,
        }
    }
}

/// Keeps a handler registered with a [`Hub`]; dropping it unsubscribes.
pub struct Subscription {
    state: Weak<HubState>,
    id: u64,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            state
                .subscribers
                .borrow_mut()
                .retain(|(id, _)| *id != self.id);
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
