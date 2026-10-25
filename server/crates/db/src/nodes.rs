use crate::{Db, DbError, Node, NodeId, NodeKind, UserId};

macro_rules! node_columns {
    () => {
        "id, owner_id, parent_id, kind, name, size, sha256, mime, created_at, updated_at"
    };
}

/// Values for a node to insert.
#[derive(Clone, Debug)]
pub struct NewNode {
    pub id: NodeId,
    pub parent_id: Option<NodeId>,
    pub kind: NodeKind,
    pub name: String,
    pub size: i64,
    pub sha256: Option<Vec<u8>>,
    pub mime: Option<String>,
}

/// Changes to apply to a node; `None` leaves a field untouched.
///
/// `parent_id: Some(None)` moves the node to the root.
#[derive(Clone, Debug, Default)]
pub struct NodeUpdate {
    pub name: Option<String>,
    pub parent_id: Option<Option<NodeId>>,
}

impl Db {
    /// Inserts a node under a folder owned by `owner` (or at the root).
    pub async fn create_node(&self, owner: UserId, new: NewNode) -> Result<Node, DbError> {
        self.require_folder(owner, new.parent_id).await?;
        let node = sqlx::query_as::<_, Node>(concat!(
            "INSERT INTO nodes (id, owner_id, parent_id, kind, name, size, sha256, mime) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING ",
            node_columns!(),
            ""
        ))
        .bind(new.id)
        .bind(owner)
        .bind(new.parent_id)
        .bind(new.kind)
        .bind(new.name)
        .bind(new.size)
        .bind(new.sha256)
        .bind(new.mime)
        .fetch_one(self.pool())
        .await?;
        Ok(node)
    }

    /// Fetches a node owned by `owner`; `NotFound` for missing or foreign nodes.
    pub async fn get_node(&self, owner: UserId, id: NodeId) -> Result<Node, DbError> {
        let node = sqlx::query_as::<_, Node>(concat!(
            "SELECT ",
            node_columns!(),
            " FROM nodes WHERE id = $1 AND owner_id = $2"
        ))
        .bind(id)
        .bind(owner)
        .fetch_one(self.pool())
        .await?;
        Ok(node)
    }

    /// Lists the children of a folder (or the root) ordered folders first, then by name.
    pub async fn list_children(
        &self,
        owner: UserId,
        parent_id: Option<NodeId>,
    ) -> Result<Vec<Node>, DbError> {
        self.require_folder(owner, parent_id).await?;
        let nodes = sqlx::query_as::<_, Node>(concat!(
            "SELECT ",
            node_columns!(),
            " FROM nodes \
             WHERE owner_id = $1 AND parent_id IS NOT DISTINCT FROM $2 \
             ORDER BY kind, name"
        ))
        .bind(owner)
        .bind(parent_id)
        .fetch_all(self.pool())
        .await?;
        Ok(nodes)
    }

    /// Renames and/or moves a node.
    ///
    /// Returns `Cycle` when moving a folder into itself or a descendant.
    pub async fn update_node(
        &self,
        owner: UserId,
        id: NodeId,
        update: NodeUpdate,
    ) -> Result<Node, DbError> {
        let current = self.get_node(owner, id).await?;
        let new_parent = update.parent_id.unwrap_or(current.parent_id);
        if new_parent != current.parent_id {
            self.require_folder(owner, new_parent).await?;
            if let Some(parent) = new_parent {
                self.ensure_not_descendant(owner, id, parent).await?;
            }
        }
        let name = update.name.unwrap_or(current.name);
        let node = sqlx::query_as::<_, Node>(concat!(
            "UPDATE nodes SET name = $3, parent_id = $4, updated_at = now() \
             WHERE id = $1 AND owner_id = $2 RETURNING ",
            node_columns!(),
            ""
        ))
        .bind(id)
        .bind(owner)
        .bind(name)
        .bind(new_parent)
        .fetch_one(self.pool())
        .await?;
        Ok(node)
    }

    /// Updates size, hash and mime type of an existing file.
    pub async fn replace_file_content(
        &self,
        owner: UserId,
        id: NodeId,
        size: i64,
        sha256: Vec<u8>,
        mime: Option<String>,
    ) -> Result<Node, DbError> {
        let node = sqlx::query_as::<_, Node>(concat!(
            "UPDATE nodes SET size = $3, sha256 = $4, mime = $5, updated_at = now() \
             WHERE id = $1 AND owner_id = $2 AND kind = 'file' RETURNING ",
            node_columns!(),
            ""
        ))
        .bind(id)
        .bind(owner)
        .bind(size)
        .bind(sha256)
        .bind(mime)
        .fetch_one(self.pool())
        .await?;
        Ok(node)
    }

    /// Deletes a node and its descendants; returns the ids of the deleted files.
    pub async fn delete_node(&self, owner: UserId, id: NodeId) -> Result<Vec<NodeId>, DbError> {
        let mut tx = self.pool().begin().await?;
        let file_ids = sqlx::query_scalar::<_, NodeId>(
            "WITH RECURSIVE subtree AS ( \
                 SELECT id, kind FROM nodes WHERE id = $1 AND owner_id = $2 \
                 UNION ALL \
                 SELECT n.id, n.kind FROM nodes n JOIN subtree s ON n.parent_id = s.id \
             ) SELECT id FROM subtree WHERE kind = 'file'",
        )
        .bind(id)
        .bind(owner)
        .fetch_all(&mut *tx)
        .await?;
        let deleted = sqlx::query("DELETE FROM nodes WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner)
            .execute(&mut *tx)
            .await?;
        if deleted.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        tx.commit().await?;
        Ok(file_ids)
    }

    async fn require_folder(&self, owner: UserId, id: Option<NodeId>) -> Result<(), DbError> {
        match id {
            None => Ok(()),
            Some(id) => {
                let exists = sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS (SELECT 1 FROM nodes \
                     WHERE id = $1 AND owner_id = $2 AND kind = 'folder')",
                )
                .bind(id)
                .bind(owner)
                .fetch_one(self.pool())
                .await?;
                if exists {
                    Ok(())
                } else {
                    Err(DbError::NotFound)
                }
            }
        }
    }

    async fn ensure_not_descendant(
        &self,
        owner: UserId,
        ancestor: NodeId,
        candidate: NodeId,
    ) -> Result<(), DbError> {
        let is_descendant = sqlx::query_scalar::<_, bool>(
            "WITH RECURSIVE subtree AS ( \
                 SELECT id FROM nodes WHERE id = $1 AND owner_id = $2 \
                 UNION ALL \
                 SELECT n.id FROM nodes n JOIN subtree s ON n.parent_id = s.id \
             ) SELECT EXISTS (SELECT 1 FROM subtree WHERE id = $3)",
        )
        .bind(ancestor)
        .bind(owner)
        .bind(candidate)
        .fetch_one(self.pool())
        .await?;
        if is_descendant {
            Err(DbError::Cycle)
        } else {
            Ok(())
        }
    }
}
