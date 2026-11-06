//! Offline copies: a file that was opened, or a file or folder made available offline, is kept
//! under `<files>/<user id>/<node id>/` and synced both ways by the engine in `sync-core`.
//!
//! The engine runs for as long as the client does. The app opens its [`Gate`] when the settings and
//! the network allow automatic transfers. Opening a file always downloads, whatever the gate says.
//! See `docs/specs/mobile-sync.feature`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use api_client::Client;
use futures::StreamExt;
use futures::channel::mpsc;
use protocol::{Node, NodeId, NodeKind};
use sync_core::{Base, Command, Commands, Gate, Mapping, PrivateFiles, Status, Store, Update};
use tokio::sync::watch;

use crate::autoupload::AutoUploadListener;
use crate::files::{MarkedFile, storage};
use crate::{CoreClient, CoreError, Entry, EntryKind, RUNTIME, off_ui_thread, parse_id};

const DATABASE: &str = "sync.db";
/// Downloads started by opening a file, on the same file system as the copies. Not synced.
const PARTS: &str = ".parts";
const UPDATES: usize = 64;

/// Where a file or folder stands on this phone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum OfflineState {
    /// The copy here matches the server.
    Synced,
    /// There is work ahead: it waits for a pass, is transferring, or has edits to send.
    Updating,
    /// The last attempt failed. It is tried again later.
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OfflineProblem {
    pub name: String,
    pub reason: String,
}

/// How offline copies are doing as a whole.
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct OfflineStatus {
    /// Files and folders made available offline, or opened.
    pub kept: u64,
    /// Files and folders with work ahead.
    pub pending: u64,
    pub problems: Vec<OfflineProblem>,
    /// The server can't be reached. The sync tries again by itself.
    pub unreachable: bool,
}

