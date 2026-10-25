use std::io;

use bytes::Bytes;
use futures_util::Stream;
use tokio::fs;

use db::{Db, NewNode, Node, NodeId, NodeKind, NodeUpdate, UserId};

use crate::{BlobStore, FilesError, store::StagedBlob};

/// Ownership-scoped operations on a user's folders and files.
#[derive(Clone, Debug)]
pub struct FileService {
    db: Db,
    store: BlobStore,
    max_upload_bytes: u64,
}

impl FileService {
    pub fn new(db: Db, store: BlobStore, max_upload_bytes: u64) -> Self {
        Self {
            db,
            store,
            max_upload_bytes,
        }
    }

    /// Creates a folder under `parent` (or at the root).
    pub async fn create_folder(
        &self,
        owner: UserId,
        parent: Option<NodeId>,
        name: &str,
    ) -> Result<Node, FilesError> {
        let new = NewNode {
            id: NodeId::new(),
            parent_id: parent,
            kind: NodeKind::Folder,
            name: name.to_owned(),
            size: 0,
            sha256: None,
            mime: None,
        };
        Ok(self.db.create_node(owner, new).await?)
    }

    /// Stores a new file streamed from `body`.
    pub async fn upload<S>(
        &self,
        owner: UserId,
        parent: Option<NodeId>,
        name: &str,
        body: S,
    ) -> Result<Node, FilesError>
    where
        S: Stream<Item = io::Result<Bytes>>,
    {
        if let Some(parent) = parent {
            self.db.get_node(owner, parent).await?;
        }
        let staged = self.store.stage(body, self.max_upload_bytes).await?;
        let new = NewNode {
            id: NodeId::new(),
            parent_id: parent,
            kind: NodeKind::File,
            name: name.to_owned(),
            size: size_as_i64(&staged),
            sha256: Some(staged.sha256.clone()),
            mime: guess_mime(name),
        };
        let id = new.id;
        match self.db.create_node(owner, new).await {
            Ok(node) => match self.store.commit(staged, owner, id).await {
                Ok(()) => Ok(node),
                Err(err) => {
                    self.rollback_node(owner, id).await;
                    Err(err.into())
                }
            },
            Err(err) => {
                self.store.discard(staged).await;
                Err(err.into())
            }
        }
    }

    /// Replaces the content of an existing file.
    pub async fn replace_content<S>(
        &self,
        owner: UserId,
        id: NodeId,
        body: S,
    ) -> Result<Node, FilesError>
    where
        S: Stream<Item = io::Result<Bytes>>,
    {
        let current = self.db.get_node(owner, id).await?;
        if current.kind != NodeKind::File {
            return Err(FilesError::NotAFile);
        }
        let staged = self.store.stage(body, self.max_upload_bytes).await?;
        let size = size_as_i64(&staged);
        let sha256 = staged.sha256.clone();
        self.store.commit(staged, owner, id).await?;
        Ok(self
            .db
            .replace_file_content(owner, id, size, sha256, guess_mime(&current.name))
            .await?)
    }

    /// Opens a file for download.
    pub async fn download(
        &self,
        owner: UserId,
        id: NodeId,
    ) -> Result<(Node, fs::File), FilesError> {
        let node = self.db.get_node(owner, id).await?;
        if node.kind == NodeKind::File {
            let file = self.store.open_blob(owner, id).await?;
            Ok((node, file))
        } else {
            Err(FilesError::NotAFile)
        }
    }

    /// Returns a node's metadata.
    pub async fn get(&self, owner: UserId, id: NodeId) -> Result<Node, FilesError> {
        Ok(self.db.get_node(owner, id).await?)
    }

    /// Lists the ancestors of a node from the root down to its parent.
    pub async fn ancestors(&self, owner: UserId, id: NodeId) -> Result<Vec<Node>, FilesError> {
        Ok(self.db.list_ancestors(owner, id).await?)
    }

    /// Lists a folder (or the root when `parent` is `None`).
    pub async fn list(
        &self,
        owner: UserId,
        parent: Option<NodeId>,
    ) -> Result<Vec<Node>, FilesError> {
        Ok(self.db.list_children(owner, parent).await?)
    }

    /// Renames and/or moves a node.
    pub async fn update(
        &self,
        owner: UserId,
        id: NodeId,
        update: NodeUpdate,
    ) -> Result<Node, FilesError> {
        Ok(self.db.update_node(owner, id, update).await?)
    }

    /// Deletes a node, its descendants and their stored content.
    pub async fn delete(&self, owner: UserId, id: NodeId) -> Result<Node, FilesError> {
        let node = self.db.get_node(owner, id).await?;
        let file_ids = self.db.delete_node(owner, id).await?;
        for file_id in file_ids {
            if let Err(err) = self.store.remove(owner, file_id).await {
                tracing::warn!(%file_id, error = %err, "failed to remove blob");
            }
        }
        Ok(node)
    }

    async fn rollback_node(&self, owner: UserId, id: NodeId) {
        if let Err(err) = self.db.delete_node(owner, id).await {
            tracing::error!(node_id = %id, error = %err, "failed to roll back node");
        }
    }
}

fn size_as_i64(staged: &StagedBlob) -> i64 {
    i64::try_from(staged.size).unwrap_or(i64::MAX)
}

fn guess_mime(name: &str) -> Option<String> {
    mime_guess::from_path(name)
        .first()
        .map(|mime| mime.essence_str().to_owned())
}

#[cfg(test)]
mod tests {
    use futures_util::stream;
    use sqlx::PgPool;
    use tokio::io::AsyncReadExt;

