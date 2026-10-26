use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Db, DbError, Node, NodeId, NodeKind, UserId};

/// MIME type stored for folders.
pub const FOLDER_MIME: &str = "inode/directory";

macro_rules! node_columns {
    () => {
        "id, owner_id, parent_id, kind, name, size, sha256, mime, version, created_at, updated_at"
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
    pub mime: String,
}

/// A blob operation the storage layer must perform after a database change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlobOp {
    /// Keep the current content of `node` as the stored content of a version.
    Archive { node: NodeId, version_id: Uuid },
    /// Make the content of node `from` the content of node `to`.
    Promote { from: NodeId, to: NodeId },
    /// Drop the content of a node that no longer exists.
    Remove { node: NodeId },
}

/// What storing file content did to the tree.
#[derive(Clone, Debug)]
pub enum StoreOutcome {
    Created(Node),
    /// The content matched the current one, nothing changed.
    Unchanged(Node),
    /// The previous content was kept as version `archived`.
    Versioned {
        node: Node,
        archived: Uuid,
    },
}

impl StoreOutcome {
    /// The file the content was stored in.
    pub fn into_node(self) -> Node {
        match self {
            Self::Created(node) | Self::Unchanged(node) | Self::Versioned { node, .. } => node,
        }
    }
}

/// The effects of moving a node, including merges into same-named nodes.
#[derive(Clone, Debug)]
pub struct MoveOutcome {
    /// The node at the final location; it is the destination when a merge happened.
    pub node: Node,
    pub updated: Vec<Node>,
    pub deleted: Vec<Node>,
    pub content_changed: Vec<NodeId>,
    pub blob_ops: Vec<BlobOp>,
}

/// Ids whose stored content must be removed after a deletion.
#[derive(Clone, Debug, Default)]
pub struct DeletedNodes {
    pub files: Vec<NodeId>,
    pub versions: Vec<Uuid>,
}

struct Placement {
    node: Node,
    parent: Option<NodeId>,
    name: String,
    touch: bool,
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

    /// Lists the ancestors of a node from the root down to its parent.
    pub async fn list_ancestors(&self, owner: UserId, id: NodeId) -> Result<Vec<Node>, DbError> {
        let node = self.get_node(owner, id).await?;
        let ancestors = sqlx::query_as::<_, Node>(concat!(
            "WITH RECURSIVE chain AS ( \
                 SELECT ",
            node_columns!(),
            ", 1 AS depth FROM nodes WHERE id = $1 AND owner_id = $2 \
                 UNION ALL \
                 SELECT ",
            "n.id, n.owner_id, n.parent_id, n.kind, n.name, n.size, n.sha256, n.mime, \
                 n.version, n.created_at, n.updated_at, c.depth + 1 \
                 FROM nodes n JOIN chain c ON n.id = c.parent_id AND n.owner_id = $2 \
             ) SELECT ",
            node_columns!(),
            " FROM chain ORDER BY depth DESC"
        ))
        .bind(node.parent_id)
        .bind(owner)
        .fetch_all(self.pool())
        .await?;
        Ok(ancestors)
    }

    /// Renames and/or moves a node.
    ///
    /// A rename onto a taken name is a `Conflict`. Moving to another folder merges into a
    /// same-named node there: files keep the destination, gain a version when the content
    /// differs and combine their dates; folders are merged recursively.
    /// Returns `Cycle` when moving a folder into itself or a descendant.
    pub async fn update_node(
        &self,
        owner: UserId,
        id: NodeId,
        update: NodeUpdate,
    ) -> Result<MoveOutcome, DbError> {
        let mut tx = self.pool().begin().await?;
        let current = lock_node(&mut tx, owner, id).await?;
        let new_parent = update.parent_id.unwrap_or(current.parent_id);
        let name = update.name.unwrap_or_else(|| current.name.clone());
        let mut outcome = MoveOutcome {
            node: current.clone(),
            updated: Vec::new(),
            deleted: Vec::new(),
            content_changed: Vec::new(),
            blob_ops: Vec::new(),
        };
        if new_parent == current.parent_id {
            let node = relocate(
                &mut tx,
                &Placement {
                    node: current,
                    parent: new_parent,
                    name,
                    touch: true,
                },
            )
            .await?;
            outcome.updated.push(node.clone());
            outcome.node = node;
        } else {
            require_folder(&mut tx, owner, new_parent).await?;
            if let Some(parent) = new_parent {
                ensure_not_descendant(&mut tx, owner, id, parent).await?;
            }
            let first = Placement {
                node: current,
                parent: new_parent,
                name,
                touch: true,
            };
            place(&mut tx, owner, first, &mut outcome).await?;
        }
        tx.commit().await?;
        Ok(outcome)
    }

