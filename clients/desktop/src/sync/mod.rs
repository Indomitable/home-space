//! Two-way sync of mapped remote folders with local folders.
//!
//! Every pass compares three states per path: the remote tree (kept current from the
//! server's change journal), a scan of the local folders and the bases recorded when both
//! last agreed (see [`plan`]). Passes run on start, after local file changes, after server
//! events and every few minutes.
//!
//! A path that fails is tried again later, waiting longer after each failure. When the
//! server cannot be reached at all, such as behind a gateway answering 502, the failures
//! are not the paths' fault: after a few in a row the pass stops starting transfers and the
//! sync reports one [`Outage`], trying again with a growing delay.
//!
//! A chunked upload's session is saved in the [`store`], so that after a restart it
//! continues from what the server already has, as long as the file is unchanged. Sessions of
//! files changed, deleted or no longer to be uploaded are cancelled on the server.

mod apply;
#[cfg(test)]
mod e2e;
mod layout;
mod model;
mod plan;
mod queue;
mod remote;
mod scan;
mod status;
mod store;

use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use time::OffsetDateTime;

use iced::{
    Subscription,
    futures::{SinkExt, Stream, StreamExt, channel::mpsc::Sender},
};
use notify::{RecursiveMode, Watcher};
use tokio::{sync::mpsc, time::Instant};

use api_client::{ApiError, Client};
use protocol::{Event, NodeId, UserId};

use self::{
    apply::{Effect, Failure, Folders},
    layout::Layout,
    plan::{Action, Phase},
    queue::{Progress, QueueItem},
    remote::{Delta, Mirror},
    scan::Scan,
    status::Badges,
    store::Store,
};
pub use self::{
    model::Mapping,
    queue::{Fault, ItemState, Operation, QueueItem as Item, Summary},
    status::Badge,
};
use crate::session;

const BUFFER: usize = 64;
const TRANSFERS: usize = 4;
/// Local changes come in bursts; a pass waits for the burst to settle.
const SETTLE: Duration = Duration::from_secs(1);
/// Catches what neither the file watcher nor the server events reported.
const RESCAN: Duration = Duration::from_secs(300);
const RETRY: Duration = Duration::from_secs(30);
const MAX_BACKOFF: Duration = Duration::from_secs(10 * 60);
/// Unreachable-server failures in a row that make a pass stop as an outage.
const OUTAGE_FAILURES: u32 = 3;
const MAX_OUTAGE_BACKOFF: Duration = Duration::from_secs(5 * 60);
/// Code of a file the scan could not read.
const UNREADABLE_CODE: &str = "unreadable";
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

/// What the engine is busy with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    Idle,
    /// Reading what changed on the server.
    Checking,
    /// Looking through the local folders, hashing new and changed files.
    Scanning,
    /// Moving, transferring and deleting.
    Syncing,
}

/// A snapshot of the sync for the interface.
#[derive(Debug, Clone, Default)]
pub struct Status {
    /// Badge of every synced node; Home is under `None`.
    pub badges: Badges,
    pub mappings: Vec<Mapping>,
    /// Names of the mapped remote folders, as far as they are known.
    pub names: HashMap<NodeId, String>,
    /// Where each synced folder is on disk, Home under `None`.
    pub locations: HashMap<Option<NodeId>, PathBuf>,
    /// Paths with work ahead.
    pub pending: usize,
    /// Paths that failed, with why.
    pub problems: Vec<(PathBuf, Fault)>,
    /// Why the last pass could not run.
    pub error: Option<String>,
    /// Set while the server cannot be reached.
    pub outage: Option<Outage>,
    pub activity: Activity,
    /// Everything with work ahead, then what failed.
    pub queue: Vec<Item>,
}

impl Status {
    pub fn badge(&self, node: Option<NodeId>) -> Option<Badge> {
        self.badges.get(&node).copied()
    }