/// The running engine of the signed-in user.
pub(crate) struct Offline {
    dir: PathBuf,
    pub(crate) store: Arc<Store>,
    commands: Commands,
    gate: Gate,
    status: watch::Receiver<Arc<Status>>,
    settled: watch::Receiver<u64>,
    tickets: AtomicU64,
    engine: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Offline {
    /// Starts the engine for what is kept in `dir`. Its gate stays closed until the app opens it.
    pub(crate) async fn start(
        client: Client,
        dir: PathBuf,
        rescue: Option<NodeId>,
        listener: Arc<Mutex<Option<Arc<dyn AutoUploadListener>>>>,
    ) -> Result<Arc<Self>, CoreError> {
        std::fs::create_dir_all(&dir).map_err(storage)?;
        let store =
            Arc::new(
                Store::open(&dir.join(DATABASE)).map_err(|err| CoreError::Failed {
                    code: "local_storage".to_owned(),
                    detail: err.to_string(),
                })?,
            );
        let gate = Gate::default();
        gate.set_closed(true);
        let (output, mut updates) = mpsc::channel(UPDATES);
        let engine = RUNTIME.spawn(sync_core::run(
            client,
            store.clone(),
            Arc::new(PrivateFiles { rescue }),
            gate.clone(),
            output,
        ));
        let Some(Update::Ready(commands)) = updates.next().await else {
            return Err(CoreError::Failed {
                code: "local_storage".to_owned(),
                detail: "the sync could not start".to_owned(),
            });
        };
        // The engine reports once as it starts, so that what is kept is known from here on.
        let first = match updates.next().await {
            Some(Update::Status(status)) => status,
            _ => Arc::default(),
        };
        let (status_sender, status) = watch::channel(first);
        let (settled_sender, settled) = watch::channel(0);
        RUNTIME.spawn(async move {
            while let Some(update) = updates.next().await {
                match update {
                    Update::Status(status) => {
                        status_sender.send_replace(status);
                        let listener = listener.lock().expect("offline listener lock").clone();
                        if let Some(listener) = listener {
                            listener.changed();
                        }
                    }
                    Update::Settled(ticket) => {
                        settled_sender.send_replace(ticket);
                    }
                    Update::Ready(_) | Update::Rejected(_) => {}
                }
            }
        });
        Ok(Arc::new(Self {
            dir,
            store,
            commands,
            gate,
            status,
            settled,
            tickets: AtomicU64::new(0),
            engine: Mutex::new(Some(engine)),
        }))
    }

    pub(crate) fn status(&self) -> Arc<Status> {
        self.status.borrow().clone()
    }

    /// The folder of a file or folder kept for its own sake.
    fn root_of(&self, id: NodeId) -> PathBuf {
        self.dir.join(id.to_string())
    }

    pub(crate) fn part_path(&self, id: NodeId) -> Result<PathBuf, CoreError> {
        let parts = self.dir.join(PARTS);
        std::fs::create_dir_all(&parts).map_err(storage)?;
        Ok(parts.join(id.to_string()))
    }

    fn wake(&self) -> u64 {
        let ticket = self.tickets.fetch_add(1, Ordering::Relaxed) + 1;
        self.commands.send(Command::Wake(ticket));
        ticket
    }

    /// The file a copy of `id` is at, if there is one.
    pub(crate) fn existing_copy(&self, id: NodeId) -> Option<PathBuf> {
        let recorded = self
            .store
            .base(id)
            .ok()
            .flatten()
            .filter(|base| base.kind == NodeKind::File)
            .map(|base| base.path);
        recorded
            .or_else(|| self.status().file_path(id).map(Path::to_path_buf))
            .filter(|path| path.is_file())
    }

    /// Where the file `node` goes: where the sync keeps it, else a folder of its own, under its
    /// current name. A copy kept for its own sake is renamed along.
    pub(crate) fn place_for(&self, node: &Node) -> Result<PathBuf, CoreError> {
        let own = self.root_of(node.id);
        let name = sync_core::local_name(&node.name);
        let target = own.join(&name);
        if let Some(existing) = self.existing_copy(node.id) {
            if existing.parent() != Some(own.as_path()) || existing == target {
                return Ok(existing);
            }
            std::fs::rename(&existing, &target).map_err(storage)?;
            if let Ok(Some(base)) = self.store.base(node.id) {
                let moved = Base {
                    path: target.clone(),
                    ..base
                };
                self.store.put_base(&moved).map_err(database)?;
            }
            return Ok(target);
        }
        if let Some(path) = self.status().file_path(node.id) {
            return Ok(path.to_path_buf());
        }
        std::fs::create_dir_all(&own).map_err(storage)?;
        Ok(target)
    }

    /// Keeps `node` in sync at `target`, unless something it is in does so already.
    pub(crate) fn keep(&self, node: &Node, target: &Path) {
        if target.parent() != Some(self.root_of(node.id).as_path()) {
            return;
        }
        self.pin(node.id, Some(node));
    }

    fn pin(&self, id: NodeId, known: Option<&Node>) {
        self.commands.send(Command::Pin {
            mapping: Mapping {
                remote: Some(id),
                local: Some(self.root_of(id)),
            },
            known: known.cloned().map(Into::into),
        });
    }

    fn is_kept_itself(status: &Status, id: NodeId) -> bool {
        status
            .mapping(Some(id))
            .is_some_and(|mapping| mapping.local.is_some())
    }

    /// Stops keeping `id` for its own sake, and removes its copy. The gate closes meanwhile so that
    /// a running transfer lets the engine get to it.
    pub(crate) async fn remove(&self, id: NodeId) {
        let root = self.root_of(id);
        if !root.exists() {
            return;
        }
        let closed = self.gate.is_closed();
        self.gate.set_closed(true);
        self.commands.send(Command::Purge(id));
        // Commands run in order, so by the time this one has settled the purge is done.
        let ticket = self.wake();
        let mut settled = self.settled.clone();
        let _ = settled.wait_for(|seen| *seen >= ticket).await;
        self.gate.set_closed(closed);
        if !closed {
            self.wake();
        }
        // A copy the engine doesn't know, or an engine that has ended.
        let _ = std::fs::remove_dir_all(root);
    }

    /// Fills in what is known here of `entries`, going by the engine's status and the state of the
    /// copies. Reads no more than the phone. With `fresh`, the entries have just come from the
    /// server, so a copy can be compared with them.
    pub(crate) fn mark(&self, entries: &mut [Entry], fresh: bool) {
        let status = self.status();
        let bases: HashMap<NodeId, Base> = self
            .store
            .bases()
            .unwrap_or_default()
            .into_iter()
            .map(|base| (base.node, base))
            .collect();
        for entry in entries.iter_mut() {
            let Ok(id) = entry.id.parse::<NodeId>() else {
                continue;
            };
            let kept = Self::is_kept_itself(&status, id);
            let reported = match status.badge(Some(id)) {
                Some(sync_core::Badge::Problem) => Some(OfflineState::Failed),
                Some(sync_core::Badge::Pending) => Some(OfflineState::Updating),
                Some(sync_core::Badge::Synced) => Some(OfflineState::Synced),
                Some(sync_core::Badge::Disabled) | None => None,
            };
            entry.offline_kept = kept;
            entry.local_path = None;
            let mut state = reported;
            if entry.kind == EntryKind::File {
                let base = bases.get(&id).filter(|base| base.kind == NodeKind::File);
                let copy = base
                    .map(|base| base.path.clone())
                    .or_else(|| status.file_path(id).map(Path::to_path_buf))
                    .filter(|path| path.is_file());
                entry.local_path = copy
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned());
                if copy.is_some()
                    && state != Some(OfflineState::Failed)
                    && let Some(base) = base
                {
                    // The engine may not have looked since: the gate can be closed for hours.
                    let behind = fresh && base.sha256 != entry.sha256;
                    state = Some(if base.is_edited() || behind {
                        OfflineState::Updating
                    } else {
                        reported.unwrap_or(OfflineState::Synced)
                    });
                }
            }
            if kept && state.is_none() {
                state = Some(OfflineState::Updating);
            }
            entry.offline_state = state;
        }
    }