    /// Stores content for the file `name` under `parent`.
    ///
    /// Creates the file, keeps the existing one when the content is identical, or archives
    /// the previous content as a version.
    pub async fn store_file(
        &self,
        owner: UserId,
        parent: Option<NodeId>,
        name: &str,
        size: i64,
        sha256: Vec<u8>,
        mime: &str,
    ) -> Result<StoreOutcome, DbError> {
        let mut tx = self.pool().begin().await?;
        require_folder(&mut tx, owner, parent).await?;
        let existing = find_sibling(&mut tx, owner, parent, NodeKind::File, name, None).await?;
        let outcome = match existing {
            Some(node) => apply_content(&mut tx, node, size, sha256, mime).await?,
            None => {
                let node = sqlx::query_as::<_, Node>(concat!(
                    "INSERT INTO nodes (id, owner_id, parent_id, kind, name, size, sha256, mime) \
                     VALUES ($1, $2, $3, 'file', $4, $5, $6, $7) RETURNING ",
                    node_columns!(),
                    ""
                ))
                .bind(NodeId::new())
                .bind(owner)
                .bind(parent)
                .bind(name)
                .bind(size)
                .bind(sha256)
                .bind(mime)
                .fetch_one(&mut *tx)
                .await?;
                StoreOutcome::Created(node)
            }
        };
        tx.commit().await?;
        Ok(outcome)
    }

    /// Replaces the content of an existing file, archiving the previous content as a version
    /// unless it is identical.
    pub async fn replace_file_content(
        &self,
        owner: UserId,
        id: NodeId,
        size: i64,
        sha256: Vec<u8>,
        mime: &str,
    ) -> Result<StoreOutcome, DbError> {
        let mut tx = self.pool().begin().await?;
        let node = lock_node(&mut tx, owner, id).await?;
        if node.kind != NodeKind::File {
            return Err(DbError::NotFound);
        }
        let outcome = apply_content(&mut tx, node, size, sha256, mime).await?;
        tx.commit().await?;
        Ok(outcome)
    }

    /// Deletes a node and its descendants; returns the ids whose stored content is now unused.
    pub async fn delete_node(&self, owner: UserId, id: NodeId) -> Result<DeletedNodes, DbError> {
        let mut tx = self.pool().begin().await?;
        let files = sqlx::query_scalar::<_, NodeId>(
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
        let versions = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM file_versions WHERE owner_id = $1 AND node_id = ANY($2)",
        )
        .bind(owner)
        .bind(files.iter().map(|file| file.0).collect::<Vec<_>>())
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
        Ok(DeletedNodes { files, versions })
    }

    async fn require_folder(&self, owner: UserId, id: Option<NodeId>) -> Result<(), DbError> {
        let mut conn = self.pool().acquire().await?;
        require_folder(&mut conn, owner, id).await
    }
}

async fn lock_node(conn: &mut PgConnection, owner: UserId, id: NodeId) -> Result<Node, DbError> {
    let node = sqlx::query_as::<_, Node>(concat!(
        "SELECT ",
        node_columns!(),
        " FROM nodes WHERE id = $1 AND owner_id = $2 FOR UPDATE"
    ))
    .bind(id)
    .bind(owner)
    .fetch_one(conn)
    .await?;
    Ok(node)
}

