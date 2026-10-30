use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use protocol::{NodeId, NodeKind};

use super::model::{Mapping, RemoteNode};
use crate::transfers::sanitize_component;

/// Where every synced remote node lives on disk.
///
/// A node belongs to its nearest mapped ancestor: it sits under that mapping's local folder,
/// at its path relative to the mapped remote folder. A folder with a mapping of its own is
/// therefore left out of its parent's local folder, and one set to not sync is left out
/// everywhere.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    /// The synced nodes by local path; mapped folders themselves are not entries.
    pub entries: HashMap<PathBuf, RemoteNode>,
    /// The local path of each entry.
    pub paths: HashMap<NodeId, PathBuf>,
    /// The remote folder each synced local folder stands for, including the mapped roots
    /// (`None` for Home).
    pub folders: HashMap<PathBuf, Option<NodeId>>,
    /// The local folders of the mappings.
    pub roots: Vec<PathBuf>,
    /// Local paths that would hold a folder mapped elsewhere or set to not sync, which the
    /// mapping they are in leaves alone.
    pub skipped: HashSet<PathBuf>,
    /// Folders set to not sync.
    pub disabled: HashSet<NodeId>,
}

impl Layout {
    /// Places the `nodes` according to `mappings`.
    pub fn new(nodes: &HashMap<NodeId, RemoteNode>, mappings: &[Mapping]) -> Self {
        let mut children: HashMap<Option<NodeId>, Vec<&RemoteNode>> = HashMap::new();
        for node in nodes.values() {
            children.entry(node.parent).or_default().push(node);
        }
        for siblings in children.values_mut() {
            // Two names can clash once sanitized; the order settles which one is synced.
            siblings.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.0.cmp(&b.id.0)));
        }
        let mapped: HashSet<Option<NodeId>> = mappings.iter().map(|m| m.remote).collect();
        let mut layout = Self::default();
        for mapping in mappings {
            let Some(root) = &mapping.local else {
                layout.disabled.extend(mapping.remote);
                continue;
            };
            let is_folder = mapping.remote.is_none_or(|id| {
                nodes
                    .get(&id)
                    .is_some_and(|node| node.kind == NodeKind::Folder)
            });
            if is_folder {
                layout.roots.push(root.clone());
                layout.folders.insert(root.clone(), mapping.remote);
                layout.place(&children, &mapped, mapping.remote, root);
            }
        }
        layout
    }

    fn place(
        &mut self,
        children: &HashMap<Option<NodeId>, Vec<&RemoteNode>>,
        mapped: &HashSet<Option<NodeId>>,
        top: Option<NodeId>,
        root: &Path,
    ) {
        let mut pending = vec![(top, root.to_path_buf())];
        while let Some((parent, dir)) = pending.pop() {
            for child in children.get(&parent).into_iter().flatten() {
                let path = dir.join(sanitize_component(&child.name));
                if mapped.contains(&Some(child.id)) {
                    self.skipped.insert(path);
                    continue;
                }
                if self.entries.contains_key(&path) || self.paths.contains_key(&child.id) {
                    continue;
                }
                self.paths.insert(child.id, path.clone());
                self.entries.insert(path.clone(), (*child).clone());
                if child.kind == NodeKind::Folder {
                    self.folders.insert(path.clone(), Some(child.id));
                    pending.push((Some(child.id), path));
                }
            }
        }
    }

    /// Whether `path` is synced: under a mapping's folder and not left out of it.
    pub fn covers(&self, path: &Path) -> bool {
        self.roots
            .iter()
            .any(|root| path.starts_with(root) && path != root)
            && !self.skipped.iter().any(|skipped| path.starts_with(skipped))
    }
}

/// Why `local` cannot be the folder of a new mapping next to `others`: local folders may not
/// lie inside one another, nor inside the app's own data.
pub fn conflict(local: &Path, others: &[&Path], data_dir: Option<&Path>) -> Option<String> {
    if !local.is_absolute() {
        return Some("choose an absolute folder".to_owned());
    }
    if data_dir.is_some_and(|data| local.starts_with(data) || data.starts_with(local)) {
        return Some("this folder holds the app's own data".to_owned());
    }
    others
        .iter()
        .find(|other| local.starts_with(other) || other.starts_with(local))
        .map(|other| format!("overlaps the synced folder {}", other.display()))
}

