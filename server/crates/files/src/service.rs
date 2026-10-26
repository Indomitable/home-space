use std::io;

use bytes::Bytes;
use futures_util::Stream;
use tokio::fs;

use db::{
    BlobOp, Db, FOLDER_MIME, FileVersion, MoveOutcome, NewNode, Node, NodeId, NodeKind, NodeUpdate,
    StoreOutcome, UserId,
};

use crate::{BlobStore, FilesError, store::StagedBlob};

const FALLBACK_MIME: &str = "application/octet-stream";

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
            mime: FOLDER_MIME.to_owned(),
        };
        Ok(self.db.create_node(owner, new).await?)
    }

    /// Stores a file streamed from `body`.
    ///
    /// A new name creates the file, identical content under an existing name changes nothing,
    /// and different content becomes a new version of the existing file.
    pub async fn upload<S>(
        &self,
        owner: UserId,
        parent: Option<NodeId>,
        name: &str,
        content_type: Option<&str>,
        body: S,
    ) -> Result<StoreOutcome, FilesError>
    where
        S: Stream<Item = io::Result<Bytes>>,
    {
        if let Some(parent) = parent {
            self.db.get_node(owner, parent).await?;
        }
        let staged = self.store.stage(body, self.max_upload_bytes).await?;
        let stored = self
            .db
            .store_file(
                owner,
                parent,
                name,
                size_as_i64(&staged),
                staged.sha256.clone(),
                &resolve_mime(content_type, name),
            )
            .await;
        self.finish_store(owner, staged, stored).await
    }

    /// Replaces the content of an existing file; different content becomes a new version.
    pub async fn replace_content<S>(
        &self,
        owner: UserId,
        id: NodeId,
        content_type: Option<&str>,
        body: S,
    ) -> Result<StoreOutcome, FilesError>
    where
        S: Stream<Item = io::Result<Bytes>>,
    {
        let current = self.db.get_node(owner, id).await?;
        if current.kind != NodeKind::File {
            return Err(FilesError::NotAFile);
        }
        let staged = self.store.stage(body, self.max_upload_bytes).await?;
        let stored = self
            .db
            .replace_file_content(
                owner,
                id,
                size_as_i64(&staged),
                staged.sha256.clone(),
                &resolve_mime(content_type, &current.name),
            )
            .await;
        self.finish_store(owner, staged, stored).await
    }

    async fn finish_store(
        &self,
        owner: UserId,
        staged: StagedBlob,
        stored: Result<StoreOutcome, db::DbError>,
    ) -> Result<StoreOutcome, FilesError> {
        let outcome = match stored {
            Ok(outcome) => outcome,
            Err(err) => {
                self.store.discard(staged).await;
                return Err(err.into());
            }
        };
        match &outcome {
            StoreOutcome::Unchanged(_) => self.store.discard(staged).await,
            StoreOutcome::Created(node) => {
                if let Err(err) = self.store.commit(staged, owner, node.id).await {
                    self.rollback_node(owner, node.id).await;
                    return Err(err.into());
                }
            }
            StoreOutcome::Versioned { node, archived } => {
                self.store.archive(owner, node.id, *archived).await?;
                self.store.commit(staged, owner, node.id).await?;
            }
        }
        Ok(outcome)
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
    ) -> Result<MoveOutcome, FilesError> {
        let outcome = self.db.update_node(owner, id, update).await?;
        for op in &outcome.blob_ops {
            let result = match *op {
                BlobOp::Archive { node, version_id } => {
                    self.store.archive(owner, node, version_id).await
                }
                BlobOp::Promote { from, to } => self.store.promote(owner, from, to).await,
                BlobOp::Remove { node } => self.store.remove(owner, node).await,
            };
            if let Err(err) = result {
                tracing::error!(?op, error = %err, "failed to apply blob operation");
            }
        }
        Ok(outcome)
    }

    /// Lists the previous versions of a file, newest first.
    pub async fn versions(
        &self,
        owner: UserId,
        id: NodeId,
    ) -> Result<Vec<FileVersion>, FilesError> {
        Ok(self.db.list_versions(owner, id).await?)
    }

    /// Opens a previous version of a file for download.
    pub async fn download_version(
        &self,
        owner: UserId,
        id: NodeId,
        version: i32,
    ) -> Result<(Node, FileVersion, fs::File), FilesError> {
        let node = self.db.get_node(owner, id).await?;
        let meta = self.db.get_version(owner, id, version).await?;
        let file = self.store.open_version(owner, meta.id).await?;
        Ok((node, meta, file))
    }

    /// Deletes a previous version and its stored content.
    pub async fn delete_version(
        &self,
        owner: UserId,
        id: NodeId,
        version: i32,
    ) -> Result<(), FilesError> {
        let version_id = self.db.delete_version(owner, id, version).await?;
        if let Err(err) = self.store.remove_version(owner, version_id).await {
            tracing::warn!(%version_id, error = %err, "failed to remove version blob");
        }
        Ok(())
    }

    /// Deletes a node, its descendants and their stored content.
    pub async fn delete(&self, owner: UserId, id: NodeId) -> Result<Node, FilesError> {
        let node = self.db.get_node(owner, id).await?;
        let deleted = self.db.delete_node(owner, id).await?;
        for file_id in deleted.files {
            if let Err(err) = self.store.remove(owner, file_id).await {
                tracing::warn!(%file_id, error = %err, "failed to remove blob");
            }
        }
        for version_id in deleted.versions {
            if let Err(err) = self.store.remove_version(owner, version_id).await {
                tracing::warn!(%version_id, error = %err, "failed to remove version blob");
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

/// Picks the MIME type to store for a file.
///
/// The type declared by the client wins unless it is missing, invalid or the generic
/// `application/octet-stream`, in which case it is guessed from the file name.
fn resolve_mime(declared: Option<&str>, name: &str) -> String {
    declared
        .and_then(|value| value.parse::<mime_guess::mime::Mime>().ok())
        .map(|mime| mime.essence_str().to_owned())
        .filter(|essence| essence != FALLBACK_MIME)
        .or_else(|| {
            mime_guess::from_path(name)
                .first()
                .map(|mime| mime.essence_str().to_owned())
        })
        .unwrap_or_else(|| FALLBACK_MIME.to_owned())
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
            .upload(fx.alice, None, "note.txt", None, body("hello"))
            .await
            .map(StoreOutcome::into_node)
            .expect("upload");
        let (meta, mut file) = fx
            .files
            .download(fx.alice, node.id)
            .await
            .expect("download");
        let mut content = String::new();
        file.read_to_string(&mut content).await.expect("read");
        assert_eq!((content.as_str(), meta.size), ("hello", 5));
        assert_eq!(meta.mime, "text/plain");
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
            .upload(fx.alice, Some(b.id), "f.txt", None, body("x"))
            .await
            .map(StoreOutcome::into_node)
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
            .upload(fx.alice, None, "note.txt", None, body("hello"))
            .await
            .map(StoreOutcome::into_node)
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
            .upload(fx.bob, Some(folder.id), "x.txt", None, body("hi"))
            .await;
        assert!(matches!(result, Err(FilesError::NotFound)));
    }

    async fn put(fx: &Fixture, parent: Option<NodeId>, name: &str, text: &'static str) -> Node {
        fx.files
            .upload(fx.alice, parent, name, None, body(text))
            .await
            .map(StoreOutcome::into_node)
            .expect("upload")
    }

    async fn move_to(fx: &Fixture, id: NodeId, parent: Option<NodeId>) -> MoveOutcome {
        let update = NodeUpdate {
            parent_id: Some(parent),
            ..NodeUpdate::default()
        };
        fx.files.update(fx.alice, id, update).await.expect("move")
    }

    async fn content_of(fx: &Fixture, id: NodeId) -> String {
        let (_, mut file) = fx.files.download(fx.alice, id).await.expect("download");
        let mut content = String::new();
        file.read_to_string(&mut content).await.expect("read");
        content
    }

    async fn version_content(fx: &Fixture, id: NodeId, version: i32) -> String {
        let (_, _, mut file) = fx
            .files
            .download_version(fx.alice, id, version)
            .await
            .expect("version");
        let mut content = String::new();
        file.read_to_string(&mut content).await.expect("read");
        content
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn uploading_identical_content_changes_nothing(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let first = put(&fx, None, "a.txt", "same").await;
        let second = fx
            .files
            .upload(fx.alice, None, "a.txt", None, body("same"))
            .await
            .expect("upload");
        assert!(matches!(second, StoreOutcome::Unchanged(ref node) if node.id == first.id));
        assert_eq!(second.into_node().version, 1);
        assert!(
            fx.files
                .versions(fx.alice, first.id)
                .await
                .expect("versions")
                .is_empty()
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn uploading_different_content_keeps_the_old_one_as_a_version(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let first = put(&fx, None, "a.txt", "one").await;
        let second = put(&fx, None, "a.txt", "two").await;
        assert_eq!((second.id, second.version), (first.id, 2));
        assert_eq!(content_of(&fx, first.id).await, "two");
        assert_eq!(version_content(&fx, first.id, 1).await, "one");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn a_file_and_a_folder_can_share_a_name(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        put(&fx, None, "same", "x").await;
        assert!(fx.files.create_folder(fx.alice, None, "same").await.is_ok());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn creating_a_folder_with_a_taken_name_conflicts(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        fx.files
            .create_folder(fx.alice, None, "a")
            .await
            .expect("a");
        let result = fx.files.create_folder(fx.alice, None, "a").await;
        assert!(matches!(result, Err(FilesError::Conflict)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn folders_are_stored_with_the_directory_mime(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let folder = fx
            .files
            .create_folder(fx.alice, None, "a")
            .await
            .expect("a");
        assert_eq!(folder.mime, FOLDER_MIME);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn renaming_onto_a_taken_name_conflicts(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        put(&fx, None, "a.txt", "1").await;
        let other = put(&fx, None, "b.txt", "2").await;
        let update = NodeUpdate {
            name: Some("a.txt".to_owned()),
            ..NodeUpdate::default()
        };
        let result = fx.files.update(fx.alice, other.id, update).await;
        assert!(matches!(result, Err(FilesError::Conflict)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn moving_an_identical_file_drops_the_source_and_merges_dates(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let folder = fx
            .files
            .create_folder(fx.alice, None, "dir")
            .await
            .expect("dir");
        let dest = put(&fx, Some(folder.id), "a.txt", "same").await;
        let source = put(&fx, None, "a.txt", "same").await;
        let outcome = move_to(&fx, source.id, Some(folder.id)).await;
        assert_eq!(outcome.node.id, dest.id);
        assert_eq!(outcome.node.version, 1);
        assert_eq!(
            outcome.node.created_at,
            dest.created_at.min(source.created_at)
        );
        assert_eq!(
            outcome.node.updated_at,
            dest.updated_at.max(source.updated_at)
        );
        assert_eq!(outcome.deleted.len(), 1);
        assert!(matches!(
            fx.files.get(fx.alice, source.id).await,
            Err(FilesError::NotFound)
        ));
        assert!(fx.files.store.open_blob(fx.alice, source.id).await.is_err());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn moving_a_different_file_versions_the_destination_and_appends_history(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let folder = fx
            .files
            .create_folder(fx.alice, None, "dir")
            .await
            .expect("dir");
        let dest = put(&fx, Some(folder.id), "a.txt", "dest1").await;
        put(&fx, Some(folder.id), "a.txt", "dest2").await;
        let source = put(&fx, None, "a.txt", "src1").await;
        put(&fx, None, "a.txt", "src2").await;
        let outcome = move_to(&fx, source.id, Some(folder.id)).await;
        assert_eq!(outcome.node.id, dest.id);
        assert_eq!(outcome.node.version, 4);
        assert_eq!(content_of(&fx, dest.id).await, "src2");
        assert_eq!(version_content(&fx, dest.id, 1).await, "dest1");
        assert_eq!(version_content(&fx, dest.id, 2).await, "dest2");
        assert_eq!(version_content(&fx, dest.id, 3).await, "src1");
        assert_eq!(outcome.content_changed, [dest.id]);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn moving_a_folder_merges_into_the_same_named_destination(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let target = fx
            .files
            .create_folder(fx.alice, None, "target")
            .await
            .expect("target");
        let dest = fx
            .files
            .create_folder(fx.alice, Some(target.id), "docs")
            .await
            .expect("dest");
        let dest_sub = fx
            .files
            .create_folder(fx.alice, Some(dest.id), "sub")
            .await
            .expect("dest sub");
        put(&fx, Some(dest_sub.id), "x.txt", "old").await;
        let source = fx
            .files
            .create_folder(fx.alice, None, "docs")
            .await
            .expect("src");
        let source_sub = fx
            .files
            .create_folder(fx.alice, Some(source.id), "sub")
            .await
            .expect("src sub");
        let new_x = put(&fx, Some(source_sub.id), "x.txt", "new").await;
        let only = put(&fx, Some(source.id), "only.txt", "o").await;

        let outcome = move_to(&fx, source.id, Some(target.id)).await;

        assert_eq!(outcome.node.id, dest.id);
        let sub = fx.files.list(fx.alice, Some(dest.id)).await.expect("list");
        assert_eq!(
            sub.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
            ["only.txt", "sub"]
        );
        let files = fx
            .files
            .list(fx.alice, Some(dest_sub.id))
            .await
            .expect("list");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].version, 2);
        assert_eq!(content_of(&fx, files[0].id).await, "new");
        assert_eq!(version_content(&fx, files[0].id, 1).await, "old");
        assert_eq!(content_of(&fx, only.id).await, "o");
        for gone in [source.id, source_sub.id, new_x.id] {
            assert!(matches!(
                fx.files.get(fx.alice, gone).await,
                Err(FilesError::NotFound)
            ));
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn deleting_a_file_removes_its_version_blobs(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let file = put(&fx, None, "a.txt", "one").await;
        put(&fx, None, "a.txt", "two").await;
        let version = fx
            .files
            .versions(fx.alice, file.id)
            .await
            .expect("versions");
        assert_eq!(version.len(), 1);
        let version_id = fx
            .files
            .db
            .get_version(fx.alice, file.id, 1)
            .await
            .expect("v")
            .id;
        fx.files.delete(fx.alice, file.id).await.expect("delete");
        assert!(
            fx.files
                .store
                .open_version(fx.alice, version_id)
                .await
                .is_err()
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn deleting_a_version_keeps_the_current_content(pool: PgPool) {
        let fx = fixture(pool, 1024).await;
        let file = put(&fx, None, "a.txt", "one").await;
        put(&fx, None, "a.txt", "two").await;
        fx.files
            .delete_version(fx.alice, file.id, 1)
            .await
            .expect("delete version");
        assert!(matches!(
            fx.files.download_version(fx.alice, file.id, 1).await,
            Err(FilesError::NotFound)
        ));
        assert_eq!(content_of(&fx, file.id).await, "two");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn oversized_upload_is_rejected(pool: PgPool) {
        let fx = fixture(pool, 3).await;
        let result = fx
            .files
            .upload(fx.alice, None, "a.txt", None, body("hello"))
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
            .upload(fx.alice, Some(folder.id), "a.txt", None, body("hi"))
            .await
            .map(StoreOutcome::into_node)
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
            .upload(fx.alice, None, "a.txt", None, body("hi"))
            .await
            .map(StoreOutcome::into_node)
            .expect("upload");
        let updated = fx
            .files
            .replace_content(fx.alice, file.id, None, body("longer"))
            .await
            .map(StoreOutcome::into_node)
            .expect("replace");
        assert_eq!(updated.size, 6);
        assert_ne!(updated.sha256, file.sha256);
    }

    #[test]
    fn declared_type_wins_over_the_file_name() {
        assert_eq!(resolve_mime(Some("image/png"), "note.txt"), "image/png");
    }

    #[test]
    fn declared_parameters_are_dropped() {
        assert_eq!(
            resolve_mime(Some("text/plain; charset=utf-8"), "a"),
            "text/plain"
        );
    }

    #[test]
    fn octet_stream_falls_back_to_the_file_name() {
        assert_eq!(
            resolve_mime(Some("application/octet-stream"), "note.txt"),
            "text/plain"
        );
    }

    #[test]
    fn missing_type_is_guessed_from_the_file_name() {
        assert_eq!(resolve_mime(None, "note.txt"), "text/plain");
    }

    #[test]
    fn unknown_extension_stays_octet_stream() {
        assert_eq!(
            resolve_mime(Some("application/octet-stream"), "blob.zzzz"),
            FALLBACK_MIME
        );
    }

    #[test]
    fn invalid_declared_type_is_ignored() {
        assert_eq!(resolve_mime(Some("not a mime"), "note.txt"), "text/plain");
    }
}