async fn find_sibling(
    conn: &mut PgConnection,
    owner: UserId,
    parent: Option<NodeId>,
    kind: NodeKind,
    name: &str,
    exclude: Option<NodeId>,
) -> Result<Option<Node>, DbError> {
    let node = sqlx::query_as::<_, Node>(concat!(
        "SELECT ",
        node_columns!(),
        " FROM nodes \
         WHERE owner_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND kind = $3 AND name = $4 \
         AND id IS DISTINCT FROM $5 FOR UPDATE"
    ))
    .bind(owner)
    .bind(parent)
    .bind(kind)
    .bind(name)
    .bind(exclude)
    .fetch_optional(conn)
    .await?;
    Ok(node)
}

async fn require_folder(
    conn: &mut PgConnection,
    owner: UserId,
    id: Option<NodeId>,
) -> Result<(), DbError> {
    match id {
        None => Ok(()),
        Some(id) => {
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM nodes \
                 WHERE id = $1 AND owner_id = $2 AND kind = 'folder')",
            )
            .bind(id)
            .bind(owner)
            .fetch_one(conn)
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
    conn: &mut PgConnection,
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
    .fetch_one(conn)
    .await?;
    if is_descendant {
        Err(DbError::Cycle)
    } else {
        Ok(())
    }
}

async fn relocate(conn: &mut PgConnection, placement: &Placement) -> Result<Node, DbError> {
    let node = sqlx::query_as::<_, Node>(concat!(
        "UPDATE nodes SET name = $2, parent_id = $3, \
         updated_at = CASE WHEN $4 THEN now() ELSE updated_at END \
         WHERE id = $1 RETURNING ",
        node_columns!(),
        ""
    ))
    .bind(placement.node.id)
    .bind(&placement.name)
    .bind(placement.parent)
    .bind(placement.touch)
    .fetch_one(conn)
    .await?;
    Ok(node)
}

/// Moves nodes into place, merging with same-named nodes at the destination.
async fn place(
    conn: &mut PgConnection,
    owner: UserId,
    first: Placement,
    outcome: &mut MoveOutcome,
) -> Result<(), DbError> {
    let mut pending = vec![first];
    let mut emptied = Vec::new();
    let mut is_first = true;
    while let Some(placement) = pending.pop() {
        let target = find_sibling(
            conn,
            owner,
            placement.parent,
            placement.node.kind,
            &placement.name,
            Some(placement.node.id),
        )
        .await?;
        let placed = match target {
            None => relocate(conn, &placement).await?,
            Some(dest) if dest.kind == NodeKind::File => {
                merge_file(conn, &placement.node, dest, outcome).await?
            }
            Some(dest) => {
                let children = sqlx::query_as::<_, Node>(concat!(
                    "SELECT ",
                    node_columns!(),
                    " FROM nodes WHERE owner_id = $1 AND parent_id = $2 FOR UPDATE"
                ))
                .bind(owner)
                .bind(placement.node.id)
                .fetch_all(&mut *conn)
                .await?;
                pending.extend(children.into_iter().map(|node| Placement {
                    name: node.name.clone(),
                    node,
                    parent: Some(dest.id),
                    touch: false,
                }));
                emptied.push(placement.node.clone());
                merge_folder(conn, &placement.node, dest).await?
            }
        };
        if is_first || placed.kind == NodeKind::File {
            outcome.updated.push(placed.clone());
        }
        if is_first {
            outcome.node = placed;
            is_first = false;
        }
    }
    for folder in emptied.into_iter().rev() {
        sqlx::query("DELETE FROM nodes WHERE id = $1")
            .bind(folder.id)
            .execute(&mut *conn)
            .await?;
        outcome.deleted.push(folder);
    }
    Ok(())
}

