use std::{
    collections::{HashMap, VecDeque},
    convert::identity,
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use iced::{
    Alignment, Element, Length, Task,
    futures::{SinkExt, Stream, StreamExt, channel::mpsc, stream},
    widget::{button, column, container, progress_bar, row, scrollable, text},
};
use tokio::{fs, io::AsyncWriteExt};
use tokio_util::io::ReaderStream;

use api_client::{ApiError, Client, listing::sort_nodes};
use protocol::{ErrorCode, Node, NodeId, NodeKind};

use crate::{
    icons::{Icon, Tint},
    theme,
};

const CHANNEL_SIZE: usize = 64;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
const MAX_DEPTH: usize = 64;
const PART_SUFFIX: &str = ".part";
const REPLACEMENT: char = '_';
const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
const TOO_DEEP: &str = "folder tree is too deep";
const ALREADY_EXISTS: &str = "already exists";
const MISSING_FOLDER: &str = "parent folder could not be created";
const PANEL_WIDTH: f32 = 420.0;
const LIST_HEIGHT: f32 = 220.0;
const PANEL_PADDING: f32 = 12.0;
const SPACING: f32 = 8.0;
const ICON_SIZE: f32 = 16.0;
const NAME_SIZE: f32 = 13.0;
const BAR_HEIGHT: f32 = 6.0;

type TransferId = u64;

/// Which way bytes flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Upload,
    Download,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TransferState {
    Queued,
    Active,
    Done,
    Failed(String),
}

#[derive(Debug, Clone)]
struct Transfer {
    id: TransferId,
    name: String,
    direction: Direction,
    done: u64,
    total: u64,
    state: TransferState,
}

impl Transfer {
    fn is_pending(&self) -> bool {
        matches!(self.state, TransferState::Queued | TransferState::Active)
    }

    fn fraction(&self) -> f32 {
        match (&self.state, self.total) {
            (TransferState::Done, _) => 1.0,
            (_, 0) => 0.0,
            (_, total) => (self.done as f64 / total as f64).min(1.0) as f32,
        }
    }
}

/// A local file to upload, with the folder path it goes into relative to the target.
#[derive(Debug, Clone)]
pub struct UploadFile {
    path: PathBuf,
    folder: Vec<String>,
    size: u64,
}

/// Folders to create (parents first) and files to upload.
#[derive(Debug, Clone, Default)]
pub struct UploadPlan {
    folders: Vec<Vec<String>>,
    files: Vec<UploadFile>,
}

impl UploadPlan {
    /// Whether the plan has nothing to do.
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty() && self.files.is_empty()
    }
}

/// What to do when a download target already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Existing {
    Replace,
    Fail,
}

/// A remote file and where to save it.
#[derive(Debug, Clone)]
pub struct DownloadFile {
    id: NodeId,
    name: String,
    target: PathBuf,
    size: u64,
    existing: Existing,
}

impl DownloadFile {
    /// Downloads `node` to `target`, replacing it; the user already confirmed the overwrite.
    pub fn replacing(node: &Node, target: PathBuf) -> Self {
        Self {
            id: node.id,
            name: node.name.clone(),
            target,
            size: byte_size(node.size),
            existing: Existing::Replace,
        }
    }
}

/// A unit of work run as one sequential task.
#[derive(Debug, Clone)]
pub enum Batch {
    Upload {
        parent: Option<NodeId>,
        plan: UploadPlan,
    },
    Download(Vec<DownloadFile>),
}

impl Batch {
    fn direction(&self) -> Direction {
        match self {
            Self::Upload { .. } => Direction::Upload,
            Self::Download(_) => Direction::Download,
        }
    }

