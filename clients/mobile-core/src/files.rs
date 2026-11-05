//! Files kept on the phone: downloading them with progress, and finding them again.
//!
//! A copy lives at `<files_dir>/<user id>/<node id>/<sha256>/<name>`. The hash in the path is the
//! only record of what was downloaded, so new content on the server simply misses and is fetched
//! on the next open. Nothing tracks these files yet: a copy changes only when the user opens it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use client_core::format::sanitize_component;
use protocol::{Node, NodeId, NodeKind};
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;

use crate::{CoreClient, CoreError, Entry, EntryKind, off_ui_thread, parse_id};

/// A partial download, inside the node's folder. It sits outside the name's folder because a
/// name at the file system's limit leaves no room for a suffix.
const PART_FILE: &str = "download.part";
/// The longest file name, in bytes, on the file systems the apps run on.
const MAX_NAME_BYTES: usize = 255;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
/// How often a download starts over because the file was replaced while it ran.
const ATTEMPTS: usize = 2;

/// Told how far a download has come.
///
/// Called on a background thread, so hand the numbers to the UI thread. It must not throw.
#[uniffi::export(with_foreign)]
pub trait DownloadListener: Send + Sync {
    /// `done` of `total` bytes of the file `id` have arrived.
    fn progress(&self, id: String, done: i64, total: i64);
}

/// The downloads in progress and where their files go.
#[derive(Default)]
pub(crate) struct LocalFiles {
    /// `None` for a client that never signs in, which keeps no files.
    root: Option<PathBuf>,
    user: Mutex<Option<String>>,
    active: Mutex<HashMap<NodeId, watch::Sender<bool>>>,
}

impl LocalFiles {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root: Some(root),
            ..Self::default()
        }
    }

    pub(crate) fn set_user(&self, user: String) {
        *self.user.lock().expect("files user lock") = Some(user);
    }

    fn user_dir(&self) -> Option<PathBuf> {
        let user = self.user.lock().expect("files user lock").clone()?;
        Some(self.root.as_ref()?.join(user))
    }

    pub(crate) fn node_dir(&self, id: NodeId) -> Result<PathBuf, CoreError> {
        self.user_dir()
            .map(|dir| dir.join(id.to_string()))
            .ok_or_else(|| CoreError::Failed {
                code: "no_files_dir".to_owned(),
                detail: "this client keeps no files".to_owned(),
            })
    }

    /// Fills in `local_path` of the files that have a copy.
    pub(crate) async fn mark_copies(&self, entries: &mut [Entry]) {
        let Some(user_dir) = self.user_dir() else {
            return;
        };
        let Ok(mut read) = fs::read_dir(&user_dir).await else {
            return;
        };
        let mut present = HashSet::new();
        while let Ok(Some(item)) = read.next_entry().await {
            present.insert(item.file_name().to_string_lossy().into_owned());
        }
        for entry in entries.iter_mut() {
            if entry.kind != EntryKind::File || !present.contains(&entry.id) {
                continue;
            }
            let key = entry
                .sha256
                .clone()
                .unwrap_or_else(|| version_key(entry.version));
            let dir = user_dir.join(&entry.id).join(key);
            entry.local_path = find_file(&dir)
                .await
                .map(|path| path.to_string_lossy().into_owned());
        }
    }

    /// Forgets the copy of `id`, if there is one.
    pub(crate) async fn remove(&self, id: NodeId) {
        if let Ok(dir) = self.node_dir(id) {
            let _ = fs::remove_dir_all(dir).await;
        }
    }

    pub(crate) fn cancel_all(&self) {
        for cancel in self.active.lock().expect("downloads lock").values() {
            cancel.send_replace(true);
        }
    }

    /// Removes every user's copies.
    pub(crate) async fn wipe(&self) {
        if let Some(root) = &self.root {
            let _ = fs::remove_dir_all(root).await;
        }
    }
}

/// One running download of a file; a second is refused until this is dropped.
struct Active {
    files: Arc<LocalFiles>,
    id: NodeId,
    cancelled: watch::Receiver<bool>,
}

impl Active {
    fn start(files: &Arc<LocalFiles>, id: NodeId) -> Result<Self, CoreError> {
        let mut active = files.active.lock().expect("downloads lock");
        if active.contains_key(&id) {
            return Err(CoreError::Failed {
                code: "already_downloading".to_owned(),
                detail: "this file is already downloading".to_owned(),
            });
        }
        let (cancel, cancelled) = watch::channel(false);
        active.insert(id, cancel);
        Ok(Self {
            files: files.clone(),
            id,
            cancelled,
        })
    }

    fn check(&self) -> Result<(), CoreError> {
        if *self.cancelled.borrow() {
            Err(CoreError::Cancelled)
        } else {
            Ok(())
        }
    }
}

impl Drop for Active {
    fn drop(&mut self) {
        self.files
            .active
            .lock()
            .expect("downloads lock")
            .remove(&self.id);
    }
}

