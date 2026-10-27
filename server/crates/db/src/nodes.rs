use std::collections::{HashMap, HashSet, VecDeque, hash_map::Entry};

use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Db, DbError, Node, NodeId, NodeKind, OnConflict, UserId};

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

/// A file that clashes with a live file at its destination.
#[derive(Clone, Debug)]
pub struct MoveConflict {
    pub source: Node,
    /// Path of the file relative to what is moved, e.g. `Photos/2024/a.jpg`.
    pub path: String,
    /// The file in the way; `None` when two selected files share a name.
    pub existing: Option<Node>,
}

/// The effects of moving several nodes at once (also used to restore binned files).
#[derive(Clone, Debug, Default)]
pub struct MoveBatch {
    /// The destination folder; `None` for the root.
    pub target: Option<Node>,
    /// The selected nodes at their final location; a merge destination when they were merged.
    pub moved: Vec<Node>,
    /// Files left where they were because of [`OnConflict::Skip`].
    pub skipped: Vec<Node>,
    /// The clashes found; only filled for [`OnConflict::Abort`], in which case nothing changed.
    pub conflicts: Vec<MoveConflict>,
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

#[derive(Default)]
struct Effects {
    updated: Vec<Node>,
    deleted: Vec<Node>,
    content_changed: Vec<NodeId>,
    blob_ops: Vec<BlobOp>,
    skipped: Vec<Node>,
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
            " FROM nodes WHERE id = $1 AND owner_id = $2 AND ",
            visible!("nodes")
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
             WHERE owner_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND ",
            visible!("nodes"),
            " ORDER BY kind, name"
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
        let mut effects = Effects::default();
        let node = if new_parent == current.parent_id {
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
            effects.updated.push(node.clone());
            node
        } else {
            require_folder(&mut tx, owner, new_parent).await?;
            if let Some(parent) = new_parent {
                ensure_not_descendant(&mut tx, owner, id, parent).await?;
            }
            let first = Placement {
                node: current.clone(),
                parent: new_parent,
                name,
                touch: true,
            };
            place(&mut tx, owner, first, &HashSet::new(), &mut effects)
                .await?
                .unwrap_or(current)
        };
        tx.commit().await?;
        Ok(MoveOutcome {
            node,
            updated: effects.updated,
            deleted: effects.deleted,
            content_changed: effects.content_changed,
            blob_ops: effects.blob_ops,
        })
    }

