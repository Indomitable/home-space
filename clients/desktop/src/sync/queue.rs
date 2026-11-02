//! The work ahead of the sync as the interface shows it, with the progress of transfers.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use protocol::NodeId;

use super::{apply::Folders, plan::Action};

/// Bytes moved so far by each transfer running, by local path.
pub type Progress = Arc<Mutex<HashMap<PathBuf, u64>>>;

fn lock(progress: &Progress) -> MutexGuard<'_, HashMap<PathBuf, u64>> {
    progress.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A copy of the bytes moved so far.
pub fn snapshot(progress: &Progress) -> HashMap<PathBuf, u64> {
    lock(progress).clone()
}

/// Sets the bytes moved for `path`, which also marks its transfer as running.
pub fn set(progress: &Progress, path: &Path, done: u64) {
    lock(progress).insert(path.to_path_buf(), done);
}

/// Adds `bytes` to what `path` has moved.
pub fn advance(progress: &Progress, path: &Path, bytes: u64) {
    if let Some(done) = lock(progress).get_mut(path) {
        *done += bytes;
    }
}

/// Marks a transfer as running while it is alive.
pub struct Active {
    progress: Progress,
    path: PathBuf,
}

impl Active {
    pub fn start(progress: &Progress, path: &Path) -> Self {
        set(progress, path, 0);
        Self {
            progress: Arc::clone(progress),
            path: path.to_path_buf(),
        }
    }
}

impl Drop for Active {
    fn drop(&mut self) {
        lock(&self.progress).remove(&self.path);
    }
}

/// What the sync does to a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Upload,
    Download,
    CreateRemoteFolder,
    CreateLocalFolder,
    Move,
    Delete,
    /// The path cannot be synced as it is.
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemState {
    Queued,
    Active,
    Failed(String),
}

/// One path with work ahead, or one that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub path: PathBuf,
    pub name: String,
    pub operation: Operation,
    pub size: u64,
    /// Bytes moved so far, while it is active.
    pub done: u64,
    pub state: ItemState,
    /// The node it concerns; `None` for something not on the server yet.
    pub node: Option<NodeId>,
    /// The remote folder it lands in (`Some(None)` for Home), when that is on the server.
    pub parent: Option<Option<NodeId>>,
}

impl QueueItem {
    fn new(path: &Path, operation: Operation, folders: &Folders) -> Self {
        Self {
            path: path.to_path_buf(),
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            operation,
            size: 0,
            done: 0,
            state: ItemState::Queued,
            node: None,
            parent: path
                .parent()
                .and_then(|parent| folders.get(parent))
                .copied(),
        }
    }

    /// An item for the interface's tests: not on the server, in Home.
    #[cfg(test)]
    pub fn test(
        path: PathBuf,
        operation: Operation,
        size: u64,
        done: u64,
        state: ItemState,
    ) -> Self {
        Self {
            size,
            done,
            state,
            parent: Some(None),
            ..Self::new(&path, operation, &Folders::new())
        }
    }

    /// A path that cannot be synced, with why.
    pub fn blocked(path: &Path, reason: String) -> Self {
        Self {
            state: ItemState::Failed(reason),
            ..Self::new(path, Operation::Blocked, &Folders::new())
        }
    }

    /// The item for `action`, or `None` for one that is only bookkeeping.
    pub fn of(action: &Action, folders: &Folders) -> Option<Self> {
        let item = |path: &Path, operation| Self::new(path, operation, folders);
        Some(match action {
            Action::MoveLocal { node, to, .. } | Action::MoveRemote { node, to, .. } => Self {
                node: Some(*node),
                ..item(to, Operation::Move)
            },
            Action::CreateLocalDir { node, path } => Self {
                node: Some(*node),
                ..item(path, Operation::CreateLocalFolder)
            },
            Action::CreateRemoteDir { path } => item(path, Operation::CreateRemoteFolder),
            Action::Download { node, path, .. } => Self {
                size: u64::try_from(node.size).unwrap_or_default(),
                node: Some(node.id),
                ..item(path, Operation::Download)
            },
            Action::Upload { node, path, local } => Self {
                size: local.size,
                node: *node,
                ..item(path, Operation::Upload)
            },
            Action::DeleteRemote { node, path }
            | Action::TrashLocal { node, path, .. }
            | Action::RemoveLocalDir { node, path } => Self {
                node: Some(*node),
                ..item(path, Operation::Delete)
            },
            Action::Adopt { .. } | Action::Forget { .. } | Action::Blocked { .. } => return None,
        })
    }

    /// The item as a failure with `reason`.
    pub fn failed(mut self, reason: String) -> Self {
        self.state = ItemState::Failed(reason);
        self
    }

    /// The item with the bytes `progress` has for it, active when it has any entry.
    pub fn with_progress(mut self, progress: &HashMap<PathBuf, u64>) -> Self {
        if let Some(done) = progress.get(&self.path) {
            self.done = (*done).min(self.size);
            self.state = ItemState::Active;
        }
        self
    }

    /// Whether this is a new file or folder waiting to be uploaded into `folder`, which the
    /// folder's listing cannot have yet.
    pub fn is_new_in(&self, folder: Option<NodeId>) -> bool {
        self.node.is_none()
            && matches!(
                self.operation,
                Operation::Upload | Operation::CreateRemoteFolder
            )
            && self.parent == Some(folder)
    }