#[cfg(test)]
pub(super) mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    pub fn id(n: u128) -> NodeId {
        NodeId(Uuid::from_u128(n))
    }

    pub fn node(n: u128, parent: Option<u128>, kind: NodeKind, name: &str) -> RemoteNode {
        RemoteNode {
            id: id(n),
            parent: parent.map(id),
            kind,
            name: name.to_owned(),
            size: 1,
            sha256: (kind == NodeKind::File).then(|| format!("sha-{n}")),
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    pub fn tree(nodes: Vec<RemoteNode>) -> HashMap<NodeId, RemoteNode> {
        nodes.into_iter().map(|node| (node.id, node)).collect()
    }

    fn map(remote: Option<u128>, local: Option<&str>) -> Mapping {
        Mapping {
            remote: remote.map(id),
            local: local.map(PathBuf::from),
        }
    }

    /// Home with `Docs/a.txt`, `Photos/p.jpg` and `Photos/Raw/r.raw`.
    fn home() -> HashMap<NodeId, RemoteNode> {
        tree(vec![
            node(1, None, NodeKind::Folder, "Docs"),
            node(2, Some(1), NodeKind::File, "a.txt"),
            node(3, None, NodeKind::Folder, "Photos"),
            node(4, Some(3), NodeKind::File, "p.jpg"),
            node(5, Some(3), NodeKind::Folder, "Raw"),
            node(6, Some(5), NodeKind::File, "r.raw"),
        ])
    }

    #[test]
    fn home_places_every_node_under_its_folder() {
        let layout = Layout::new(&home(), &[map(None, Some("/sync"))]);
        assert_eq!(layout.paths[&id(2)], Path::new("/sync/Docs/a.txt"));
        assert_eq!(layout.paths[&id(6)], Path::new("/sync/Photos/Raw/r.raw"));
        assert_eq!(layout.folders[Path::new("/sync")], None);
        assert_eq!(layout.folders[Path::new("/sync/Photos")], Some(id(3)));
    }

    #[test]
    fn a_folder_mapped_elsewhere_lives_only_at_its_own_location() {
        let layout = Layout::new(
            &home(),
            &[map(None, Some("/sync")), map(Some(3), Some("/photos"))],
        );
        assert_eq!(layout.paths[&id(4)], Path::new("/photos/p.jpg"));
        assert_eq!(layout.paths[&id(6)], Path::new("/photos/Raw/r.raw"));
        assert!(!layout.paths.contains_key(&id(3)));
        assert!(layout.skipped.contains(Path::new("/sync/Photos")));
        assert!(!layout.covers(Path::new("/sync/Photos/p.jpg")));
        assert!(layout.covers(Path::new("/photos/p.jpg")));
    }

    #[test]
    fn a_folder_set_to_not_sync_is_left_out() {
        let layout = Layout::new(&home(), &[map(None, Some("/sync")), map(Some(5), None)]);
        assert!(layout.paths.contains_key(&id(4)));
        assert!(!layout.paths.contains_key(&id(5)));
        assert!(!layout.paths.contains_key(&id(6)));
        assert!(layout.disabled.contains(&id(5)));
        assert!(!layout.covers(Path::new("/sync/Photos/Raw/new.txt")));
    }

    #[test]
    fn the_mapped_folders_themselves_are_not_covered() {
        let layout = Layout::new(&home(), &[map(Some(1), Some("/docs"))]);
        assert!(!layout.covers(Path::new("/docs")));
        assert!(layout.covers(Path::new("/docs/a.txt")));
        assert!(!layout.covers(Path::new("/elsewhere/a.txt")));
    }

    #[test]
    fn local_folders_may_not_overlap() {
        let others = [Path::new("/sync")];
        assert!(conflict(Path::new("/sync/inner"), &others, None).is_some());
        assert!(conflict(Path::new("/"), &others, None).is_some());
        assert!(conflict(Path::new("/photos"), &others, None).is_none());
        assert!(conflict(Path::new("relative"), &[], None).is_some());
        let data = Path::new("/home/me/.local/share/home-space");
        assert!(conflict(Path::new("/home/me"), &[], Some(data)).is_some());
    }

    #[test]
    fn clashing_names_sync_only_one_node() {
        let nodes = tree(vec![
            node(1, None, NodeKind::File, "a:b"),
            node(2, None, NodeKind::File, "a?b"),
        ]);
        let layout = Layout::new(&nodes, &[map(None, Some("/sync"))]);
        assert_eq!(layout.entries.len(), 1);
    }
}