    /// Moves several nodes into `parent` (the root when `None`).
    ///
    /// Folders merge into same-named folders. A file clashes when a live file of the same name
    /// is in its way, or when two selected files share a name; `on_conflict` decides what
    /// happens then. Selected nodes inside another selected folder move along with it.
    /// Returns `Cycle` when moving a folder into itself or a descendant.
    pub async fn move_nodes(
        &self,
        owner: UserId,
        ids: &[NodeId],
        parent: Option<NodeId>,
        on_conflict: OnConflict,
    ) -> Result<MoveBatch, DbError> {
        let mut tx = self.pool().begin().await?;
        let sources = lock_selection(&mut tx, owner, ids).await?;
        let target = match parent {
            Some(id) => Some(lock_folder(&mut tx, owner, id).await?),
            None => None,
        };
        let batch = move_into(&mut tx, owner, sources, target, on_conflict).await?;
        tx.commit().await?;
        Ok(batch)
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

pub(crate) async fn lock_node(
    conn: &mut PgConnection,
    owner: UserId,
    id: NodeId,
) -> Result<Node, DbError> {
    let node = sqlx::query_as::<_, Node>(concat!(
        "SELECT ",
        node_columns!(),
        " FROM nodes WHERE id = $1 AND owner_id = $2 AND ",
        visible!("nodes"),
        " FOR UPDATE"
    ))
    .bind(id)
    .bind(owner)
    .fetch_one(conn)
    .await?;
    Ok(node)
}

async fn lock_folder(conn: &mut PgConnection, owner: UserId, id: NodeId) -> Result<Node, DbError> {
    let node = lock_node(conn, owner, id).await?;
    if node.kind == NodeKind::Folder {
        Ok(node)
    } else {
        Err(DbError::NotFound)
    }
}

/// Locks the nodes of a selection in the order given, without repeats and without nodes
/// that sit inside another selected node.
async fn lock_selection(
    conn: &mut PgConnection,
    owner: UserId,
    ids: &[NodeId],
) -> Result<Vec<Node>, DbError> {
    let mut seen = HashSet::new();
    let ids = ids
        .iter()
        .filter(|id| seen.insert(**id))
        .collect::<Vec<_>>();
    let raw = ids.iter().map(|id| id.0).collect::<Vec<Uuid>>();
    let mut found = sqlx::query_as::<_, Node>(concat!(
        "SELECT ",
        node_columns!(),
        " FROM nodes WHERE id = ANY($1) AND owner_id = $2 AND ",
        visible!("nodes"),
        " ORDER BY id FOR UPDATE"
    ))
    .bind(&raw)
    .bind(owner)
    .fetch_all(&mut *conn)
    .await?;
    if found.len() != ids.len() {
        return Err(DbError::NotFound);
    }
    let nested = sqlx::query_scalar::<_, NodeId>(
        "WITH RECURSIVE chain AS ( \
             SELECT id, parent_id FROM nodes WHERE id = ANY($1) AND owner_id = $2 \
             UNION ALL \
             SELECT c.id, n.parent_id FROM chain c JOIN nodes n ON n.id = c.parent_id \
         ) SELECT DISTINCT id FROM chain WHERE parent_id = ANY($1)",
    )
    .bind(&raw)
    .bind(owner)
    .fetch_all(conn)
    .await?;
    found.retain(|node| !nested.contains(&node.id));
    let order = ids
        .iter()
        .enumerate()
        .map(|(position, id)| (**id, position))
        .collect::<HashMap<_, _>>();
    found.sort_by_key(|node| order[&node.id]);
    Ok(found)
}

async fn lock_children(
    conn: &mut PgConnection,
    owner: UserId,
    parent: NodeId,
) -> Result<Vec<Node>, DbError> {
    let nodes = sqlx::query_as::<_, Node>(concat!(
        "SELECT ",
        node_columns!(),
        " FROM nodes WHERE owner_id = $1 AND parent_id = $2 ORDER BY kind, name FOR UPDATE"
    ))
    .bind(owner)
    .bind(parent)
    .fetch_all(conn)
    .await?;
    Ok(nodes)
}

pub(crate) async fn find_sibling(
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
         AND id IS DISTINCT FROM $5 AND ",
        visible!("nodes"),
        " FOR UPDATE"
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
            let exists = sqlx::query_scalar::<_, bool>(concat!(
                "SELECT EXISTS (SELECT 1 FROM nodes \
                 WHERE id = $1 AND owner_id = $2 AND kind = 'folder' AND ",
                visible!("nodes"),
                ")"
            ))
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

pub(crate) async fn ensure_not_descendant(
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
        "UPDATE nodes SET name = $2, parent_id = $3, trashed_at = NULL, \
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

/// Moves several selected nodes into `target`, applying `on_conflict` to clashing files.
///
/// `sources` and `target` must be locked. Binned files can be moved too: they come out live.
pub(crate) async fn move_into(
    conn: &mut PgConnection,
    owner: UserId,
    sources: Vec<Node>,
    target: Option<Node>,
    on_conflict: OnConflict,
) -> Result<MoveBatch, DbError> {
    let parent = target.as_ref().map(|folder| folder.id);
    let mut batch = MoveBatch {
        target,
        ..MoveBatch::default()
    };
    if let Some(parent) = parent {
        for folder in sources.iter().filter(|node| node.kind == NodeKind::Folder) {
            ensure_not_descendant(conn, owner, folder.id, parent).await?;
        }
    }
    let conflicts = plan_conflicts(conn, owner, &sources, parent).await?;
    if on_conflict == OnConflict::Abort && !conflicts.is_empty() {
        batch.conflicts = conflicts;
        return Ok(batch);
    }
    let skip = match on_conflict {
        OnConflict::Skip => conflicts
            .iter()
            .map(|conflict| conflict.source.id)
            .collect(),
        OnConflict::Abort | OnConflict::Override => HashSet::new(),
    };
    let mut effects = Effects::default();
    for source in sources {
        let placement = Placement {
            name: source.name.clone(),
            node: source,
            parent,
            touch: true,
        };
        if let Some(node) = place(conn, owner, placement, &skip, &mut effects).await? {
            batch.moved.push(node);
        }
    }
    batch.skipped = effects.skipped;
    batch.updated = effects.updated;
    batch.deleted = effects.deleted;
    batch.content_changed = effects.content_changed;
    batch.blob_ops = effects.blob_ops;
    Ok(batch)
}

/// Finds the files of `sources` that would clash at `parent` without changing anything.
///
/// Folders merge into same-named folders (also into one an earlier source moves there), so
/// only the files inside them can clash. Conflicts are ordered by path.
async fn plan_conflicts(
    conn: &mut PgConnection,
    owner: UserId,
    sources: &[Node],
    parent: Option<NodeId>,
) -> Result<Vec<MoveConflict>, DbError> {
    struct Claim {
        node: Node,
        path: String,
        reported: bool,
    }
    let mut claims = HashMap::<(Option<NodeId>, NodeKind, String), Claim>::new();
    let mut conflicts = Vec::new();
    let mut queue = sources
        .iter()
        .map(|node| (node.clone(), parent, node.name.clone()))
        .collect::<VecDeque<_>>();
    while let Some((node, dest, path)) = queue.pop_front() {
        let existing =
            find_sibling(conn, owner, dest, node.kind, &node.name, Some(node.id)).await?;
        let claim = Claim {
            node: node.clone(),
            path: path.clone(),
            reported: false,
        };
        // The folder whose children `node` merges into, if it merges.
        let merge_into = match (node.kind, existing) {
            (NodeKind::File, Some(existing)) => {
                conflicts.push(MoveConflict {
                    source: node,
                    path,
                    existing: Some(existing),
                });
                continue;
            }
            (NodeKind::Folder, Some(existing)) => existing.id,
            (_, None) => match claims.entry((dest, node.kind, node.name.clone())) {
                Entry::Vacant(slot) => {
                    slot.insert(claim);
                    continue;
                }
                Entry::Occupied(mut slot) if node.kind == NodeKind::File => {
                    let first = slot.get_mut();
                    if !first.reported {
                        first.reported = true;
                        conflicts.push(MoveConflict {
                            source: first.node.clone(),
                            path: first.path.clone(),
                            existing: None,
                        });
                    }
                    conflicts.push(MoveConflict {
                        source: node,
                        path,
                        existing: None,
                    });
                    continue;
                }
                Entry::Occupied(slot) => slot.get().node.id,
            },
        };
        for child in lock_children(conn, owner, node.id).await? {
            let child_path = format!("{path}/{}", child.name);
            queue.push_back((child, Some(merge_into), child_path));
        }
    }
    conflicts.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(conflicts)
}

/// Moves nodes into place, merging with same-named nodes at the destination.
///
/// Files in `skip` stay where they are, and so do the folders that still hold them.
/// Returns the final location of the first node, `None` when it was skipped.
async fn place(
    conn: &mut PgConnection,
    owner: UserId,
    first: Placement,
    skip: &HashSet<NodeId>,
    effects: &mut Effects,
) -> Result<Option<Node>, DbError> {
    let mut pending = vec![first];
    let mut emptied = Vec::new();
    let mut result = None;
    let mut is_first = true;
    while let Some(placement) = pending.pop() {
        let first_placement = is_first;
        is_first = false;
        if skip.contains(&placement.node.id) {
            effects.skipped.push(placement.node);
            continue;
        }
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
                merge_file(conn, &placement.node, dest, effects).await?
            }
            Some(dest) => {
                let children = lock_children(conn, owner, placement.node.id).await?;
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
        if first_placement || placed.kind == NodeKind::File {
            effects.updated.push(placed.clone());
        }
        if first_placement {
            result = Some(placed);
        }
    }
    for folder in emptied.into_iter().rev() {
        let removed = sqlx::query(
            "DELETE FROM nodes WHERE id = $1 \
             AND NOT EXISTS (SELECT 1 FROM nodes child WHERE child.parent_id = $1)",
        )
        .bind(folder.id)
        .execute(&mut *conn)
        .await?;
        if removed.rows_affected() > 0 {
            effects.deleted.push(folder);
        }
    }
    Ok(result)
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
    effects: &mut Effects,
) -> Result<Node, DbError> {
    let identical = same_content(&dest, source.size, source.sha256.as_deref());
    let (offset, version) = if identical {
        effects.blob_ops.push(BlobOp::Remove { node: source.id });
        (dest.version - 1, dest.version + source.version - 1)
    } else {
        let archived = archive_current(conn, &dest).await?;
        effects.blob_ops.push(BlobOp::Archive {
            node: dest.id,
            version_id: archived,
        });
        effects.blob_ops.push(BlobOp::Promote {
            from: source.id,
            to: dest.id,
        });
        effects.content_changed.push(dest.id);
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
    effects.deleted.push(source.clone());
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

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    async fn add(
        db: &Db,
        owner: UserId,
        parent: Option<NodeId>,
        kind: NodeKind,
        name: &str,
    ) -> Node {
        let new = NewNode {
            id: NodeId::new(),
            parent_id: parent,
            kind,
            name: name.to_owned(),
            size: 0,
            sha256: None,
            mime: "text/plain".to_owned(),
        };
        db.create_node(owner, new).await.expect("node")
    }

    async fn file(db: &Db, owner: UserId, parent: NodeId, name: &str) -> Node {
        add(db, owner, Some(parent), NodeKind::File, name).await
    }

    async fn folder(db: &Db, owner: UserId, parent: Option<NodeId>, name: &str) -> Node {
        add(db, owner, parent, NodeKind::Folder, name).await
    }

    struct Tree {
        alice: UserId,
        /// Destination with `Photos/2024/a.jpg` and `Photos/old.jpg`.
        target: Node,
        existing: Node,
        /// Source with `Photos/2024/a.jpg` (clashing), `Photos/2024/b.jpg` and `Photos/new.jpg`.
        source_photos: Node,
        clashing: Node,
        fresh: Node,
        nested_fresh: Node,
    }

    async fn tree(db: &Db) -> Tree {
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let target = folder(db, alice, None, "target").await;
        let photos = folder(db, alice, Some(target.id), "Photos").await;
        let year = folder(db, alice, Some(photos.id), "2024").await;
        let existing = file(db, alice, year.id, "a.jpg").await;
        file(db, alice, photos.id, "old.jpg").await;
        let source_photos = folder(db, alice, None, "Photos").await;
        let source_year = folder(db, alice, Some(source_photos.id), "2024").await;
        let clashing = file(db, alice, source_year.id, "a.jpg").await;
        let nested_fresh = file(db, alice, source_year.id, "b.jpg").await;
        let fresh = file(db, alice, source_photos.id, "new.jpg").await;
        Tree {
            alice,
            target,
            existing,
            source_photos,
            clashing,
            fresh,
            nested_fresh,
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn aborting_a_move_lists_nested_conflicts_and_changes_nothing(pool: PgPool) {
        let db = Db::from_pool(pool);
        let tree = tree(&db).await;

        let batch = db
            .move_nodes(
                tree.alice,
                &[tree.source_photos.id],
                Some(tree.target.id),
                OnConflict::Abort,
            )
            .await
            .expect("move");

        assert!(batch.moved.is_empty());
        assert_eq!(batch.conflicts.len(), 1);
        let conflict = &batch.conflicts[0];
        assert_eq!(conflict.path, "Photos/2024/a.jpg");
        assert_eq!(conflict.source.id, tree.clashing.id);
        assert_eq!(
            conflict.existing.as_ref().map(|node| node.id),
            Some(tree.existing.id)
        );
        assert_eq!(
            batch.target.as_ref().map(|node| node.id),
            Some(tree.target.id)
        );
        for id in [tree.source_photos.id, tree.clashing.id, tree.fresh.id] {
            assert!(db.get_node(tree.alice, id).await.is_ok());
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn moving_without_conflicts_merges_folders_even_when_aborting(pool: PgPool) {
        let db = Db::from_pool(pool);
        let tree = tree(&db).await;
        db.delete_node(tree.alice, tree.clashing.id)
            .await
            .expect("remove clash");

        let batch = db
            .move_nodes(
                tree.alice,
                &[tree.source_photos.id],
                Some(tree.target.id),
                OnConflict::Abort,
            )
            .await
            .expect("move");

        assert!(batch.conflicts.is_empty());
        assert_eq!(batch.moved.len(), 1);
        assert_ne!(batch.moved[0].id, tree.source_photos.id);
        let fresh = db.get_node(tree.alice, tree.fresh.id).await.expect("fresh");
        assert_eq!(fresh.parent_id, Some(batch.moved[0].id));
        assert!(
            db.get_node(tree.alice, tree.source_photos.id)
                .await
                .is_err()
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn skipping_a_move_keeps_clashing_files_and_their_folders(pool: PgPool) {
        let db = Db::from_pool(pool);
        let tree = tree(&db).await;

        let batch = db
            .move_nodes(
                tree.alice,
                &[tree.source_photos.id],
                Some(tree.target.id),
                OnConflict::Skip,
            )
            .await
            .expect("move");

        assert!(batch.conflicts.is_empty());
        assert_eq!(
            batch.skipped.iter().map(|node| node.id).collect::<Vec<_>>(),
            [tree.clashing.id]
        );
        let clashing = db
            .get_node(tree.alice, tree.clashing.id)
            .await
            .expect("clash");
        let existing = db
            .get_node(tree.alice, tree.existing.id)
            .await
            .expect("existing");
        assert_ne!(clashing.parent_id, existing.parent_id);
        assert_eq!(existing.version, 1);
        // The other files moved; the source folders that still hold the clash stay.
        for moved in [tree.fresh.id, tree.nested_fresh.id] {
            let node = db.get_node(tree.alice, moved).await.expect("moved");
            assert_ne!(node.parent_id, Some(tree.source_photos.id));
        }
        assert!(db.get_node(tree.alice, tree.source_photos.id).await.is_ok());
        assert!(
            batch
                .deleted
                .iter()
                .all(|node| node.id != tree.source_photos.id)
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn selected_files_with_the_same_name_clash_with_each_other(pool: PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let one = folder(&db, alice, None, "one").await;
        let two = folder(&db, alice, None, "two").await;
        let target = folder(&db, alice, None, "target").await;
        let a = file(&db, alice, one.id, "same.txt").await;
        let b = file(&db, alice, two.id, "same.txt").await;
        let other = file(&db, alice, two.id, "other.txt").await;
        let ids = [a.id, b.id, other.id];

        let aborted = db
            .move_nodes(alice, &ids, Some(target.id), OnConflict::Abort)
            .await
            .expect("abort");
        assert_eq!(aborted.conflicts.len(), 2);
        assert!(aborted.conflicts.iter().all(|c| c.existing.is_none()));
        assert!(aborted.conflicts.iter().all(|c| c.path == "same.txt"));

        let skipped = db
            .move_nodes(alice, &ids, Some(target.id), OnConflict::Skip)
            .await
            .expect("skip");
        assert_eq!(skipped.skipped.len(), 2);
        assert_eq!(
            skipped.moved.iter().map(|node| node.id).collect::<Vec<_>>(),
            [other.id]
        );
        let listing = db
            .list_children(alice, Some(target.id))
            .await
            .expect("list");
        assert_eq!(listing.len(), 1);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn nodes_inside_a_selected_folder_move_along_with_it(pool: PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let dir = folder(&db, alice, None, "dir").await;
        let inner = file(&db, alice, dir.id, "a.txt").await;
        let target = folder(&db, alice, None, "target").await;

        let batch = db
            .move_nodes(
                alice,
                &[inner.id, dir.id, dir.id],
                Some(target.id),
                OnConflict::Abort,
            )
            .await
            .expect("move");

        assert_eq!(batch.moved.len(), 1);
        assert_eq!(batch.moved[0].id, dir.id);
        let inner = db.get_node(alice, inner.id).await.expect("inner");
        assert_eq!(inner.parent_id, Some(dir.id));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn moving_checks_cycles_ownership_and_the_target(pool: PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let bob = db.create_user("bob", "x").await.expect("bob").id;
        let outer = folder(&db, alice, None, "outer").await;
        let inner = folder(&db, alice, Some(outer.id), "inner").await;
        let plain = file(&db, alice, outer.id, "a.txt").await;
        let move_to = |ids: Vec<NodeId>, parent: Option<NodeId>, owner| {
            let db = db.clone();
            async move {
                db.move_nodes(owner, &ids, parent, OnConflict::Override)
                    .await
            }
        };
        assert!(matches!(
            move_to(vec![outer.id], Some(inner.id), alice).await,
            Err(DbError::Cycle)
        ));
        assert!(matches!(
            move_to(vec![outer.id], Some(outer.id), alice).await,
            Err(DbError::Cycle)
        ));
        assert!(matches!(
            move_to(vec![plain.id], Some(plain.id), alice).await,
            Err(DbError::NotFound)
        ));
        assert!(matches!(
            move_to(vec![plain.id], Some(inner.id), bob).await,
            Err(DbError::NotFound)
        ));
        assert!(matches!(
            move_to(vec![plain.id, NodeId::new()], Some(inner.id), alice).await,
            Err(DbError::NotFound)
        ));
        let root = move_to(vec![plain.id], None, alice).await.expect("to root");
        assert!(root.target.is_none());
        assert_eq!(root.moved[0].parent_id, None);
    }
}