#[uniffi::export]
impl CoreClient {
    /// A local copy of the file to show in a viewer: the one already here if it is current, else
    /// downloaded first. While the server can't be reached, any copy will do.
    pub async fn open_copy(
        &self,
        id: String,
        listener: Arc<dyn DownloadListener>,
    ) -> Result<String, CoreError> {
        let id = parse_id(&id)?;
        let client = self.inner.clone();
        let files = self.files.clone();
        off_ui_thread(async move {
            let active = Active::start(&files, id)?;
            let node_dir = files.node_dir(id)?;
            let node = match client.get(id).await {
                Ok(node) => node,
                Err(err) if err.is_unreachable() => {
                    return match any_copy(&node_dir).await {
                        Some(copy) => Ok(path_text(&copy)),
                        None => Err(err.into()),
                    };
                }
                Err(err) => return Err(err.into()),
            };
            require_file(&node)?;
            let name = local_name(&node.name);
            if let Some(copy) = find_file(&node_dir.join(content_key(&node))).await {
                return named(copy, &name).await;
            }
            fs::create_dir_all(&node_dir).await.map_err(storage)?;
            let part = node_dir.join(PART_FILE);
            let node = fetch(&client, node, &part, &active, listener.as_ref()).await?;
            let name = local_name(&node.name);
            let key = content_key(&node);
            let dir = node_dir.join(&key);
            fs::create_dir_all(&dir).await.map_err(storage)?;
            let path = dir.join(name);
            fs::rename(&part, &path).await.map_err(storage)?;
            remove_older_copies(&node_dir, &key).await;
            Ok(path_text(&path))
        })
        .await
    }

    /// Saves the file at `dest_path`, which ends up complete or not at all. It doesn't count as a
    /// copy: this file gets no badge from it.
    pub async fn save_copy(
        &self,
        id: String,
        dest_path: String,
        listener: Arc<dyn DownloadListener>,
    ) -> Result<(), CoreError> {
        let id = parse_id(&id)?;
        let client = self.inner.clone();
        let files = self.files.clone();
        off_ui_thread(async move {
            let active = Active::start(&files, id)?;
            let node = client.get(id).await?;
            require_file(&node)?;
            let dest = PathBuf::from(dest_path);
            let parent = dest.parent().unwrap_or(Path::new("")).to_path_buf();
            fs::create_dir_all(&parent).await.map_err(storage)?;
            if let Ok(node_dir) = files.node_dir(id)
                && let Some(copy) = find_file(&node_dir.join(content_key(&node))).await
            {
                fs::copy(copy, &dest).await.map_err(storage)?;
                return Ok(());
            }
            let part = parent.join(format!("{id}.part"));
            fetch(&client, node, &part, &active, listener.as_ref()).await?;
            fs::rename(&part, &dest).await.map_err(storage)
        })
        .await
    }

    /// Stops a running download of `id`, whose call then fails as `Cancelled`.
    pub fn cancel_download(&self, id: String) -> Result<(), CoreError> {
        let id = parse_id(&id)?;
        if let Some(cancel) = self.files.active.lock().expect("downloads lock").get(&id) {
            cancel.send_replace(true);
        }
        Ok(())
    }

    /// Deletes the phone's copy of `id`; the file on the server stays.
    pub async fn remove_local_copy(&self, id: String) -> Result<(), CoreError> {
        let id = parse_id(&id)?;
        let files = self.files.clone();
        off_ui_thread(async move { files.remove(id).await }).await;
        Ok(())
    }
}

/// Downloads to `part`, starting over once if the file was replaced meanwhile, and returns the
/// node the content belongs to. No partial file is left behind on failure.
async fn fetch(
    client: &api_client::Client,
    mut node: Node,
    part: &Path,
    active: &Active,
    listener: &dyn DownloadListener,
) -> Result<Node, CoreError> {
    for _ in 0..ATTEMPTS {
        let written = stream(client, &node, part, active, listener).await;
        let matches = match written {
            Ok(sha256) => node.sha256.as_ref().is_none_or(|expect| *expect == sha256),
            Err(err) => {
                let _ = fs::remove_file(part).await;
                return Err(err);
            }
        };
        if matches {
            return Ok(node);
        }
        node = match client.get(node.id).await {
            Ok(node) => node,
            Err(err) => {
                let _ = fs::remove_file(part).await;
                return Err(err.into());
            }
        };
    }
    let _ = fs::remove_file(part).await;
    Err(CoreError::Failed {
        code: "content_changed".to_owned(),
        detail: "the file changed while downloading".to_owned(),
    })
}

