use std::{
    io,
    path::{Path, PathBuf},
};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};

use uuid::Uuid;

use db::{NodeId, UserId};

use crate::FilesError;

const BLOBS_DIR: &str = "blobs";
const VERSIONS_DIR: &str = "versions";
const TMP_DIR: &str = "tmp";

/// A fully received upload waiting to be committed into the store.
#[derive(Debug)]
pub struct StagedBlob {
    path: PathBuf,
    pub size: u64,
    pub sha256: Vec<u8>,
}

/// Blob storage on the local file system.
///
/// Layout: `<root>/blobs/<user_id>/<node_id>` and `<root>/versions/<user_id>/<version_id>`; uploads are staged in `<root>/tmp`.
#[derive(Clone, Debug)]
pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    /// Creates the directory layout under `root` if it does not exist.
    pub async fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join(BLOBS_DIR)).await?;
        fs::create_dir_all(root.join(VERSIONS_DIR)).await?;
        fs::create_dir_all(root.join(TMP_DIR)).await?;
        Ok(Self { root })
    }

    /// Streams `body` into a temp file, hashing it and enforcing `max_bytes`.
    pub async fn stage<S>(&self, body: S, max_bytes: u64) -> Result<StagedBlob, FilesError>
    where
        S: Stream<Item = io::Result<Bytes>>,
    {
        let path = self.root.join(TMP_DIR).join(NodeId::new().to_string());
        let staged = Self::write_stream(&path, body, max_bytes).await;
        match staged {
            Ok((size, sha256)) => Ok(StagedBlob { path, size, sha256 }),
            Err(err) => {
                let _ = fs::remove_file(&path).await;
                Err(err)
            }
        }
    }

    /// Moves a staged blob into its final place, replacing any previous content.
    pub async fn commit(&self, staged: StagedBlob, owner: UserId, node: NodeId) -> io::Result<()> {
        let target = self.blob_path(owner, node);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir).await?;
        }
        fs::rename(&staged.path, target).await
    }

    /// Moves a staged blob into place as the stored content of a version.
    pub async fn commit_version(
        &self,
        staged: StagedBlob,
        owner: UserId,
        version: Uuid,
    ) -> io::Result<()> {
        let target = self.version_path(owner, version);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir).await?;
        }
        fs::rename(&staged.path, target).await
    }

    /// Discards a staged blob.
    pub async fn discard(&self, staged: StagedBlob) {
        let _ = fs::remove_file(&staged.path).await;
    }

    /// Opens the stored content of a node.
    pub async fn open_blob(&self, owner: UserId, node: NodeId) -> io::Result<fs::File> {
        fs::File::open(self.blob_path(owner, node)).await
    }

    /// Removes the stored content of a node; a missing blob is not an error.
    pub async fn remove(&self, owner: UserId, node: NodeId) -> io::Result<()> {
        match fs::remove_file(self.blob_path(owner, node)).await {
            Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
            _ => Ok(()),
        }
    }

    /// Keeps the current content of `node` as the stored content of a version.
    pub async fn archive(&self, owner: UserId, node: NodeId, version: Uuid) -> io::Result<()> {
        let target = self.version_path(owner, version);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir).await?;
        }
        fs::rename(self.blob_path(owner, node), target).await
    }

    /// Makes the content of node `from` the content of node `to`, replacing it.
    pub async fn promote(&self, owner: UserId, from: NodeId, to: NodeId) -> io::Result<()> {
        fs::rename(self.blob_path(owner, from), self.blob_path(owner, to)).await
    }

    /// Opens the stored content of a version.
    pub async fn open_version(&self, owner: UserId, version: Uuid) -> io::Result<fs::File> {
        fs::File::open(self.version_path(owner, version)).await
    }

    /// Removes the stored content of a version; a missing blob is not an error.
    pub async fn remove_version(&self, owner: UserId, version: Uuid) -> io::Result<()> {
        match fs::remove_file(self.version_path(owner, version)).await {
            Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
            _ => Ok(()),
        }
    }

    fn version_path(&self, owner: UserId, version: Uuid) -> PathBuf {
        self.root
            .join(VERSIONS_DIR)
            .join(owner.to_string())
            .join(version.to_string())
    }

    fn blob_path(&self, owner: UserId, node: NodeId) -> PathBuf {
        self.root
            .join(BLOBS_DIR)
            .join(owner.to_string())
            .join(node.to_string())
    }

    async fn write_stream<S>(
        path: &Path,
        body: S,
        max_bytes: u64,
    ) -> Result<(u64, Vec<u8>), FilesError>
    where
        S: Stream<Item = io::Result<Bytes>>,
    {
        futures_util::pin_mut!(body);
        let mut file = fs::File::create(path).await?;
        let mut hasher = Sha256::new();
        let mut size = 0u64;
        while let Some(chunk) = body.next().await {
            let chunk = chunk?;
            size += chunk.len() as u64;
            if size > max_bytes {
                return Err(FilesError::TooLarge { limit: max_bytes });
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }
        file.sync_all().await?;
        Ok((size, hasher.finalize().to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use futures_util::stream;

    use super::*;

    fn body(parts: &[&'static str]) -> impl Stream<Item = io::Result<Bytes>> + use<> {
        stream::iter(
            parts
                .iter()
                .map(|part| Ok(Bytes::from_static(part.as_bytes())))
                .collect::<Vec<_>>(),
        )
    }

    #[tokio::test]
    async fn staged_blob_reports_size_and_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = BlobStore::open(dir.path()).await.expect("open");
        let staged = store.stage(body(&["hel", "lo"]), 100).await.expect("stage");
        assert_eq!(staged.size, 5);
        assert_eq!(staged.sha256, Sha256::digest(b"hello").to_vec());
    }

    #[tokio::test]
    async fn oversized_upload_is_rejected_and_cleaned_up() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = BlobStore::open(dir.path()).await.expect("open");
        let result = store.stage(body(&["hello"]), 4).await;
        assert!(matches!(result, Err(FilesError::TooLarge { limit: 4 })));
        let mut leftovers = fs::read_dir(dir.path().join(TMP_DIR))
            .await
            .expect("read_dir");
        assert!(leftovers.next_entry().await.expect("entry").is_none());
    }

    #[tokio::test]
    async fn committed_blob_can_be_read_and_removed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = BlobStore::open(dir.path()).await.expect("open");
        let (owner, node) = (UserId::new(), NodeId::new());
        let staged = store.stage(body(&["data"]), 100).await.expect("stage");
        store.commit(staged, owner, node).await.expect("commit");
        assert!(store.open_blob(owner, node).await.is_ok());
        store.remove(owner, node).await.expect("remove");
        assert!(store.open_blob(owner, node).await.is_err());
    }
}
