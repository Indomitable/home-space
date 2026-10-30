//! Two-way sync of mapped remote folders with local folders.
//!
//! Every pass compares three states per path: the remote tree (kept current from the
//! server's change journal), a scan of the local folders and the bases recorded when both
//! last agreed (see [`plan`]). Passes run on start, after local file changes, after server
//! events and every few minutes.

mod apply;
#[cfg(test)]
mod e2e;
mod layout;
mod model;
mod plan;
mod remote;
mod scan;
mod status;
mod store;

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use iced::{
    Subscription,
    futures::{SinkExt, Stream, StreamExt, channel::mpsc::Sender},
};
use notify::{RecursiveMode, Watcher};
use tokio::{sync::mpsc, time::Instant};

use api_client::{ApiError, Client};
use protocol::{NodeId, UserId};

use self::{
    apply::{Effect, Failure, Folders},
    layout::Layout,
    plan::{Action, Phase},
    remote::{Delta, Mirror},
    scan::Scan,
    status::Badges,
    store::Store,
};
pub use self::{model::Mapping, status::Badge};
use crate::session;

const BUFFER: usize = 64;
const TRANSFERS: usize = 4;
/// Local changes come in bursts; a pass waits for the burst to settle.
const SETTLE: Duration = Duration::from_secs(1);
/// Catches what neither the file watcher nor the server events reported.
const RESCAN: Duration = Duration::from_secs(300);
const RETRY: Duration = Duration::from_secs(30);
const MAX_BACKOFF: Duration = Duration::from_secs(30 * 60);
/// How often badges are refreshed while a pass is transferring.
const PROGRESS: Duration = Duration::from_millis(500);

/// What the sync reports to the app.
#[derive(Debug, Clone)]
pub enum Update {
    /// The engine started; send it commands through this.
    Ready(Commands),
    Status(Arc<Status>),
    /// A requested mapping was not made.
    Rejected(String),
}

/// A snapshot of the sync for the interface.
#[derive(Debug, Clone, Default)]
pub struct Status {
    /// Badge of every synced node; Home is under `None`.
    pub badges: Badges,
    pub mappings: Vec<Mapping>,
    /// Names of the mapped remote folders, as far as they are known.
    pub names: HashMap<NodeId, String>,
    /// Paths with work ahead.
    pub pending: usize,
    /// Paths that failed, with why.
    pub problems: Vec<(PathBuf, String)>,
    /// Why the last pass could not run, such as the server being unreachable.
    pub error: Option<String>,
}

impl Status {
    pub fn badge(&self, node: Option<NodeId>) -> Option<Badge> {
        self.badges.get(&node).copied()
    }

    /// The mapping set on `node` itself, if any.
    pub fn mapping(&self, node: Option<NodeId>) -> Option<&Mapping> {
        self.mappings.iter().find(|mapping| mapping.remote == node)
    }

