use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use iced::futures::{StreamExt, stream};
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};
use tokio_util::io::ReaderStream;

use api_client::{ApiError, Client};
use protocol::{ErrorCode, NodeId, NodeKind};

use super::{
    model::{Base, LocalEntry, RemoteNode, datetime, nanos},
    plan::Action,
    scan::is_ignored,
};
use crate::transfers::part_path;

/// What a finished action changed, for the engine to record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Base(Base),
    /// Drops the base of a node; with a path, only while it is still recorded there.
    Forget(NodeId, Option<PathBuf>),
    Remote(RemoteNode),
    RemoteGone(NodeId),
    /// Everything recorded at and under `from` is now under `to`.
    Rebase {
        from: PathBuf,
        to: PathBuf,
    },
    /// A remote folder now stands for the local folder at the path.
    Folder(PathBuf, NodeId),
}

/// Why an action did not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The local file changed after the scan; the next pass looks again.
    Stale,
    Error(String),
    SignedOut,
}

impl From<ApiError> for Failure {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::Unauthorized => Self::SignedOut,
            other => Self::Error(other.to_string()),
        }
    }
}

impl From<io::Error> for Failure {
    fn from(err: io::Error) -> Self {
        Self::Error(err.to_string())
    }
}

/// The remote folder for each local folder that has one, `None` standing for Home.
pub type Folders = HashMap<PathBuf, Option<NodeId>>;

/// Carries out `action`.
pub async fn execute(
    client: &Client,
    folders: &Folders,
    action: Action,
) -> Result<Vec<Effect>, Failure> {
    match action {
        Action::MoveLocal { from, to, .. } => {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).await?;
            }
            if fs::try_exists(&to).await? {
                return Err(Failure::Stale);
            }
            fs::rename(&from, &to).await?;
            Ok(vec![Effect::Rebase { from, to }])
        }
        Action::MoveRemote { node, from, to } => {
            let (parent, name) = place(folders, &to)?;
            let moved = client.relocate(node, parent, &name).await?;
            Ok(vec![
                Effect::Remote(moved.into()),
                Effect::Rebase { from, to },
            ])
        }
        Action::CreateLocalDir { node, path } => {
            fs::create_dir_all(&path).await?;
            Ok(vec![Effect::Base(Base::new(
                node,
                path,
                &LocalEntry::folder(),
            ))])
        }
        Action::CreateRemoteDir { path } => {
            let (parent, name) = place(folders, &path)?;
            let folder = match client.create_folder(parent, &name).await {
                Err(ApiError::Api {
                    code: ErrorCode::Conflict,
                    ..
                }) => client
                    .list(parent)
                    .await?
                    .into_iter()
                    .find(|node| node.kind == NodeKind::Folder && node.name == name)
                    .ok_or_else(|| {
                        Failure::Error("a file of this name is on the server".to_owned())
                    })?,
                other => other?,
            };
            let id = folder.id;
            Ok(vec![
                Effect::Remote(folder.into()),
                Effect::Folder(path.clone(), id),
                Effect::Base(Base::new(id, path, &LocalEntry::folder())),
            ])
        }
        Action::Download { node, path, expect } => {
            let base = download(client, &node, &path, expect.as_ref()).await?;
            Ok(vec![Effect::Base(base)])
        }
        Action::Upload { node, path, local } => upload(client, folders, node, path, local).await,
        Action::Adopt { node, path, local } => {
            Ok(vec![Effect::Base(Base::new(node, path, &local))])
        }
        Action::DeleteRemote { node, .. } => {
            match client.delete(node).await {
                Err(ApiError::Api {
                    code: ErrorCode::NotFound,
                    ..
                }) => {}
                other => other?,
            }
            Ok(vec![Effect::RemoteGone(node), Effect::Forget(node, None)])
        }
        Action::TrashLocal { node, path, expect } => {
            unchanged(&path, Some(&expect)).await?;
            let target = path.clone();
            tokio::task::spawn_blocking(move || trash::delete(target))
                .await
                .map_err(|err| Failure::Error(err.to_string()))?
                .map_err(|err| Failure::Error(err.to_string()))?;
            Ok(vec![Effect::Forget(node, Some(path))])
        }
        Action::RemoveLocalDir { node, path } => {
            let target = path.clone();
            let removed = tokio::task::spawn_blocking(move || remove_if_empty(&target))
                .await
                .map_err(|err| Failure::Error(err.to_string()))??;
            // A folder still holding something unsynced is left; the next pass tries again.
            Ok(if removed {
                vec![Effect::Forget(node, Some(path))]
            } else {
                Vec::new()
            })
        }
        Action::Forget { node } => Ok(vec![Effect::Forget(node, None)]),
        Action::Blocked { reason, .. } => Err(Failure::Error(reason)),
    }
}