    pub fn is_folder(&self) -> bool {
        self.operation == Operation::CreateRemoteFolder
    }
}

/// The items of `actions`, in order.
pub fn items(actions: &[Action], folders: &Folders) -> Vec<QueueItem> {
    actions
        .iter()
        .filter_map(|action| QueueItem::of(action, folders))
        .collect()
}

/// Totals over a queue for a one-line summary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub uploads: usize,
    pub downloads: usize,
    pub others: usize,
    pub failed: usize,
    /// Bytes still to move.
    pub bytes_left: u64,
}

pub fn summarize(items: &[QueueItem]) -> Summary {
    let mut summary = Summary::default();
    for item in items {
        if matches!(item.state, ItemState::Failed(_)) {
            summary.failed += 1;
            continue;
        }
        match item.operation {
            Operation::Upload => summary.uploads += 1,
            Operation::Download => summary.downloads += 1,
            _ => summary.others += 1,
        }
        summary.bytes_left += item.size.saturating_sub(item.done);
    }
    summary
}

#[cfg(test)]
mod tests {
    use protocol::NodeKind;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;
    use crate::sync::model::{LocalEntry, RemoteNode};

    fn id(n: u128) -> NodeId {
        NodeId(Uuid::from_u128(n))
    }

    fn file(size: u64) -> LocalEntry {
        LocalEntry {
            kind: NodeKind::File,
            size,
            mtime: 0,
            sha256: None,
        }
    }

    fn upload(node: Option<NodeId>, path: &str, size: u64) -> Action {
        Action::Upload {
            node,
            path: PathBuf::from(path),
            local: file(size),
        }
    }

    fn folders() -> Folders {
        Folders::from([
            (PathBuf::from("/sync"), None),
            (PathBuf::from("/sync/a"), Some(id(1))),
        ])
    }

    #[test]
    fn transfers_and_folders_become_items_and_bookkeeping_does_not() {
        let remote = RemoteNode {
            id: id(2),
            parent: None,
            kind: NodeKind::File,
            name: "d.txt".to_owned(),
            size: 7,
            sha256: None,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };
        let actions = [
            upload(None, "/sync/u.txt", 5),
            Action::Download {
                node: remote,
                path: PathBuf::from("/sync/d.txt"),
                expect: None,
            },
            Action::CreateRemoteDir {
                path: PathBuf::from("/sync/new"),
            },
            Action::Adopt {
                node: id(3),
                path: PathBuf::from("/sync/x"),
                local: file(1),
            },
            Action::Forget { node: id(4) },
        ];
        let items = items(&actions, &folders());
        let seen: Vec<_> = items
            .iter()
            .map(|item| (item.name.as_str(), item.operation, item.size))
            .collect();
        assert_eq!(
            seen,
            [
                ("u.txt", Operation::Upload, 5),
                ("d.txt", Operation::Download, 7),
                ("new", Operation::CreateRemoteFolder, 0),
            ]
        );
    }

    #[test]
    fn the_parent_is_the_remote_folder_of_the_local_one() {
        let folders = folders();
        let home = QueueItem::of(&upload(None, "/sync/u.txt", 1), &folders).expect("item");
        let nested = QueueItem::of(&upload(None, "/sync/a/u.txt", 1), &folders).expect("item");
        let inside_new =
            QueueItem::of(&upload(None, "/sync/new/u.txt", 1), &folders).expect("item");
        assert_eq!(home.parent, Some(None));
        assert_eq!(nested.parent, Some(Some(id(1))));
        assert_eq!(inside_new.parent, None);
        assert!(home.is_new_in(None));
        assert!(nested.is_new_in(Some(id(1))));
        assert!(!nested.is_new_in(None));
        assert!(!inside_new.is_new_in(None));
    }

    #[test]
    fn an_edited_file_is_not_new() {
        let item = QueueItem::of(&upload(Some(id(9)), "/sync/u.txt", 1), &folders()).expect("item");
        assert!(!item.is_new_in(None));
    }

    #[test]
    fn progress_makes_an_item_active_and_is_capped_at_its_size() {
        let item = QueueItem::of(&upload(None, "/sync/u.txt", 10), &folders()).expect("item");
        assert_eq!(item.state, ItemState::Queued);
        let progress = HashMap::from([(PathBuf::from("/sync/u.txt"), 40)]);
        let active = item.with_progress(&progress);
        assert_eq!((active.state, active.done), (ItemState::Active, 10));
    }

    #[test]
    fn an_active_guard_registers_and_removes_the_transfer() {
        let progress = Progress::default();
        let path = Path::new("/sync/u.txt");
        {
            let _active = Active::start(&progress, path);
            advance(&progress, path, 3);
            advance(&progress, path, 4);
            assert_eq!(lock(&progress).get(path), Some(&7));
        }
        assert!(lock(&progress).is_empty());
    }

    #[test]
    fn the_summary_counts_by_kind_and_sums_what_is_left() {
        let folders = folders();
        let mut items = items(
            &[
                upload(None, "/sync/a.txt", 10),
                upload(None, "/sync/b.txt", 20),
            ],
            &folders,
        );
        items[0].done = 4;
        items.push(QueueItem::blocked(Path::new("/sync/c"), "no".to_owned()));
        assert_eq!(
            summarize(&items),
            Summary {
                uploads: 2,
                failed: 1,
                bytes_left: 26,
                ..Summary::default()
            }
        );
    }
}