    fn entries(&self) -> Vec<(String, u64)> {
        match self {
            Self::Upload { plan, .. } => plan
                .files
                .iter()
                .map(|file| (display_name(&file.path), file.size))
                .collect(),
            Self::Download(files) => files
                .iter()
                .map(|file| (file.name.clone(), file.size))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Started(TransferId),
    Progress(TransferId, u64),
    Finished(TransferId, Result<(), String>),
    BatchDone(Direction),
    ToggleExpanded,
    ClearCompleted,
    Close,
}

/// The transfer queue: one batch runs at a time, the rest wait.
#[derive(Debug, Default)]
pub struct Transfers {
    items: Vec<Transfer>,
    pending: VecDeque<(Batch, Vec<TransferId>)>,
    running: bool,
    next_id: TransferId,
    expanded: bool,
}

impl Transfers {
    /// Queues `batch` and starts it when nothing else runs.
    pub fn enqueue(&mut self, client: &Client, batch: Batch) -> Task<Message> {
        let direction = batch.direction();
        let ids: Vec<TransferId> = batch
            .entries()
            .into_iter()
            .map(|(name, total)| {
                let id = self.next_id;
                self.next_id += 1;
                self.items.push(Transfer {
                    id,
                    name,
                    direction,
                    done: 0,
                    total,
                    state: TransferState::Queued,
                });
                id
            })
            .collect();
        self.pending.push_back((batch, ids));
        self.start_next(client)
    }

    /// Applies a progress or panel message.
    pub fn update(&mut self, client: &Client, message: Message) -> Task<Message> {
        match message {
            Message::Started(id) => {
                self.change(id, |item| item.state = TransferState::Active);
                Task::none()
            }
            Message::Progress(id, done) => {
                self.change(id, |item| item.done = done);
                Task::none()
            }
            Message::Finished(id, result) => {
                self.change(id, |item| match result {
                    Ok(()) => {
                        item.done = item.total;
                        item.state = TransferState::Done;
                    }
                    Err(message) => item.state = TransferState::Failed(message),
                });
                Task::none()
            }
            Message::BatchDone(_) => {
                self.running = false;
                self.start_next(client)
            }
            Message::ToggleExpanded => {
                self.expanded = !self.expanded;
                Task::none()
            }
            Message::ClearCompleted => {
                self.items.retain(|item| item.state != TransferState::Done);
                Task::none()
            }
            Message::Close => {
                if !self.is_busy() {
                    self.items.clear();
                }
                Task::none()
            }
        }
    }

    /// Whether the panel has anything to show.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn is_busy(&self) -> bool {
        self.running || self.items.iter().any(Transfer::is_pending)
    }

    fn change(&mut self, id: TransferId, change: impl FnOnce(&mut Transfer)) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            change(item);
        }
    }

    fn start_next(&mut self, client: &Client) -> Task<Message> {
        if self.running {
            Task::none()
        } else if let Some((batch, ids)) = self.pending.pop_front() {
            self.running = true;
            Task::run(run_batch(client.clone(), batch, ids), identity)
        } else {
            Task::none()
        }
    }

    /// The floating progress panel.
    pub fn view(&self) -> Element<'_, Message> {
        let busy = self.is_busy();
        let has_done = self
            .items
            .iter()
            .any(|item| item.state == TransferState::Done);
        let header = row![
            text(summarize(&self.items)).width(Length::Fill),
            button(text("Clear completed").size(NAME_SIZE))
                .style(theme::tool)
                .on_press_maybe(has_done.then_some(Message::ClearCompleted)),
            button(text(if self.expanded { "▾" } else { "▴" }))
                .style(theme::tool)
                .on_press(Message::ToggleExpanded),
            button(Icon::Close.view(ICON_SIZE, Tint::Text))
                .style(theme::tool)
                .on_press_maybe((!busy).then_some(Message::Close)),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center);
        let overall = progress_bar(0.0..=1.0, overall_progress(&self.items)).girth(BAR_HEIGHT);
        let mut panel = column![header, overall].spacing(SPACING);
        if self.expanded {
            let list = column(self.items.iter().map(item_view)).spacing(SPACING);
            panel = panel.push(scrollable(list).height(LIST_HEIGHT));
        }
        container(panel)
            .padding(PANEL_PADDING)
            .width(PANEL_WIDTH)
            .style(theme::panel)
            .into()
    }
}

fn item_view(item: &Transfer) -> Element<'_, Message> {
    let icon = match item.direction {
        Direction::Upload => Icon::Upload,
        Direction::Download => Icon::Download,
    };
    let status = match &item.state {
        TransferState::Queued => "Waiting".to_owned(),
        TransferState::Active => "In progress".to_owned(),
        TransferState::Done => "Done".to_owned(),
        TransferState::Failed(message) => format!("Failed: {message}"),
    };
    let status: Element<'_, Message> = match item.state {
        TransferState::Failed(_) => container(text(status).size(NAME_SIZE))
            .style(theme::error)
            .into(),
        _ => text(status).size(NAME_SIZE).into(),
    };
    column![
        row![
            icon.view(ICON_SIZE, Tint::Muted),
            text(item.name.as_str()).size(NAME_SIZE).width(Length::Fill),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center),
        progress_bar(0.0..=1.0, item.fraction()).girth(BAR_HEIGHT),
        status,
    ]
    .spacing(SPACING / 2.0)
    .into()
}

