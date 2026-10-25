//! Per-user live update events delivered over websockets.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use tokio::sync::broadcast;

use db::UserId;

pub use protocol::Event;

const CHANNEL_CAPACITY: usize = 256;

/// Fan-out of events to the connected sessions of each user.
#[derive(Clone, Debug, Default)]
pub struct Hub {
    channels: Arc<Mutex<HashMap<UserId, broadcast::Sender<Event>>>>,
}

impl Hub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribes to the events of `user`.
    pub fn subscribe(&self, user: UserId) -> broadcast::Receiver<Event> {
        self.lock()
            .entry(user)
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
            .subscribe()
    }

    /// Sends `event` to every session of `user`; a no-op when nobody listens.
    pub fn publish(&self, user: UserId, event: Event) {
        let mut channels = self.lock();
        let delivered = channels
            .get(&user)
            .is_some_and(|sender| sender.send(event).is_ok());
        if !delivered {
            channels.remove(&user);
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<UserId, broadcast::Sender<Event>>> {
        self.channels.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use db::NodeId;

    use super::*;

    fn deleted() -> Event {
        Event::NodeDeleted {
            id: NodeId::new(),
            parent_id: None,
        }
    }

    #[tokio::test]
    async fn subscriber_receives_events_for_its_user() {
        let hub = Hub::new();
        let user = UserId::new();
        let mut rx = hub.subscribe(user);
        let event = deleted();
        hub.publish(user, event.clone());
        assert_eq!(rx.recv().await.expect("event"), event);
    }

    #[tokio::test]
    async fn other_users_do_not_receive_events() {
        let hub = Hub::new();
        let mut other = hub.subscribe(UserId::new());
        hub.publish(UserId::new(), deleted());
        assert!(other.try_recv().is_err());
    }

    #[test]
    fn publishing_without_subscribers_drops_the_channel() {
        let hub = Hub::new();
        let user = UserId::new();
        drop(hub.subscribe(user));
        hub.publish(user, deleted());
        assert!(hub.lock().is_empty());
    }
}