/// Carries out `actions` at most `limit` at a time, yielding each with its outcome.
pub fn execute_all<'a>(
    client: &'a Client,
    folders: &'a Folders,
    actions: Vec<Action>,
    limit: usize,
) -> impl iced::futures::Stream<Item = (Action, Result<Vec<Effect>, Failure>)> + 'a {
    stream::iter(actions)
        .map(move |action| async move {
            let outcome = execute(client, folders, action.clone()).await;
            (action, outcome)
        })
        .buffer_unordered(limit)
}

/// The remote parent and name of the local `path`.
fn place(folders: &Folders, path: &Path) -> Result<(Option<NodeId>, String), Failure> {
    let parent = path
        .parent()
        .and_then(|parent| folders.get(parent))
        .ok_or_else(|| Failure::Error("its folder is not on the server yet".to_owned()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Failure::Error("the name is not valid Unicode".to_owned()))?;
    Ok((*parent, name.to_owned()))
}

async fn upload(
    client: &Client,
    folders: &Folders,
    node: Option<NodeId>,
    path: PathBuf,
    local: LocalEntry,
) -> Result<Vec<Effect>, Failure> {
    unchanged(&path, Some(&local)).await?;
    let modified = Some(datetime(local.mtime));
    let body = || file_body(&path);
    let stored = match node {
        Some(id) => client.replace_content(id, modified, body).await?,
        None => {
            let (parent, name) = place(folders, &path)?;
            client
                .upload_modified(parent, &name, modified, body)
                .await?
        }
    };
    let stored = RemoteNode::from(stored);
    let base = if stored.sha256 == local.sha256 {
        Base::new(stored.id, path, &local)
    } else {
        // The server's content changed later and stays current; ours is now a version.
        download(client, &stored, &path, Some(&local)).await?
    };
    Ok(vec![Effect::Remote(stored), Effect::Base(base)])
}

/// Writes the content of `node` to `path` through a partial file, unless the local file is
/// no longer `expect`. The file takes the node's modification time.
async fn download(
    client: &Client,
    node: &RemoteNode,
    path: &Path,
    expect: Option<&LocalEntry>,
) -> Result<Base, Failure> {
    unchanged(path, expect).await?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let response = client.download(node.id).await?;
    let part = part_path(path);
    let written = write_part(response, &part, node).await;
    let sha256 = match written {
        Ok(sha256) => sha256,
        Err(err) => {
            let _ = fs::remove_file(&part).await;
            return Err(err.into());
        }
    };
    if let Err(failure) = unchanged(path, expect).await {
        let _ = fs::remove_file(&part).await;
        return Err(failure);
    }
    fs::rename(&part, path).await?;
    let meta = fs::metadata(path).await?;
    Ok(Base {
        node: node.id,
        path: path.to_path_buf(),
        kind: NodeKind::File,
        sha256: Some(sha256),
        size: meta.len(),
        mtime: meta.modified().map(nanos).unwrap_or(0),
    })
}

/// Streams the response into `part`, returning the hash of what was written.
async fn write_part(
    response: reqwest::Response,
    part: &Path,
    node: &RemoteNode,
) -> io::Result<String> {
    let mut out = fs::File::create(part).await?;
    let mut hasher = Sha256::new();
    let mut body = response.bytes_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(io::Error::other)?;
        hasher.update(&chunk);
        out.write_all(&chunk).await?;
    }
    out.flush().await?;
    let out = out.into_std().await;
    let modified = SystemTime::from(node.updated_at);
    tokio::task::spawn_blocking(move || out.set_modified(modified))
        .await
        .map_err(io::Error::other)??;
    Ok(hex::encode(hasher.finalize()))
}