fn summarize(items: &[Transfer]) -> String {
    let total = items.len();
    let pending: Vec<&Transfer> = items.iter().filter(|item| item.is_pending()).collect();
    let failed = items
        .iter()
        .filter(|item| matches!(item.state, TransferState::Failed(_)))
        .count();
    match pending.first() {
        Some(next) => {
            let verb = match next.direction {
                Direction::Upload => "Uploading",
                Direction::Download => "Downloading",
            };
            format!("{verb} {} of {total}", total - pending.len() + 1)
        }
        None if failed > 0 => format!("{failed} failed, {} completed", total - failed),
        None => format!("{total} completed"),
    }
}

fn overall_progress(items: &[Transfer]) -> f32 {
    if items.is_empty() {
        0.0
    } else {
        items.iter().map(Transfer::fraction).sum::<f32>() / items.len() as f32
    }
}

/// Replaces characters that are not allowed in a local file name.
pub fn sanitize_component(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| {
            if character.is_control() || FORBIDDEN.contains(&character) {
                REPLACEMENT
            } else {
                character
            }
        })
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        REPLACEMENT.to_string()
    } else {
        cleaned
    }
}

fn byte_size(size: i64) -> u64 {
    u64::try_from(size).unwrap_or_default()
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Walks local files and folders into an upload plan, parents before children.
pub async fn plan_upload(paths: Vec<PathBuf>) -> Result<UploadPlan, String> {
    walk_local(paths).await.map_err(|err| err.to_string())
}

async fn walk_local(paths: Vec<PathBuf>) -> io::Result<UploadPlan> {
    let mut plan = UploadPlan::default();
    let mut stack: Vec<(PathBuf, Vec<String>, usize)> = paths
        .into_iter()
        .rev()
        .map(|path| (path, Vec::new(), 0))
        .collect();
    while let Some((path, folder, depth)) = stack.pop() {
        let metadata = fs::metadata(&path).await?;
        if metadata.is_dir() {
            if depth >= MAX_DEPTH {
                return Err(io::Error::other(TOO_DEEP));
            }
            let mut inner = folder;
            inner.push(display_name(&path));
            let mut entries = fs::read_dir(&path).await?;
            let mut children = Vec::new();
            while let Some(entry) = entries.next_entry().await? {
                children.push(entry.path());
            }
            children.sort();
            stack.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, inner.clone(), depth + 1)),
            );
            plan.folders.push(inner);
        } else if metadata.is_file() {
            plan.files.push(UploadFile {
                path,
                folder,
                size: metadata.len(),
            });
        }
    }
    Ok(plan)
}

/// Creates local folders for `nodes` under `root` and lists every file to download.
pub async fn plan_download(
    client: Client,
    nodes: Vec<Node>,
    root: PathBuf,
) -> Result<Vec<DownloadFile>, String> {
    let mut files = Vec::new();
    let mut stack: Vec<(Node, PathBuf, usize)> = nodes
        .into_iter()
        .rev()
        .map(|node| {
            let target = root.join(sanitize_component(&node.name));
            (node, target, 0)
        })
        .collect();
    while let Some((node, target, depth)) = stack.pop() {
        match node.kind {
            NodeKind::File => files.push(DownloadFile {
                id: node.id,
                name: node.name,
                target,
                size: byte_size(node.size),
                existing: Existing::Fail,
            }),
            NodeKind::Folder if depth >= MAX_DEPTH => return Err(TOO_DEEP.to_owned()),
            NodeKind::Folder => {
                fs::create_dir_all(&target)
                    .await
                    .map_err(|err| format!("{}: {err}", target.display()))?;
                let mut children = client
                    .list(Some(node.id))
                    .await
                    .map_err(|err| err.to_string())?;
                sort_nodes(&mut children);
                stack.extend(children.into_iter().rev().map(|child| {
                    let path = target.join(sanitize_component(&child.name));
                    (child, path, depth + 1)
                }));
            }
        }
    }
    Ok(files)
}

