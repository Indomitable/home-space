use std::collections::{HashMap, HashSet};

use api_client::{ApiError, Client};
use protocol::{Change, ErrorCode, NodeId, NodeKind};

use super::model::{Mapping, RemoteNode};

/// The remote tree as last seen, kept current from the server's change journal.
#[derive(Debug, Clone, Default)]
pub struct Mirror {
    pub nodes: HashMap<NodeId, RemoteNode>,
    pub cursor: Option<i64>,
}

/// How the mirror changed in a refresh, for the store to record.
#[derive(Debug, Default)]
pub struct Delta {
    pub replace: bool,
    pub upserted: Vec<RemoteNode>,
    pub removed: Vec<NodeId>,
}

impl Mirror {
    /// Brings the mirror up to date: from the journal when it has a cursor, otherwise (or
    /// when the journal was pruned past it) by reading every mapped folder again.
    pub async fn refresh(
        &mut self,
        client: &Client,
        mappings: &[Mapping],
    ) -> Result<Delta, ApiError> {
        if let Some(cursor) = self.cursor {
            match self.catch_up(client, cursor).await {
                Err(ApiError::Api {
                    code: ErrorCode::Gone,
                    ..
                }) => {}
                other => return other,
            }
        }
        self.reload(client, mappings).await
    }

    /// Reads every mapped folder from scratch, taking the cursor first so that changes made
    /// meanwhile are caught up on next time.
    pub async fn reload(
        &mut self,
        client: &Client,
        mappings: &[Mapping],
    ) -> Result<Delta, ApiError> {
        let cursor = client.changes(None).await?.cursor;
        let mapped: HashSet<Option<NodeId>> = mappings.iter().map(|m| m.remote).collect();
        let mut nodes = HashMap::new();
        for mapping in mappings.iter().filter(|m| m.local.is_some()) {
            if let Some(id) = mapping.remote {
                match client.get(id).await {
                    Ok(node) => nodes.insert(id, RemoteNode::from(node)),
                    // The mapped folder is gone; its mapping simply has nothing to sync.
                    Err(ApiError::Api {
                        code: ErrorCode::NotFound,
                        ..
                    }) => continue,
                    Err(err) => return Err(err),
                };
            }
            for node in walk(client, mapping.remote, &mapped).await? {
                nodes.insert(node.id, node);
            }
        }
        // Folders set to not sync are kept so their own node is known for the badges.
        for id in mappings
            .iter()
            .filter(|m| m.local.is_none())
            .filter_map(|m| m.remote)
        {
            if let Some(node) = self.nodes.get(&id) {
                nodes.entry(id).or_insert_with(|| node.clone());
            }
        }
        self.nodes = nodes;
        self.cursor = Some(cursor);
        Ok(Delta {
            replace: true,
            upserted: self.nodes.values().cloned().collect(),
            removed: Vec::new(),
        })
    }

    async fn catch_up(&mut self, client: &Client, mut cursor: i64) -> Result<Delta, ApiError> {
        let mut delta = Delta::default();
        loop {
            let page = client.changes(Some(cursor)).await?;
            for change in page.changes {
                match change {
                    Change::Upserted { node, .. } => {
                        let node = RemoteNode::from(node);
                        let previous = self.nodes.insert(node.id, node.clone());
                        let arrived =
                            previous.is_none_or(|previous| previous.parent != node.parent);
                        delta.upserted.push(node.clone());
                        // A folder moved in brings content the journal does not list again.
                        if node.kind == NodeKind::Folder && arrived {
                            for inner in walk(client, Some(node.id), &HashSet::new()).await? {
                                self.nodes.insert(inner.id, inner.clone());
                                delta.upserted.push(inner);
                            }
                        }
                    }
                    Change::Deleted { id, .. } => delta.removed.extend(self.remove(id)),
                }
            }
            cursor = page.cursor;
            if !page.more {
                break;
            }
        }
        self.cursor = Some(cursor);
        Ok(delta)
    }

    /// Removes `id` and everything under it; returns what was removed.
    fn remove(&mut self, id: NodeId) -> Vec<NodeId> {
        let mut removed = Vec::new();
        let mut pending = vec![id];
        while let Some(id) = pending.pop() {
            let Some(node) = self.nodes.remove(&id) else {
                continue;
            };
            removed.push(id);
            if node.kind != NodeKind::Folder {
                continue;
            }
            pending.extend(
                self.nodes
                    .values()
                    .filter(|node| node.parent == Some(id))
                    .map(|node| node.id),
            );
        }
        removed
    }
}

/// Everything under `top`, not descending into the `mapped` folders, which are read on
/// their own (or not at all).
async fn walk(
    client: &Client,
    top: Option<NodeId>,
    mapped: &HashSet<Option<NodeId>>,
) -> Result<Vec<RemoteNode>, ApiError> {
    let mut found = Vec::new();
    let mut pending = vec![top];
    while let Some(parent) = pending.pop() {
        for node in client.list(parent).await? {
            let node = RemoteNode::from(node);
            if node.kind == NodeKind::Folder && !mapped.contains(&Some(node.id)) {
                pending.push(Some(node.id));
            }
            found.push(node);
        }
    }
    Ok(found)
}