    use super::*;

    struct Fixture {
        files: FileService,
        alice: UserId,
        bob: UserId,
        _dir: tempfile::TempDir,
    }

    fn body(text: &'static str) -> impl Stream<Item = io::Result<Bytes>> + use<> {
        stream::iter([Ok(Bytes::from_static(text.as_bytes()))])
    }

    async fn fixture(pool: PgPool, max_upload_bytes: u64) -> Fixture {
        let db = Db::from_pool(pool);
        let dir = tempfile::tempdir().expect("tempdir");
        let store = BlobStore::open(dir.path()).await.expect("store");
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let bob = db.create_user("bob", "x").await.expect("bob").id;
        Fixture {
            files: FileService::new(db, store, max_upload_bytes),
            alice,
            bob,
            _dir: dir,
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn upload_then_download_returns_same_content(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let node = fx
            .files
            .upload(fx.alice, None, "note.txt", body("hello"))
            .await
            .expect("upload");
        let (meta, mut file) = fx
            .files
            .download(fx.alice, node.id)
            .await
            .expect("download");
        let mut content = String::new();
        file.read_to_string(&mut content).await.expect("read");
        assert_eq!((content.as_str(), meta.size), ("hello", 5));
        assert_eq!(meta.mime.as_deref(), Some("text/plain"));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn ancestors_run_from_root_to_parent(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let a = fx
            .files
            .create_folder(fx.alice, None, "a")
            .await
            .expect("a");
        let b = fx
            .files
            .create_folder(fx.alice, Some(a.id), "b")
            .await
            .expect("b");
        let file = fx
            .files
            .upload(fx.alice, Some(b.id), "f.txt", body("x"))
            .await
            .expect("file");
        let names = fx
            .files
            .ancestors(fx.alice, file.id)
            .await
            .expect("ancestors")
            .into_iter()
            .map(|node| node.name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["a", "b"]);
        assert!(
            fx.files
                .ancestors(fx.alice, a.id)
                .await
                .expect("root level")
                .is_empty()
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn ancestors_of_another_users_node_are_not_found(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let a = fx
            .files
            .create_folder(fx.alice, None, "a")
            .await
            .expect("a");
        assert!(matches!(
            fx.files.ancestors(fx.bob, a.id).await,
            Err(FilesError::NotFound)
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn other_users_cannot_access_a_node(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let node = fx
            .files
            .upload(fx.alice, None, "note.txt", body("hello"))
            .await
            .expect("upload");
        assert!(matches!(
            fx.files.get(fx.bob, node.id).await,
            Err(FilesError::NotFound)
        ));
        assert!(matches!(
            fx.files.download(fx.bob, node.id).await,
            Err(FilesError::NotFound)
        ));
        assert!(matches!(
            fx.files.delete(fx.bob, node.id).await,
            Err(FilesError::NotFound)
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn cannot_upload_into_another_users_folder(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let folder = fx
            .files
            .create_folder(fx.alice, None, "docs")
            .await
            .expect("folder");
        let result = fx
            .files
            .upload(fx.bob, Some(folder.id), "x.txt", body("hi"))
            .await;
        assert!(matches!(result, Err(FilesError::NotFound)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn duplicate_name_is_a_conflict(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        fx.files
            .upload(fx.alice, None, "a.txt", body("1"))
            .await
            .expect("first");
        let result = fx.files.upload(fx.alice, None, "a.txt", body("2")).await;
        assert!(matches!(result, Err(FilesError::Conflict)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn oversized_upload_is_rejected(pool: PgPool) {
        let fx = fixture(pool, 3).await;
        let result = fx
            .files
            .upload(fx.alice, None, "a.txt", body("hello"))
            .await;
        assert!(matches!(result, Err(FilesError::TooLarge { limit: 3 })));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn invalid_name_is_rejected(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let result = fx.files.create_folder(fx.alice, None, "a/b").await;
        assert!(matches!(result, Err(FilesError::InvalidName)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn folder_cannot_move_into_its_descendant(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let outer = fx
            .files
            .create_folder(fx.alice, None, "outer")
            .await
            .expect("outer");
        let inner = fx
            .files
            .create_folder(fx.alice, Some(outer.id), "inner")
            .await
            .expect("inner");
        let update = NodeUpdate {
            parent_id: Some(Some(inner.id)),
            ..NodeUpdate::default()
        };
        let result = fx.files.update(fx.alice, outer.id, update).await;
        assert!(matches!(result, Err(FilesError::Cycle)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn deleting_a_folder_removes_its_blobs(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let folder = fx
            .files
            .create_folder(fx.alice, None, "docs")
            .await
            .expect("folder");
        let file = fx
            .files
            .upload(fx.alice, Some(folder.id), "a.txt", body("hi"))
            .await
            .expect("upload");
        fx.files.delete(fx.alice, folder.id).await.expect("delete");
        assert!(matches!(
            fx.files.get(fx.alice, file.id).await,
            Err(FilesError::NotFound)
        ));
        assert!(fx.files.store.open_blob(fx.alice, file.id).await.is_err());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn replace_content_updates_size_and_hash(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let file = fx
            .files
            .upload(fx.alice, None, "a.txt", body("hi"))
            .await
            .expect("upload");
        let updated = fx
            .files
            .replace_content(fx.alice, file.id, body("longer"))
            .await
            .expect("replace");
        assert_eq!(updated.size, 6);
        assert_ne!(updated.sha256, file.sha256);
    }
}