async fn merge_folder(conn: &mut PgConnection, source: &Node, dest: Node) -> Result<Node, DbError> {
    let node = sqlx::query_as::<_, Node>(concat!(
        "UPDATE nodes SET created_at = $2, updated_at = $3 WHERE id = $1 RETURNING ",
        node_columns!(),
        ""
    ))
    .bind(dest.id)
    .bind(dest.created_at.min(source.created_at))
    .bind(dest.updated_at.max(source.updated_at))
    .fetch_one(conn)
    .await?;
    Ok(node)
}

async fn merge_file(
    conn: &mut PgConnection,
    source: &Node,
    dest: Node,
    outcome: &mut MoveOutcome,
) -> Result<Node, DbError> {
    let identical = same_content(&dest, source.size, source.sha256.as_deref());
    let (offset, version) = if identical {
        outcome.blob_ops.push(BlobOp::Remove { node: source.id });
        (dest.version - 1, dest.version + source.version - 1)
    } else {
        let archived = archive_current(conn, &dest).await?;
        outcome.blob_ops.push(BlobOp::Archive {
            node: dest.id,
            version_id: archived,
        });
        outcome.blob_ops.push(BlobOp::Promote {
            from: source.id,
            to: dest.id,
        });
        outcome.content_changed.push(dest.id);
        (dest.version, dest.version + source.version)
    };
    sqlx::query("UPDATE file_versions SET node_id = $2, version = version + $3 WHERE node_id = $1")
        .bind(source.id)
        .bind(dest.id)
        .bind(offset)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM nodes WHERE id = $1")
        .bind(source.id)
        .execute(&mut *conn)
        .await?;
    outcome.deleted.push(source.clone());
    let (size, sha256, mime) = if identical {
        (dest.size, dest.sha256.clone(), dest.mime.clone())
    } else {
        (source.size, source.sha256.clone(), source.mime.clone())
    };
    let node = sqlx::query_as::<_, Node>(concat!(
        "UPDATE nodes SET size = $2, sha256 = $3, mime = $4, version = $5, \
         created_at = $6, updated_at = $7 WHERE id = $1 RETURNING ",
        node_columns!(),
        ""
    ))
    .bind(dest.id)
    .bind(size)
    .bind(sha256)
    .bind(mime)
    .bind(version)
    .bind(dest.created_at.min(source.created_at))
    .bind(dest.updated_at.max(source.updated_at))
    .fetch_one(conn)
    .await?;
    Ok(node)
}

fn same_content(node: &Node, size: i64, sha256: Option<&[u8]>) -> bool {
    node.size == size && node.sha256.as_deref() == sha256
}

async fn archive_current(conn: &mut PgConnection, node: &Node) -> Result<Uuid, DbError> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO file_versions \
         (id, node_id, owner_id, version, size, sha256, mime, modified_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(node.id)
    .bind(node.owner_id)
    .bind(node.version)
    .bind(node.size)
    .bind(node.sha256.clone().unwrap_or_default())
    .bind(&node.mime)
    .bind(node.updated_at)
    .execute(conn)
    .await?;
    Ok(id)
}

async fn apply_content(
    conn: &mut PgConnection,
    node: Node,
    size: i64,
    sha256: Vec<u8>,
    mime: &str,
) -> Result<StoreOutcome, DbError> {
    if same_content(&node, size, Some(&sha256)) {
        return Ok(StoreOutcome::Unchanged(node));
    }
    let archived = archive_current(conn, &node).await?;
    let updated = sqlx::query_as::<_, Node>(concat!(
        "UPDATE nodes SET size = $2, sha256 = $3, mime = $4, version = version + 1, \
         updated_at = now() WHERE id = $1 RETURNING ",
        node_columns!(),
        ""
    ))
    .bind(node.id)
    .bind(size)
    .bind(sha256)
    .bind(mime)
    .fetch_one(conn)
    .await?;
    Ok(StoreOutcome::Versioned {
        node: updated,
        archived,
    })
}