fn run_batch(client: Client, batch: Batch, ids: Vec<TransferId>) -> impl Stream<Item = Message> {
    iced::stream::channel(
        CHANNEL_SIZE,
        async move |mut output: mpsc::Sender<Message>| {
            let direction = batch.direction();
            match batch {
                Batch::Upload { parent, plan } => {
                    upload_all(&client, parent, plan, &ids, &mut output).await;
                }
                Batch::Download(files) => download_all(&client, files, &ids, &mut output).await,
            }
            let _ = output.send(Message::BatchDone(direction)).await;
        },
    )
}

async fn upload_all(
    client: &Client,
    parent: Option<NodeId>,
    plan: UploadPlan,
    ids: &[TransferId],
    output: &mut mpsc::Sender<Message>,
) {
    let mut folders: HashMap<Vec<String>, Result<Option<NodeId>, String>> =
        HashMap::from([(Vec::new(), Ok(parent))]);
    for path in plan.folders {
        if let Some((name, base)) = path.split_last() {
            let created = match folders.get(base) {
                Some(Ok(base_id)) => ensure_folder(client, *base_id, name)
                    .await
                    .map(Some)
                    .map_err(|err| err.to_string()),
                Some(Err(err)) => Err(err.clone()),
                None => Err(MISSING_FOLDER.to_owned()),
            };
            folders.insert(path, created);
        }
    }
    for (file, id) in plan.files.iter().zip(ids) {
        let _ = output.send(Message::Started(*id)).await;
        let result = match folders.get(&file.folder) {
            Some(Ok(target)) => upload_file(client, *target, file, *id, output).await,
            Some(Err(err)) => Err(err.clone()),
            None => Err(MISSING_FOLDER.to_owned()),
        };
        let _ = output.send(Message::Finished(*id, result)).await;
    }
}

async fn ensure_folder(
    client: &Client,
    parent: Option<NodeId>,
    name: &str,
) -> Result<NodeId, ApiError> {
    match client.create_folder(parent, name).await {
        Ok(folder) => Ok(folder.id),
        Err(ApiError::Api {
            code: ErrorCode::Conflict,
            message,
        }) => client
            .list(parent)
            .await?
            .into_iter()
            .find(|node| node.kind == NodeKind::Folder && node.name == name)
            .map(|node| node.id)
            .ok_or(ApiError::Api {
                code: ErrorCode::Conflict,
                message,
            }),
        Err(err) => Err(err),
    }
}

async fn upload_file(
    client: &Client,
    parent: Option<NodeId>,
    file: &UploadFile,
    id: TransferId,
    output: &mpsc::Sender<Message>,
) -> Result<(), String> {
    let name = display_name(&file.path);
    client
        .upload(parent, &name, || {
            upload_body(&file.path, id, output.clone())
        })
        .await
        .map(drop)
        .map_err(|err| err.to_string())
}

fn upload_body(path: &Path, id: TransferId, mut output: mpsc::Sender<Message>) -> reqwest::Body {
    match std::fs::File::open(path) {
        Ok(file) => {
            let mut meter = Meter::new(id);
            let chunks = ReaderStream::new(fs::File::from_std(file)).inspect(move |chunk| {
                if let Ok(bytes) = chunk
                    && let Some(message) = meter.advance(bytes.len())
                {
                    let _ = output.try_send(message);
                }
            });
            reqwest::Body::wrap_stream(chunks)
        }
        Err(err) => {
            reqwest::Body::wrap_stream(stream::once(async move { Err::<Vec<u8>, io::Error>(err) }))
        }
    }
}

async fn download_all(
    client: &Client,
    files: Vec<DownloadFile>,
    ids: &[TransferId],
    output: &mut mpsc::Sender<Message>,
) {
    for (file, id) in files.iter().zip(ids) {
        let _ = output.send(Message::Started(*id)).await;
        let result = download_file(client, file, *id, output).await;
        let _ = output.send(Message::Finished(*id, result)).await;
    }
}

async fn download_file(
    client: &Client,
    file: &DownloadFile,
    id: TransferId,
    output: &mut mpsc::Sender<Message>,
) -> Result<(), String> {
    let exists = fs::try_exists(&file.target)
        .await
        .map_err(|err| err.to_string())?;
    if exists && file.existing == Existing::Fail {
        Err(ALREADY_EXISTS.to_owned())
    } else {
        let response = client
            .download(file.id)
            .await
            .map_err(|err| err.to_string())?;
        let part = part_path(&file.target);
        let result = write_part(response, &part, id, output).await;
        let result = match result {
            Ok(()) => fs::rename(&part, &file.target).await,
            Err(err) => Err(err),
        };
        if result.is_err() {
            let _ = fs::remove_file(&part).await;
        }
        result.map_err(|err| err.to_string())
    }
}

