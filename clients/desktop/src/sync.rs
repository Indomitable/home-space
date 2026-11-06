//! The sync engine (`sync-core`) on the desktop: files go to the system trash, a watcher reports
//! local changes, and it runs as an iced subscription.

use std::{
    any::Any,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use iced::{
    Subscription,
    futures::{SinkExt, Stream, channel::mpsc::Sender},
};
use notify::{RecursiveMode, Watcher};
use tokio::sync::mpsc;

use api_client::Client;
use protocol::UserId;
pub use sync_core::{
    Activity, Badge, Command, Commands, Fault, Item, ItemState, Mapping, Operation, Outage,
    Status, Summary, Update, part_path, read_range,
};
use sync_core::{Platform, Store};

use crate::session;

const BUFFER: usize = 64;

struct Desktop;

impl Platform for Desktop {
    fn protected_dir(&self) -> Option<PathBuf> {
        session::data_dir()
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        trash::delete(path).map_err(io::Error::other)
    }

    fn watch(
        &self,
        roots: &[&Path],
        changed: mpsc::UnboundedSender<()>,
    ) -> Option<Box<dyn Any + Send>> {
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok_and(|event| !event.kind.is_access()) {
                    let _ = changed.send(());
                }
            })
            .ok()?;
        for root in roots {
            // A folder that is missing now (an unmounted drive) is picked up by the rescan.
            let _ = watcher.watch(root, RecursiveMode::Recursive);
        }
        Some(Box::new(watcher))
    }
}

struct Account {
    client: Client,
    user: UserId,
}

impl Hash for Account {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.client.url("").hash(state);
        self.user.hash(state);
    }
}

/// Runs the sync for the signed-in `user` until the session ends.
pub fn subscription(client: &Client, user: UserId) -> Subscription<Update> {
    Subscription::run_with(
        Account {
            client: client.clone(),
            user,
        },
        |account| run(account.client.clone(), account.user),
    )
}

fn run(client: Client, user: UserId) -> impl Stream<Item = Update> {
    iced::stream::channel(BUFFER, async move |mut output: Sender<Update>| {
        let store = session::data_dir()
            .ok_or_else(|| "HOME is not set".to_owned())
            .and_then(|dir| {
                Store::open(&dir.join(format!("sync-{user}.db"))).map_err(|err| err.to_string())
            });
        match store {
            Ok(store) => sync_core::run(client, store, Arc::new(Desktop), output).await,
            Err(error) => {
                let status = sync_core::unavailable(&error);
                let _ = output.send(Update::Status(Arc::new(status))).await;
            }
        }
    })
}