    fn summary(&self) -> OfflineStatus {
        let status = self.status();
        let name_of = |path: &Path| {
            path.file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
        };
        OfflineStatus {
            kept: status
                .mappings
                .iter()
                .filter(|mapping| mapping.local.is_some())
                .count() as u64,
            pending: status.pending as u64,
            problems: status
                .problems
                .iter()
                .map(|(path, fault)| OfflineProblem {
                    name: name_of(path),
                    reason: fault.reason.clone(),
                })
                .collect(),
            unreachable: status.outage.is_some(),
        }
    }

    /// The names of the files with edits not on the server yet, under `id` if it is given.
    fn unsent_edits(&self, id: Option<NodeId>) -> Vec<String> {
        let root = id.map(|id| self.root_of(id));
        self.store
            .bases()
            .unwrap_or_default()
            .into_iter()
            .filter(|base| root.as_ref().is_none_or(|root| base.path.starts_with(root)))
            .filter(Base::is_edited)
            .filter_map(|base| {
                let name = base.path.file_name()?;
                Some(name.to_string_lossy().into_owned())
            })
            .collect()
    }

    /// Ends the engine, for signing out. Returns the uploads under way on the server.
    pub(crate) async fn shut_down(&self) -> Vec<uuid::Uuid> {
        self.gate.set_closed(true);
        self.commands.send(Command::Shutdown);
        let engine = self.engine.lock().expect("offline engine lock").take();
        if let Some(engine) = engine {
            let _ = engine.await;
        }
        self.store
            .saved_uploads()
            .unwrap_or_default()
            .into_iter()
            .map(|upload| upload.id)
            .collect()
    }
}

fn database(err: impl std::fmt::Display) -> CoreError {
    CoreError::Failed {
        code: "local_storage".to_owned(),
        detail: err.to_string(),
    }
}

/// Records that the copy at `path` holds what the server calls `sha256`, as of now.
pub(crate) fn record_copy(
    offline: &Offline,
    id: NodeId,
    path: &Path,
    sha256: String,
) -> Result<(), CoreError> {
    let meta = std::fs::metadata(path).map_err(storage)?;
    offline
        .store
        .put_base(&Base {
            node: id,
            path: path.to_path_buf(),
            kind: NodeKind::File,
            sha256: Some(sha256),
            size: meta.len(),
            mtime: meta.modified().map(sync_core::nanos).unwrap_or(0),
        })
        .map_err(database)
}

/// What opening `node` should do, given its copy at `target`.
pub(crate) fn decide(offline: &Offline, node: &Node, target: &Path) -> MarkedFile {
    if !target.is_file() {
        return MarkedFile::Fetch;
    }
    let base = offline.store.base(node.id).ok().flatten();
    if let Some(base) = base.filter(|base| base.path == target && !base.is_edited()) {
        return if base.sha256 == node.sha256 {
            MarkedFile::Current
        } else {
            MarkedFile::Fetch
        };
    }
    // Nothing says this is what the server holds, so it may be edits: only the same content can be
    // recorded. Otherwise the sync sorts out which is current, and nothing is overwritten here.
    let stop = std::sync::atomic::AtomicBool::new(false);
    match sync_core::hash_file(target, &stop) {
        Ok(sha256) if Some(&sha256) == node.sha256.as_ref() => MarkedFile::SameContent(sha256),
        _ => MarkedFile::Edited,
    }
}