    /// The state of the sync as a whole: the worst of all synced nodes, `None` while
    /// nothing is synced.
    pub fn overall(&self) -> Option<Badge> {
        if self.error.is_some() || !self.problems.is_empty() {
            return Some(Badge::Problem);
        }
        if self.pending > 0 {
            return Some(Badge::Pending);
        }
        self.mappings
            .iter()
            .any(|mapping| mapping.local.is_some())
            .then_some(Badge::Synced)
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    /// Syncs a remote folder to a local folder, or sets it to not sync.
    Map(Mapping),
    /// Removes the mapping of a remote folder; its local files stay.
    Unmap(Option<NodeId>),
    /// The server reported a change.
    RemoteChanged,
}

/// Sends commands to the running engine.
#[derive(Debug, Clone)]
pub struct Commands(mpsc::UnboundedSender<Command>);

impl Commands {
    pub fn send(&self, command: Command) {
        let _ = self.0.send(command);
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
        let opened = session::data_dir()
            .ok_or_else(|| "HOME is not set".to_owned())
            .and_then(|dir| {
                Store::open(&dir.join(format!("sync-{user}.db"))).map_err(|err| err.to_string())
            })
            .and_then(|store| Engine::load(client, store).map_err(|err| err.to_string()));
        let mut engine = match opened {
            Ok(engine) => engine,
            Err(error) => {
                let status = Status {
                    error: Some(format!("sync is unavailable: {error}")),
                    ..Status::default()
                };
                let _ = output.send(Update::Status(Arc::new(status))).await;
                return;
            }
        };
        let (commands, mut received) = mpsc::unbounded_channel();
        if output
            .send(Update::Ready(Commands(commands)))
            .await
            .is_err()
        {
            return;
        }
        let (changed, mut local_changes) = mpsc::unbounded_channel();
        let mut watcher = engine.watch(&changed);
        let mut due = Some(Instant::now());
        loop {
            let wake = due.unwrap_or_else(|| Instant::now() + RESCAN);
            tokio::select! {
                command = received.recv() => {
                    let Some(command) = command else { break };
                    if engine.handle(command, &mut output).await {
                        watcher = engine.watch(&changed);
                    }
                    due = Some(due.map_or(Instant::now(), |at| at.min(Instant::now())));
                }
                Some(()) = local_changes.recv() => {
                    due.get_or_insert_with(|| Instant::now() + SETTLE);
                }
                () = tokio::time::sleep_until(wake) => {
                    match engine.pass(&mut output).await {
                        Ok(next) => due = next.map(|delay| Instant::now() + delay),
                        Err(SignedOut) => break,
                    }
                    // Our own writes during the pass were reported too; they need no pass.
                    while local_changes.try_recv().is_ok() {}
                }
            }
        }
        drop(watcher);
    })
}

/// The session ended; the engine stops.
struct SignedOut;

/// A path that failed and when it may be tried again.
#[derive(Debug, Clone)]
struct Problem {
    reason: String,
    attempts: u32,
    retry_at: Instant,
}

struct Engine {
    client: Client,
    store: Store,
    mirror: Mirror,
    mappings: Vec<Mapping>,
    /// Set when the mapped folders must be read from scratch.
    reload: bool,
    problems: HashMap<PathBuf, Problem>,
}

impl Engine {
    fn load(client: Client, store: Store) -> rusqlite::Result<Self> {
        let mirror = Mirror {
            nodes: store
                .remote_nodes()?
                .into_iter()
                .map(|node| (node.id, node))
                .collect(),
            cursor: store.cursor()?,
        };
        Ok(Self {
            client,
            mappings: store.mappings()?,
            reload: mirror.cursor.is_none(),
            mirror,
            store,
            problems: HashMap::new(),
        })
    }