/// Fails as [`Failure::Stale`] unless the file at `path` still has the size and time of
/// `expect`, or is still absent when `expect` is `None`.
async fn unchanged(path: &Path, expect: Option<&LocalEntry>) -> Result<(), Failure> {
    let meta = match fs::symlink_metadata(path).await {
        Ok(meta) => Some(meta),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err.into()),
    };
    let same = match (meta, expect) {
        (None, None) => true,
        (Some(meta), Some(expect)) => {
            meta.is_file()
                && meta.len() == expect.size
                && meta.modified().map(nanos).unwrap_or(0) == expect.mtime
        }
        _ => false,
    };
    if same { Ok(()) } else { Err(Failure::Stale) }
}

fn file_body(path: &Path) -> reqwest::Body {
    match std::fs::File::open(path) {
        Ok(file) => reqwest::Body::wrap_stream(ReaderStream::new(fs::File::from_std(file))),
        Err(err) => {
            reqwest::Body::wrap_stream(stream::once(async move { Err::<Vec<u8>, io::Error>(err) }))
        }
    }
}

/// Removes the folder at `path` when it holds nothing but files the sync ignores.
fn remove_if_empty(path: &Path) -> Result<bool, Failure> {
    let mut clutter = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let ignored =
            entry.file_type()?.is_file() && entry.file_name().to_str().is_some_and(is_ignored);
        if !ignored {
            return Ok(false);
        }
        clutter.push(entry.path());
    }
    for file in clutter {
        std::fs::remove_file(file)?;
    }
    std::fs::remove_dir(path)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("home-space-apply-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp dir");
        path
    }

    #[test]
    fn a_folder_with_only_clutter_is_removed() {
        let dir = temp("clutter");
        std::fs::write(dir.join(".DS_Store"), "x").expect("clutter");
        assert_eq!(remove_if_empty(&dir), Ok(true));
        assert!(!dir.exists());
    }

    #[test]
    fn a_folder_with_a_real_file_stays() {
        let dir = temp("real");
        std::fs::write(dir.join("a.txt"), "x").expect("file");
        assert_eq!(remove_if_empty(&dir), Ok(false));
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[tokio::test]
    async fn a_file_that_changed_since_the_scan_is_stale() {
        let dir = temp("stale");
        let path = dir.join("a.txt");
        std::fs::write(&path, "x").expect("file");
        let meta = std::fs::metadata(&path).expect("meta");
        let scanned = LocalEntry {
            kind: NodeKind::File,
            size: meta.len(),
            mtime: nanos(meta.modified().expect("mtime")),
            sha256: None,
        };
        assert_eq!(unchanged(&path, Some(&scanned)).await, Ok(()));
        assert_eq!(unchanged(&path, None).await, Err(Failure::Stale));
        std::fs::write(&path, "longer").expect("change");
        assert_eq!(unchanged(&path, Some(&scanned)).await, Err(Failure::Stale));
        assert_eq!(unchanged(&dir.join("missing"), None).await, Ok(()));
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn place_finds_the_remote_parent_and_name() {
        let folders = Folders::from([(PathBuf::from("/sync"), None)]);
        assert_eq!(
            place(&folders, Path::new("/sync/a.txt")),
            Ok((None, "a.txt".to_owned()))
        );
        assert!(place(&folders, Path::new("/sync/new/a.txt")).is_err());
    }
}