    /// Where the folder `node` syncs to, if it does.
    pub fn location(&self, node: Option<NodeId>) -> Option<&Path> {
        self.locations.get(&node).map(PathBuf::as_path)
    }

    /// The new files and folders waiting to be uploaded into `folder` (`None` for Home),
    /// which the server's listing of it cannot show yet.
    pub fn new_uploads_in(&self, folder: Option<NodeId>) -> impl Iterator<Item = &Item> {
        self.queue.iter().filter(move |item| item.is_new_in(folder))
    }

    /// The queue in numbers.
    pub fn summary(&self) -> Summary {
        queue::summarize(&self.queue)
    }

    /// The mapping set on `node` itself, if any.
    pub fn mapping(&self, node: Option<NodeId>) -> Option<&Mapping> {
        self.mappings.iter().find(|mapping| mapping.remote == node)
    }

    /// The state of the sync as a whole: the worst of all synced nodes, `None` while
    /// nothing is synced.
    pub fn overall(&self) -> Option<Badge> {
        if self.error.is_some() || self.outage.is_some() || !self.problems.is_empty() {
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

/// The server cannot be reached; the sync waits and tries again on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outage {
    pub reason: String,
    pub since: OffsetDateTime,
    /// When the sync tries again.
    pub retry_at: OffsetDateTime,
    attempts: u32,
}

#[derive(Debug, Clone)]
pub enum Command {
    /// Syncs a remote folder to a local folder, or sets it to not sync.
    Map(Mapping),
    /// Removes the mapping of a remote folder; its local files stay.
    Unmap(Option<NodeId>),
    /// The server reported a change.
    RemoteChanged(Event),
    /// Tries the failed paths again now: these ones, or all of them with `None`.
    Retry(Option<Vec<PathBuf>>),
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
        let mut interrupted: Option<Command> = None;
        loop {
            let wake = due.unwrap_or_else(|| Instant::now() + RESCAN);
            tokio::select! {
                command = async {
                    match interrupted.take() {
                        Some(command) => Some(command),
                        None => received.recv().await,
                    }
                } => {
                    let Some(command) = command else { break };
                    if engine.is_echo(&command) {
                        continue;
                    }
                    if engine.handle(command, &mut output).await {
                        watcher = engine.watch(&changed);
                        engine.activity = Activity::Checking;
                        engine.report_now(&mut output).await;
                    }
                    due = Some(due.map_or(Instant::now(), |at| at.min(Instant::now())));
                }
                Some(()) = local_changes.recv() => {
                    due.get_or_insert_with(|| Instant::now() + SETTLE);
                }
                () = tokio::time::sleep_until(wake) => {
                    match engine.pass(&mut output, &mut received).await {
                        Ok(next) => due = next.map(|delay| Instant::now() + delay),
                        Err(SignedOut) => break,
                    }
                    interrupted = engine.interrupted.take();
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
    fault: Fault,
    attempts: u32,
    retry_at: Instant,
    /// What was being done, when it is known.
    item: Option<QueueItem>,
}

struct Engine {
    client: Client,
    store: Arc<Store>,
    mirror: Mirror,
    mappings: Vec<Mapping>,
    /// Set when the mapped folders must be read from scratch.
    reload: bool,
    problems: HashMap<PathBuf, Problem>,
    /// Set while the server cannot be reached.
    outage: Option<Outage>,
    /// Local paths the last scan could not read, with why.
    unreadable: Vec<(PathBuf, Fault)>,
    activity: Activity,
    /// The work ahead as of the last plan, less what has finished since.
    queue: Vec<QueueItem>,
    /// Bytes moved so far by the transfers running.
    progress: Progress,
    /// Set when the engine goes away, so a scan on a blocking thread stops with it; the
    /// app cannot quit while one runs.
    stop: Arc<AtomicBool>,
    /// A command that arrived while a scan ran and cut it short, to be handled next.
    interrupted: Option<Command>,
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
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
            store: Arc::new(store),
            problems: HashMap::new(),
            outage: None,
            unreadable: Vec::new(),
            activity: Activity::Idle,
            queue: Vec::new(),
            progress: Progress::default(),
            stop: Arc::new(AtomicBool::new(false)),
            interrupted: None,
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
            Command::RemoteChanged(_) => false,
            Command::Retry(paths) => {
                let now = Instant::now();
                self.problems
                    .iter_mut()
                    .filter(|(path, _)| paths.as_ref().is_none_or(|paths| paths.contains(path)))
                    .for_each(|(_, problem)| {
                        problem.retry_at = now;
                        problem.attempts = 0;
                    });
                self.outage = None;
                false
            }
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
    async fn pass(
        &mut self,
        output: &mut Sender<Update>,
        commands: &mut mpsc::UnboundedReceiver<Command>,
    ) -> Result<Option<Duration>, SignedOut> {
        if self.mappings.iter().all(|m| m.local.is_none()) {
            self.activity = Activity::Idle;
            self.unreadable.clear();
            self.queue.clear();
            self.report(output, &Layout::default(), &[], None).await;
            return Ok(None);
        }
        self.activity = Activity::Checking;
        self.report_now(output).await;
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
                self.activity = Activity::Idle;
                let layout = Layout::new(&self.mirror.nodes, &self.mappings);
                let (delay, error) = if err.is_unreachable() {
                    (self.unreachable(err.to_string()), None)
                } else {
                    (
                        RETRY,
                        Some(format!("cannot read changes from the server: {err}")),
                    )
                };
                self.report(output, &layout, &[], error).await;
                return Ok(Some(delay));
            }
        }
        let layout = Layout::new(&self.mirror.nodes, &self.mappings);
        let bases = self.store.bases().unwrap_or_default();
        self.activity = Activity::Scanning;
        self.report(output, &layout, &[], None).await;
        let Some(scan) = self.scan(&layout, &bases, commands).await else {
            // A command cut the scan short; it is handled first, then the pass starts over.
            return Ok(Some(Duration::ZERO));
        };
        self.unreadable = self.faults_of(&scan.unreadable);
        let now = Instant::now();
        let planned = plan::plan(&layout, &scan, &bases);
        self.discard_uploads(&planned).await;
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
        self.queue = queue::items(&actions, &layout.folders);
        if actions.is_empty() {
            // The server answered and nothing is left to send it.
            self.outage = None;
            self.activity = Activity::Idle;
            self.report(output, &layout, &[], None).await;
            return Ok(self.next_retry());
        }
        self.activity = Activity::Syncing;
        self.report(output, &layout, &actions, None).await;
        let outage = self.execute(output, &layout, actions).await?;
        self.activity = Activity::Idle;
        self.queue.clear();
        self.report(output, &layout, &[], None).await;
        Ok(Some(outage.unwrap_or(SETTLE)))
    }

    /// Scans the local folders, or gives up with `None` when a command arrives meanwhile
    /// (kept in `interrupted`), since the scan of a big folder takes long.
    async fn scan(
        &mut self,
        layout: &Layout,
        bases: &[model::Base],
        commands: &mut mpsc::UnboundedReceiver<Command>,
    ) -> Option<Scan> {
        let roots = layout.roots.clone();
        let skipped = layout.skipped.clone();
        let known = bases
            .iter()
            .map(|base| (base.path.clone(), base.clone()))
            .collect();
        let remote = layout.entries.keys().cloned().collect();
        let stop = Arc::clone(&self.stop);
        let mut task = tokio::task::spawn_blocking(move || {
            scan::scan(&roots, &skipped, &known, &remote, &stop)
        });
        let blind = || Scan {
            blind: layout.roots.clone(),
            ..Scan::default()
        };
        let command = loop {
            tokio::select! {
                scan = &mut task => return Some(scan.unwrap_or_else(|_| blind())),
                command = commands.recv() => {
                    if command.as_ref().is_some_and(|command| self.is_echo(command)) {
                        continue;
                    }
                    break command;
                }
            }
        };
        // The scan gives up at its next folder or file chunk; wait for it to let go.
        self.stop.store(true, Ordering::Relaxed);
        let _ = task.await;
        self.stop.store(false, Ordering::Relaxed);
        // Without a command the app is going away, which ends the loop.
        self.interrupted = command;
        None
    }

    /// Whether `command` reports a server change the mirror already holds, mostly the
    /// engine's own changes coming back, which need no pass.
    fn is_echo(&self, command: &Command) -> bool {
        matches!(command, Command::RemoteChanged(event) if self.mirror.already_has(event))
    }

    /// Cancels the saved chunked uploads of paths with no upload planned any more, such as
    /// files deleted, or synced some other way, since the upload began.
    async fn discard_uploads(&self, planned: &[Action]) {
        let uploading: HashSet<&Path> = planned
            .iter()
            .filter_map(|action| match action {
                Action::Upload { path, .. } => Some(path.as_path()),
                _ => None,
            })
            .collect();
        let saved = self.store.saved_uploads().unwrap_or_default();
        for upload in saved
            .iter()
            .filter(|upload| !uploading.contains(upload.path.as_path()))
        {
            // One the server cannot be asked about now stays saved for a later pass.
            let _ = apply::discard(&self.client, &self.store, upload).await;
        }
    }

    /// The faults of the paths the scan could not read, keeping when each was first seen.
    fn faults_of(&self, unreadable: &[(PathBuf, String)]) -> Vec<(PathBuf, Fault)> {
        let seen: HashMap<&Path, &Fault> = self
            .unreadable
            .iter()
            .map(|(path, fault)| (path.as_path(), fault))
            .collect();
        unreadable
            .iter()
            .map(|(path, reason)| {
                let fault = seen
                    .get(path.as_path())
                    .filter(|fault| fault.reason == *reason)
                    .map_or_else(
                        || Fault::now(UNREADABLE_CODE, reason.clone()),
                        |fault| (*fault).clone(),
                    );
                (path.clone(), fault)
            })
            .collect()
    }

    /// Runs the actions phase by phase; transfers run several at a time. Returns when to
    /// try again if the server turned out to be unreachable.
    async fn execute(
        &mut self,
        output: &mut Sender<Update>,
        layout: &Layout,
        actions: Vec<Action>,
    ) -> Result<Option<Duration>, SignedOut> {
        let mut folders: Folders = layout.folders.clone();
        let mut breaker = Breaker::default();
        // Everything not finished yet, which the badges show as pending.
        let mut unfinished = actions.clone();
        let mut reported = Instant::now();
        let finish = |unfinished: &mut Vec<Action>, action: &Action| {
            if let Some(at) = unfinished.iter().position(|other| other == action) {
                unfinished.swap_remove(at);
            }
        };
        for phase in [Phase::Move, Phase::Create, Phase::Transfer, Phase::Delete] {
            if breaker.is_tripped() {
                break;
            }
            let now: Vec<Action> = actions
                .iter()
                .filter(|action| action.phase() == phase)
                .cloned()
                .collect();
            if phase == Phase::Transfer {
                let snapshot = folders.clone();
                let client = self.client.clone();
                let store = Arc::clone(&self.store);
                let progress = Arc::clone(&self.progress);
                let halt = Arc::clone(&breaker.halt);
                let mut outcomes = apply::execute_all(
                    &client, &store, &snapshot, &progress, &halt, now, TRANSFERS,
                );
                let mut tick = tokio::time::interval(PROGRESS);
                loop {
                    // A large file reports on the tick, between the ends of transfers.
                    let due = tokio::select! {
                        next = outcomes.next() => {
                            let Some((action, outcome)) = next else { break };
                            finish(&mut unfinished, &action);
                            self.settle(&mut folders, &mut breaker, action, outcome)?;
                            reported.elapsed() >= PROGRESS
                        }
                        _ = tick.tick() => true,
                    };
                    if due {
                        reported = Instant::now();
                        self.queue = queue::items(&unfinished, &folders);
                        self.report(output, layout, &unfinished, None).await;
                    }
                }
            } else {
                // One at a time, so each action sees the folders the previous ones created.
                for action in now {
                    if breaker.is_tripped() {
                        break;
                    }
                    let outcome = apply::execute(
                        &self.client,
                        &self.store,
                        &folders,
                        &self.progress,
                        action.clone(),
                    )
                    .await;
                    finish(&mut unfinished, &action);
                    self.settle(&mut folders, &mut breaker, action, outcome)?;
                }
            }
            if reported.elapsed() >= PROGRESS || phase == Phase::Delete {
                reported = Instant::now();
                self.queue = queue::items(&unfinished, &folders);
                self.report(output, layout, &unfinished, None).await;
            }
        }
        Ok(self.conclude(&folders, breaker))
    }

    /// Records how the pass ended: an outage, returning when to try again, or the paths that
    /// failed on their own.
    fn conclude(&mut self, folders: &Folders, breaker: Breaker) -> Option<Duration> {
        match breaker.tripped {
            Some(reason) => Some(self.unreachable(reason)),
            None => {
                // The server answered meanwhile, so these paths failed on their own.
                for (action, fault) in breaker.held {
                    self.fail(folders, &action, fault);
                }
                self.outage = None;
                None
            }
        }
    }

    fn settle(
        &mut self,
        folders: &mut Folders,
        breaker: &mut Breaker,
        action: Action,
        outcome: Result<Vec<Effect>, Failure>,
    ) -> Result<(), SignedOut> {
        match outcome {
            Ok(effects) => {
                breaker.in_row = 0;
                if let Some(path) = action.path() {
                    self.problems.remove(path);
                }
                for effect in effects {
                    self.record(folders, effect);
                }
            }
            Err(Failure::SignedOut) => return Err(SignedOut),
            Err(Failure::Stale | Failure::Halted) => {}
            Err(Failure::Unreachable(fault)) => breaker.failed(action, fault),
            Err(Failure::Error(fault)) => self.fail(folders, &action, fault),
        }
        Ok(())
    }

    /// Records that `action` failed with `fault`, to be tried again after a while.
    fn fail(&mut self, folders: &Folders, action: &Action, fault: Fault) {
        let Some(path) = action.path() else {
            return;
        };
        let attempts = self.problems.get(path).map_or(1, |p| p.attempts + 1);
        self.problems.insert(
            path.to_path_buf(),
            Problem {
                fault,
                attempts,
                retry_at: Instant::now() + backoff(attempts, MAX_BACKOFF),
                item: QueueItem::of(action, folders),
            },
        );
    }

    /// Records that the server could not be reached; returns when to try again.
    fn unreachable(&mut self, reason: String) -> Duration {
        let attempts = self.outage.as_ref().map_or(1, |outage| outage.attempts + 1);
        let delay = backoff(attempts, MAX_OUTAGE_BACKOFF);
        let now = OffsetDateTime::now_utc();
        self.outage = Some(Outage {
            reason,
            since: self.outage.as_ref().map_or(now, |outage| outage.since),
            retry_at: now + delay,
            attempts,
        });
        delay
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

    /// Reports with the layout of what is known now, between passes.
    async fn report_now(&self, output: &mut Sender<Update>) {
        let layout = Layout::new(&self.mirror.nodes, &self.mappings);
        self.report(output, &layout, &[], None).await;
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
        let problem_paths: Vec<PathBuf> = self
            .problems
            .keys()
            .chain(self.unreadable.iter().map(|(path, _)| path))
            .cloned()
            .collect();
        let badges = status::badges(
            layout,
            &self.mirror.nodes,
            &status::based(&bases),
            &pending_paths,
            &problem_paths,
        );
        let mut problems: Vec<(PathBuf, Fault)> = self
            .problems
            .iter()
            .map(|(path, problem)| (path.clone(), problem.fault.clone()))
            .chain(self.unreadable.iter().cloned())
            .collect();
        problems.sort_by(|a, b| a.0.cmp(&b.0));
        let names = self
            .mappings
            .iter()
            .filter_map(|mapping| mapping.remote)
            .filter_map(|id| Some((id, self.mirror.nodes.get(&id)?.name.clone())))
            .collect();
        let locations = layout
            .folders
            .iter()
            .map(|(path, id)| (*id, path.clone()))
            .collect();
        let moving = queue::snapshot(&self.progress);
        let mut queue: Vec<QueueItem> = self
            .queue
            .iter()
            .cloned()
            .map(|item| item.with_progress(&moving))
            .collect();
        // What is moving now first; the sort keeps the order of the rest.
        queue.sort_by_key(|item| item.state != ItemState::Active);
        let mut failed: Vec<QueueItem> = self
            .problems
            .iter()
            .map(|(path, problem)| match &problem.item {
                Some(item) => item.clone().failed(problem.fault.clone()),
                None => QueueItem::blocked(path, problem.fault.clone()),
            })
            .chain(
                self.unreadable
                    .iter()
                    .map(|(path, fault)| QueueItem::blocked(path, fault.clone())),
            )
            .collect();
        failed.sort_by(|a, b| a.path.cmp(&b.path));
        queue.extend(failed);
        let status = Status {
            badges,
            mappings: self.mappings.clone(),
            names,
            locations,
            pending: pending_paths.len(),
            problems,
            error,
            outage: self.outage.clone(),
            activity: self.activity,
            queue,
        };
        let _ = output.send(Update::Status(Arc::new(status))).await;
    }
}

/// Tells a server that cannot be reached from paths that fail on their own, during one pass.
#[derive(Default)]
struct Breaker {
    /// Unreachable-server failures since the last action that worked.
    in_row: u32,
    /// Actions that failed as unreachable, held until it is clear whether the server was down.
    held: Vec<(Action, Fault)>,
    /// Why the pass stopped as an outage.
    tripped: Option<String>,
    /// Set once tripped, so no further transfer starts.
    halt: Arc<AtomicBool>,
}

impl Breaker {
    fn failed(&mut self, action: Action, fault: Fault) {
        self.in_row += 1;
        if self.in_row >= OUTAGE_FAILURES && self.tripped.is_none() {
            self.tripped = Some(fault.reason.clone());
            self.halt.store(true, Ordering::Relaxed);
        }
        self.held.push((action, fault));
    }

    fn is_tripped(&self) -> bool {
        self.tripped.is_some()
    }
}

/// The wait after `attempts` failures: [`RETRY`], doubling each time up to `max`.
fn backoff(attempts: u32, max: Duration) -> Duration {
    RETRY
        .saturating_mul(2u32.saturating_pow(attempts.saturating_sub(1)))
        .min(max)
}

#[cfg(test)]
mod tests {
    use super::{model::LocalEntry, *};
    use protocol::NodeKind;

    const UNREACHABLE: &str = "server unreachable (502 Bad Gateway)";

    fn engine() -> Engine {
        Engine::load(
            Client::new("http://127.0.0.1:9"),
            Store::in_memory().expect("store"),
        )
        .expect("engine")
    }

    fn upload(name: &str) -> Action {
        Action::Upload {
            node: None,
            path: PathBuf::from("/sync").join(name),
            local: LocalEntry {
                kind: NodeKind::File,
                size: 1,
                mtime: 0,
                sha256: None,
                unhashed: false,
            },
        }
    }

    fn folders() -> Folders {
        Folders::from([(PathBuf::from("/sync"), None)])
    }

    fn unreachable() -> Result<Vec<Effect>, Failure> {
        Err(Failure::Unreachable(Fault::now("502", UNREACHABLE)))
    }

    fn settle_all(
        engine: &mut Engine,
        breaker: &mut Breaker,
        outcomes: Vec<(&str, Result<Vec<Effect>, Failure>)>,
    ) {
        let mut folders = folders();
        for (name, outcome) in outcomes {
            assert!(
                engine
                    .settle(&mut folders, breaker, upload(name), outcome)
                    .is_ok()
            );
        }
    }

    #[test]
    fn unreachable_failures_in_a_row_are_one_outage_and_no_file_problems() {
        let mut engine = engine();
        let mut breaker = Breaker::default();
        settle_all(
            &mut engine,
            &mut breaker,
            vec![
                ("a", unreachable()),
                ("b", unreachable()),
                ("c", unreachable()),
            ],
        );
        assert!(breaker.halt.load(Ordering::Relaxed));
        let delay = engine.conclude(&folders(), breaker);
        assert_eq!(delay, Some(RETRY));
        assert!(engine.problems.is_empty());
        let outage = engine.outage.as_ref().expect("outage");
        assert_eq!(outage.reason, UNREACHABLE);
    }

    #[test]
    fn an_outage_waits_longer_each_time() {
        let mut engine = engine();
        assert_eq!(engine.unreachable(UNREACHABLE.to_owned()), RETRY);
        assert_eq!(engine.unreachable(UNREACHABLE.to_owned()), RETRY * 2);
        let since = engine.outage.as_ref().map(|outage| outage.since);
        engine.unreachable(UNREACHABLE.to_owned());
        assert_eq!(engine.outage.as_ref().map(|outage| outage.since), since);
    }

    #[test]
    fn unreachable_failures_between_successes_are_file_problems() {
        let mut engine = engine();
        engine.unreachable(UNREACHABLE.to_owned());
        let mut breaker = Breaker::default();
        settle_all(
            &mut engine,
            &mut breaker,
            vec![
                ("a", unreachable()),
                ("b", unreachable()),
                ("c", Ok(Vec::new())),
                ("d", unreachable()),
            ],
        );
        assert!(!breaker.halt.load(Ordering::Relaxed));
        assert_eq!(engine.conclude(&folders(), breaker), None);
        assert_eq!(engine.problems.len(), 3);
        assert!(engine.outage.is_none());
    }

    #[test]
    fn a_file_error_is_a_problem_with_its_code() {
        let mut engine = engine();
        let mut breaker = Breaker::default();
        let too_large = Failure::from(ApiError::Status {
            status: 413,
            reason: "Payload Too Large".to_owned(),
        });
        settle_all(&mut engine, &mut breaker, vec![("big", Err(too_large))]);
        let problem = engine
            .problems
            .get(Path::new("/sync/big"))
            .expect("problem");
        assert_eq!(problem.fault.code, "413");
        assert_eq!(problem.attempts, 1);
    }

    #[tokio::test]
    async fn retry_makes_the_chosen_problems_due_now() {
        let mut engine = engine();
        let mut breaker = Breaker::default();
        let failed = || Err(Failure::Error(Fault::now("local", "busy")));
        settle_all(
            &mut engine,
            &mut breaker,
            vec![("a", failed()), ("b", failed())],
        );
        engine.unreachable(UNREACHABLE.to_owned());
        let (mut output, _updates) = iced::futures::channel::mpsc::channel(4);
        let chosen = PathBuf::from("/sync/a");
        engine
            .handle(Command::Retry(Some(vec![chosen.clone()])), &mut output)
            .await;
        let now = Instant::now();
        let due = |path: &Path| engine.problems.get(path).expect("problem").retry_at <= now;
        assert!(due(&chosen));
        assert!(!due(Path::new("/sync/b")));
        assert!(engine.outage.is_none());
    }
}