/// Writes the content of `node` to `part`, returning the hash of what arrived.
///
/// Cancelling stops only the transfer of the body. Requests before it run to their end, since one
/// of them may be refreshing the session and a rotated token must not be lost.
async fn stream(
    client: &api_client::Client,
    node: &Node,
    part: &Path,
    active: &Active,
    listener: &dyn DownloadListener,
) -> Result<String, CoreError> {
    active.check()?;
    let mut response = client.download(node.id).await?;
    active.check()?;
    let mut out = fs::File::create(part).await.map_err(storage)?;
    let mut hasher = Sha256::new();
    let mut cancelled = active.cancelled.clone();
    let id = node.id.to_string();
    let mut done: i64 = 0;
    let mut reported: Option<Instant> = None;
    loop {
        let chunk = tokio::select! {
            biased;
            _ = cancelled.wait_for(|cancelled| *cancelled) => return Err(CoreError::Cancelled),
            chunk = response.chunk() => chunk,
        };
        let Some(chunk) = chunk.map_err(|err| CoreError::Unreachable {
            detail: crate::last_cause(&err.to_string()).to_owned(),
        })?
        else {
            break;
        };
        hasher.update(&chunk);
        out.write_all(&chunk).await.map_err(storage)?;
        done = done.saturating_add(i64::try_from(chunk.len()).unwrap_or(i64::MAX));
        if reported.is_none_or(|at| at.elapsed() >= PROGRESS_INTERVAL) {
            listener.progress(id.clone(), done, node.size);
            reported = Some(Instant::now());
        }
    }
    out.flush().await.map_err(storage)?;
    active.check()?;
    listener.progress(id, done, node.size);
    Ok(hex::encode(hasher.finalize()))
}

fn require_file(node: &Node) -> Result<(), CoreError> {
    if node.kind == NodeKind::File {
        Ok(())
    } else {
        Err(CoreError::Failed {
            code: "not_a_file".to_owned(),
            detail: format!("“{}” is a folder", node.name),
        })
    }
}

fn storage(err: std::io::Error) -> CoreError {
    CoreError::Failed {
        code: "local_storage".to_owned(),
        detail: err.to_string(),
    }
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// What tells this file's content apart: its hash, or its version on the rare file without one.
fn content_key(node: &Node) -> String {
    node.sha256
        .clone()
        .unwrap_or_else(|| version_key(node.version))
}

fn version_key(version: i32) -> String {
    format!("v{version}")
}

/// `name` as a file name here: nothing the file system forbids, and short enough for it, keeping
/// the extension that viewers go by.
fn local_name(name: &str) -> String {
    let name = sanitize_component(name);
    if name.len() <= MAX_NAME_BYTES {
        return name;
    }
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && extension.len() < MAX_NAME_BYTES / 2 => {
            (stem, format!(".{extension}"))
        }
        _ => (name.as_str(), String::new()),
    };
    let room = MAX_NAME_BYTES - extension.len();
    let end = (0..=room)
        .rev()
        .find(|index| stem.is_char_boundary(*index))
        .unwrap_or(0);
    format!("{}{extension}", &stem[..end])
}

/// The file in `dir`, which holds one at most.
async fn find_file(dir: &Path) -> Option<PathBuf> {
    let mut read = fs::read_dir(dir).await.ok()?;
    while let Ok(Some(item)) = read.next_entry().await {
        if item.file_type().await.is_ok_and(|kind| kind.is_file()) {
            return Some(item.path());
        }
    }
    None
}

/// A copy of any content under the node's folder.
async fn any_copy(node_dir: &Path) -> Option<PathBuf> {
    let mut read = fs::read_dir(node_dir).await.ok()?;
    while let Ok(Some(item)) = read.next_entry().await {
        if let Some(file) = find_file(&item.path()).await {
            return Some(file);
        }
    }
    None
}

/// Gives a copy the file's current name, since a rename on the server keeps the content.
async fn named(copy: PathBuf, name: &str) -> Result<String, CoreError> {
    if copy.file_name().is_some_and(|current| current == name) {
        return Ok(path_text(&copy));
    }
    let renamed = copy.with_file_name(name);
    fs::rename(&copy, &renamed).await.map_err(storage)?;
    Ok(path_text(&renamed))
}

async fn remove_older_copies(node_dir: &Path, keep: &str) {
    let Ok(mut read) = fs::read_dir(node_dir).await else {
        return;
    };
    while let Ok(Some(item)) = read.next_entry().await {
        let is_dir = item.file_type().await.is_ok_and(|kind| kind.is_dir());
        if is_dir && item.file_name() != keep {
            let _ = fs::remove_dir_all(item.path()).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_names_fit_the_file_system_and_keep_their_extension() {
        assert_eq!(local_name("a/b.txt"), "a_b.txt");
        let long = format!("{}.pdf", "é".repeat(200));
        let name = local_name(&long);
        assert!(name.len() <= MAX_NAME_BYTES, "{}", name.len());
        assert!(name.ends_with(".pdf"));
        assert!(local_name(&"x".repeat(300)).len() <= MAX_NAME_BYTES);
    }
}