#[uniffi::export]
impl CoreClient {
    /// Who to tell when what offline copies report may have changed.
    pub fn set_offline_listener(&self, listener: Arc<dyn AutoUploadListener>) {
        self.files.set_offline_listener(listener);
    }

    /// Lets automatic transfers of offline copies run, or stops them at their next chunk. The app
    /// calls this as the network and the settings change. It starts with them stopped.
    pub async fn offline_allow(&self, allowed: bool) -> Result<(), CoreError> {
        let (files, client) = (self.files.clone(), self.inner.clone());
        off_ui_thread(async move {
            let offline = files.offline(&client).await?;
            offline.gate.set_closed(!allowed);
            if allowed {
                offline.wake();
            }
            Ok(())
        })
        .await
    }

    /// Looks for work now, such as when a viewer was closed. With `retry_failed`, what is waiting
    /// to be tried again goes too.
    pub async fn offline_sync_now(&self, retry_failed: bool) -> Result<(), CoreError> {
        let (files, client) = (self.files.clone(), self.inner.clone());
        off_ui_thread(async move {
            let offline = files.offline(&client).await?;
            if retry_failed {
                offline.commands.send(Command::Retry(None));
            }
            offline.wake();
            Ok(())
        })
        .await
    }

    /// Syncs until nothing is left to do for now, for a background job. Calling
    /// [`CoreClient::offline_allow`] with `false` ends it early.
    pub async fn offline_pass(&self) -> Result<(), CoreError> {
        let (files, client) = (self.files.clone(), self.inner.clone());
        off_ui_thread(async move {
            let offline = files.offline(&client).await?;
            offline.gate.set_closed(false);
            let ticket = offline.wake();
            let mut settled = offline.settled.clone();
            let _ = settled.wait_for(|seen| *seen >= ticket).await;
            Ok(())
        })
        .await
    }

    /// Keeps a file, or a folder with what is in it, on this phone and in sync. Anything to download
    /// waits until automatic transfers are allowed.
    pub async fn make_available_offline(&self, id: String) -> Result<(), CoreError> {
        let id = parse_id(&id)?;
        let (files, client) = (self.files.clone(), self.inner.clone());
        off_ui_thread(async move {
            let node = client.get(id).await?;
            let offline = files.offline(&client).await?;
            std::fs::create_dir_all(offline.root_of(id)).map_err(storage)?;
            // A folder's content is read by the engine, in its next pass.
            let known = (node.kind == NodeKind::File).then_some(&node);
            offline.pin(id, known);
            offline.wake();
            Ok(())
        })
        .await
    }

    /// Deletes the copy of a file or folder kept for its own sake. It stays on the server. Check
    /// [`CoreClient::unsent_edits`] first: edits not sent yet are lost.
    pub async fn remove_offline_copy(&self, id: String) -> Result<(), CoreError> {
        let id = parse_id(&id)?;
        let (files, client) = (self.files.clone(), self.inner.clone());
        off_ui_thread(async move {
            files.offline(&client).await?.remove(id).await;
            Ok(())
        })
        .await
    }

    /// The names of the files with edits that would be lost by removing the copy of `id`, or by
    /// signing out when `id` is `None`.
    pub async fn unsent_edits(&self, id: Option<String>) -> Result<Vec<String>, CoreError> {
        let id = id.as_deref().map(parse_id).transpose()?;
        let (files, client) = (self.files.clone(), self.inner.clone());
        off_ui_thread(async move { Ok(files.offline(&client).await?.unsent_edits(id)) }).await
    }

    pub async fn offline_status(&self) -> Result<OfflineStatus, CoreError> {
        let (files, client) = (self.files.clone(), self.inner.clone());
        off_ui_thread(async move { Ok(files.offline(&client).await?.summary()) }).await
    }

    /// `entries` with their local state brought up to date. Asks nothing of the server.
    pub async fn refresh_offline(&self, entries: Vec<Entry>) -> Vec<Entry> {
        let (files, client) = (self.files.clone(), self.inner.clone());
        off_ui_thread(async move {
            let mut entries = entries;
            if let Ok(offline) = files.offline(&client).await {
                offline.mark(&mut entries, false);
            }
            entries
        })
        .await
    }
}