    /// Watches the mapped local folders; changes are reported as `()` on `changed`.
    fn watch(&self, changed: &mpsc::UnboundedSender<()>) -> Option<notify::RecommendedWatcher> {
        let changed = changed.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok_and(|event| !event.kind.is_access()) {
                    let _ = changed.send(());
                }
            })
            .ok()?;
        for root in self.mappings.iter().filter_map(|m| m.local.as_deref()) {
            // A folder that is missing now (an unmounted drive) is picked up by the rescan.
            let _ = watcher.watch(root, RecursiveMode::Recursive);
        }
        Some(watcher)
    }

    /// Applies a command; returns whether the watched folders changed.
    async fn handle(&mut self, command: Command, output: &mut Sender<Update>) -> bool {
        match command {
            Command::RemoteChanged => false,
            Command::Map(mapping) => {
                if let Some(local) = &mapping.local {
                    let others: Vec<&Path> = self
                        .mappings
                        .iter()
                        .filter(|m| m.remote != mapping.remote)
                        .filter_map(|m| m.local.as_deref())
                        .collect();
                    let data = session::data_dir();
                    if let Some(reason) = layout::conflict(local, &others, data.as_deref()) {
                        let _ = output.send(Update::Rejected(reason)).await;
                        return false;
                    }
                }
                if let Err(err) = self.store.set_mapping(&mapping) {
                    let _ = output.send(Update::Rejected(err.to_string())).await;
                    return false;
                }
                self.mappings.retain(|m| m.remote != mapping.remote);
                self.mappings.push(mapping);
                self.reload = true;
                true
            }
            Command::Unmap(remote) => {
                if let Err(err) = self.store.remove_mapping(remote) {
                    let _ = output.send(Update::Rejected(err.to_string())).await;
                    return false;
                }
                self.mappings.retain(|m| m.remote != remote);
                true
            }
        }
    }

    /// Brings everything mapped in sync once; returns when the next pass is due unless
    /// local or remote changes call for one earlier.
    async fn pass(&mut self, output: &mut Sender<Update>) -> Result<Option<Duration>, SignedOut> {
        if self.mappings.iter().all(|m| m.local.is_none()) {
            self.report(output, &Layout::default(), &[], None).await;
            return Ok(None);
        }
        let refreshed = if self.reload {
            self.mirror.reload(&self.client, &self.mappings).await
        } else {
            self.mirror.refresh(&self.client, &self.mappings).await
        };
        match refreshed {
            Ok(delta) => {
                self.reload = false;
                self.record_remote(&delta);
            }
            Err(ApiError::Unauthorized) => return Err(SignedOut),
            Err(err) => {
                let layout = Layout::new(&self.mirror.nodes, &self.mappings);
                self.report(
                    output,
                    &layout,
                    &[],
                    Some(format!("cannot reach the server: {err}")),
                )
                .await;
                return Ok(Some(RETRY));
            }
        }
        let layout = Layout::new(&self.mirror.nodes, &self.mappings);
        let bases = self.store.bases().unwrap_or_default();
        let scan = self.scan(&layout, &bases).await;
        let now = Instant::now();
        let planned = plan::plan(&layout, &scan, &bases);
        // A path with nothing left to do was resolved meanwhile, perhaps by the user.
        self.problems.retain(|path, _| {
            planned
                .iter()
                .any(|action| action.path() == Some(path.as_path()))
        });
        let actions: Vec<Action> = planned
            .into_iter()
            .filter(|action| {
                action.path().is_none_or(|path| {
                    self.problems
                        .get(path)
                        .is_none_or(|problem| problem.retry_at <= now)
                })
            })
            .collect();
        self.report(output, &layout, &actions, None).await;
        if actions.is_empty() {
            return Ok(self.next_retry());
        }
        self.execute(output, &layout, actions).await?;
        Ok(Some(SETTLE))
    }

    async fn scan(&self, layout: &Layout, bases: &[model::Base]) -> Scan {
        let roots = layout.roots.clone();
        let skipped = layout.skipped.clone();
        let known = bases
            .iter()
            .map(|base| (base.path.clone(), base.clone()))
            .collect();
        tokio::task::spawn_blocking(move || scan::scan(&roots, &skipped, &known))
            .await
            .unwrap_or_else(|_| Scan {
                blind: layout.roots.clone(),
                ..Scan::default()
            })
    }

    /// Runs the actions phase by phase; transfers run several at a time.
    async fn execute(
        &mut self,
        output: &mut Sender<Update>,
        layout: &Layout,
        actions: Vec<Action>,
    ) -> Result<(), SignedOut> {
        let mut folders: Folders = layout.folders.clone();
        // Everything not finished yet, which the badges show as pending.
        let mut unfinished = actions.clone();
        let mut reported = Instant::now();
        let finish = |unfinished: &mut Vec<Action>, action: &Action| {
            if let Some(at) = unfinished.iter().position(|other| other == action) {
                unfinished.swap_remove(at);
            }
        };
        for phase in [Phase::Move, Phase::Create, Phase::Transfer, Phase::Delete] {
            let now: Vec<Action> = actions
                .iter()
                .filter(|action| action.phase() == phase)
                .cloned()
                .collect();
            if phase == Phase::Transfer {
                let snapshot = folders.clone();
                let client = self.client.clone();
                let mut outcomes = apply::execute_all(&client, &snapshot, now, TRANSFERS);
                while let Some((action, outcome)) = outcomes.next().await {
                    finish(&mut unfinished, &action);
                    self.settle(&mut folders, action, outcome)?;
                    if reported.elapsed() >= PROGRESS {
                        reported = Instant::now();
                        self.report(output, layout, &unfinished, None).await;
                    }
                }
            } else {
                // One at a time, so each action sees the folders the previous ones created.
                for action in now {
                    let outcome = apply::execute(&self.client, &folders, action.clone()).await;
                    finish(&mut unfinished, &action);
                    self.settle(&mut folders, action, outcome)?;
                }
            }
            if reported.elapsed() >= PROGRESS || phase == Phase::Delete {
                reported = Instant::now();
                self.report(output, layout, &unfinished, None).await;
            }
        }
        Ok(())
    }

    fn settle(
        &mut self,
        folders: &mut Folders,
        action: Action,
        outcome: Result<Vec<Effect>, Failure>,
    ) -> Result<(), SignedOut> {
        let path = action.path().map(Path::to_path_buf);
        match outcome {
            Ok(effects) => {
                if let Some(path) = &path {
                    self.problems.remove(path);
                }
                for effect in effects {
                    self.record(folders, effect);
                }
            }
            Err(Failure::SignedOut) => return Err(SignedOut),
            Err(Failure::Stale) => {}
            Err(Failure::Error(reason)) => {
                if let Some(path) = path {
                    let attempts = self.problems.get(&path).map_or(1, |p| p.attempts + 1);
                    let backoff = RETRY
                        .saturating_mul(2u32.saturating_pow(attempts - 1))
                        .min(MAX_BACKOFF);
                    self.problems.insert(
                        path,
                        Problem {
                            reason,
                            attempts,
                            retry_at: Instant::now() + backoff,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    fn record(&mut self, folders: &mut Folders, effect: Effect) {
        let stored = match effect {
            Effect::Base(base) => self.store.put_base(&base),
            Effect::Forget(node, at) => self.store.forget_base(node, at.as_deref()),
            Effect::Remote(node) => {
                self.mirror.nodes.insert(node.id, node.clone());
                self.store.update_remote(false, &[node], &[])
            }
            Effect::RemoteGone(id) => {
                self.mirror.nodes.remove(&id);
                self.store.update_remote(false, &[], &[id])
            }
            Effect::Rebase { from, to } => {
                *folders = std::mem::take(folders)
                    .into_iter()
                    .map(|(path, id)| (model::rebased(&path, &from, &to).unwrap_or(path), id))
                    .collect();
                self.store.rebase(&from, &to)
            }
            Effect::Folder(path, id) => {
                folders.insert(path, Some(id));
                Ok(())
            }
        };
        if let Err(err) = stored {
            eprintln!("could not record sync state: {err}");
        }
    }

    fn record_remote(&mut self, delta: &Delta) {
        let stored = self
            .store
            .update_remote(delta.replace, &delta.upserted, &delta.removed)
            .and_then(|()| self.store.set_cursor(self.mirror.cursor));
        if let Err(err) = stored {
            eprintln!("could not record the remote tree: {err}");
        }
    }

    /// When the earliest failed path may be tried again.
    fn next_retry(&self) -> Option<Duration> {
        self.problems
            .values()
            .map(|problem| problem.retry_at.saturating_duration_since(Instant::now()))
            .min()
    }

    async fn report(
        &self,
        output: &mut Sender<Update>,
        layout: &Layout,
        pending: &[Action],
        error: Option<String>,
    ) {
        let bases = self.store.bases().unwrap_or_default();
        let pending_paths: Vec<PathBuf> = pending
            .iter()
            .filter_map(|action| action.path().map(Path::to_path_buf))
            .collect();
        let problem_paths: Vec<PathBuf> = self.problems.keys().cloned().collect();
        let badges = status::badges(
            layout,
            &self.mirror.nodes,
            &status::based(&bases),
            &pending_paths,
            &problem_paths,
        );
        let mut problems: Vec<(PathBuf, String)> = self
            .problems
            .iter()
            .map(|(path, problem)| (path.clone(), problem.reason.clone()))
            .collect();
        problems.sort();
        let names = self
            .mappings
            .iter()
            .filter_map(|mapping| mapping.remote)
            .filter_map(|id| Some((id, self.mirror.nodes.get(&id)?.name.clone())))
            .collect();
        let status = Status {
            badges,
            mappings: self.mappings.clone(),
            names,
            pending: pending_paths.len(),
            problems,
            error,
        };
        let _ = output.send(Update::Status(Arc::new(status))).await;
    }
}