async fn write_part(
    response: reqwest::Response,
    part: &Path,
    id: TransferId,
    output: &mut mpsc::Sender<Message>,
) -> io::Result<()> {
    let mut out = fs::File::create(part).await?;
    let mut body = response.bytes_stream();
    let mut meter = Meter::new(id);
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(io::Error::other)?;
        out.write_all(&chunk).await?;
        if let Some(message) = meter.advance(chunk.len()) {
            let _ = output.try_send(message);
        }
    }
    out.flush().await
}

fn part_path(target: &Path) -> PathBuf {
    let mut name = target.as_os_str().to_owned();
    name.push(PART_SUFFIX);
    PathBuf::from(name)
}

struct Meter {
    id: TransferId,
    done: u64,
    reported: Option<Instant>,
}

impl Meter {
    fn new(id: TransferId) -> Self {
        Self {
            id,
            done: 0,
            reported: None,
        }
    }

    fn advance(&mut self, bytes: usize) -> Option<Message> {
        self.done = self
            .done
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
        let now = Instant::now();
        let due = self
            .reported
            .is_none_or(|at| now.duration_since(at) >= PROGRESS_INTERVAL);
        if due {
            self.reported = Some(now);
            Some(Message::Progress(self.id, self.done))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transfer(state: TransferState, done: u64, total: u64) -> Transfer {
        Transfer {
            id: 0,
            name: String::new(),
            direction: Direction::Upload,
            done,
            total,
            state,
        }
    }

    #[test]
    fn sanitize_replaces_forbidden_characters() {
        assert_eq!(sanitize_component("a/b:c?.txt"), "a_b_c_.txt");
    }

    #[test]
    fn sanitize_maps_dot_names() {
        let names: Vec<_> = ["", ".", ".."]
            .into_iter()
            .map(sanitize_component)
            .collect();
        assert_eq!(names, ["_", "_", "_"]);
    }

    #[test]
    fn sanitize_keeps_ordinary_names() {
        assert_eq!(sanitize_component("Führerschein.pdf"), "Führerschein.pdf");
    }

    #[test]
    fn summary_counts_the_current_item() {
        let items = [
            transfer(TransferState::Done, 1, 1),
            transfer(TransferState::Active, 0, 1),
            transfer(TransferState::Queued, 0, 1),
        ];
        assert_eq!(summarize(&items), "Uploading 2 of 3");
    }

    #[test]
    fn summary_reports_failures() {
        let items = [
            transfer(TransferState::Done, 1, 1),
            transfer(TransferState::Failed("boom".to_owned()), 0, 1),
        ];
        assert_eq!(summarize(&items), "1 failed, 1 completed");
    }

    #[test]
    fn overall_progress_averages_fractions() {
        let items = [
            transfer(TransferState::Done, 0, 0),
            transfer(TransferState::Active, 25, 100),
        ];
        assert!((overall_progress(&items) - 0.625).abs() < f32::EPSILON);
    }

    #[test]
    fn part_path_appends_the_suffix() {
        assert_eq!(
            part_path(Path::new("/tmp/a.pdf")),
            PathBuf::from("/tmp/a.pdf.part")
        );
    }

    #[tokio::test]
    async fn local_plan_lists_folders_before_their_files() {
        let root = std::env::temp_dir().join(format!("home-space-plan-{}", std::process::id()));
        let inner = root.join("inner");
        fs::create_dir_all(&inner).await.expect("create dirs");
        fs::write(inner.join("a.txt"), b"hello")
            .await
            .expect("write file");

        let plan = plan_upload(vec![root.clone()]).await.expect("plan");
        fs::remove_dir_all(&root).await.expect("cleanup");

        let top = display_name(&root);
        assert_eq!(
            plan.folders,
            [vec![top.clone()], vec![top.clone(), "inner".to_owned()]]
        );
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.files[0].folder, [top, "inner".to_owned()]);
        assert_eq!(plan.files[0].size, 5);
    }
}
