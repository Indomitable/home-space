use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use protocol::NodeId;

use super::{
    layout::Layout,
    model::{Base, RemoteNode},
};

/// The sync state shown on a file or folder; nodes outside every mapping have none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Badge {
    Synced,
    Pending,
    Problem,
    /// Set to not sync by the user.
    Disabled,
}

/// The badges of all nodes, with Home's under `None`.
pub type Badges = HashMap<Option<NodeId>, Badge>;

/// Works out every badge: synced nodes are [`Badge::Synced`] once recorded, nodes with
/// work ahead are [`Badge::Pending`] and failed ones [`Badge::Problem`]. A folder shows the
/// worst state of anything inside it, and so do the mapped folders and Home.
pub fn badges(
    layout: &Layout,
    nodes: &HashMap<NodeId, RemoteNode>,
    bases: &HashSet<NodeId>,
    pending: &[PathBuf],
    problems: &[PathBuf],
) -> Badges {
    let mut badges = Badges::new();
    for root in &layout.roots {
        if let Some(remote) = layout.folders.get(root) {
            badges.insert(*remote, Badge::Synced);
        }
    }
    for id in layout.paths.keys() {
        let badge = if bases.contains(id) {
            Badge::Synced
        } else {
            Badge::Pending
        };
        badges.insert(Some(*id), badge);
    }
    let raise = |badges: &mut Badges, path: &Path, badge: Badge| {
        let Some(owner) = owner(layout, path) else {
            return;
        };
        let mut current = owner;
        loop {
            let entry = badges.entry(current).or_insert(badge);
            *entry = (*entry).max(badge);
            let Some(id) = current else { break };
            match nodes.get(&id) {
                Some(node) => current = node.parent,
                None => break,
            }
        }
    };
    for path in pending {
        raise(&mut badges, path, Badge::Pending);
    }
    for path in problems {
        raise(&mut badges, path, Badge::Problem);
    }
    // Up the parents, a folder outside every mapping above a mapped one gets no badge.
    badges.retain(|id, _| match id {
        Some(id) => {
            layout.paths.contains_key(id) || layout.folders.values().any(|f| *f == Some(*id))
        }
        None => layout.folders.values().any(Option::is_none),
    });
    for id in &layout.disabled {
        badges.insert(Some(*id), Badge::Disabled);
    }
    badges
}

/// The node standing for `path`, or for the nearest folder above it that has one.
fn owner(layout: &Layout, path: &Path) -> Option<Option<NodeId>> {
    if let Some(node) = layout.entries.get(path) {
        return Some(Some(node.id));
    }
    path.ancestors()
        .skip(1)
        .find_map(|parent| layout.folders.get(parent).copied())
}

/// The ids of the recorded bases.
pub fn based(bases: &[Base]) -> HashSet<NodeId> {
    bases.iter().map(|base| base.node).collect()
}

#[cfg(test)]
mod tests {
    use protocol::NodeKind::{File, Folder};

    use super::*;
    use crate::sync::{
        layout::tests::{id, node, tree},
        model::Mapping,
    };

    fn layout(nodes: &HashMap<NodeId, RemoteNode>) -> Layout {
        Layout::new(
            nodes,
            &[
                Mapping {
                    remote: None,
                    local: Some(PathBuf::from("/sync")),
                },
                Mapping {
                    remote: Some(id(4)),
                    local: None,
                },
            ],
        )
    }

    fn nodes() -> HashMap<NodeId, RemoteNode> {
        tree(vec![
            node(1, None, Folder, "dir"),
            node(2, Some(1), File, "a.txt"),
            node(3, Some(1), File, "b.txt"),
            node(4, None, Folder, "off"),
        ])
    }

    #[test]
    fn recorded_nodes_are_synced_and_others_pending() {
        let nodes = nodes();
        let badges = badges(
            &layout(&nodes),
            &nodes,
            &HashSet::from([id(1), id(2)]),
            &[],
            &[],
        );
        assert_eq!(badges[&Some(id(2))], Badge::Synced);
        assert_eq!(badges[&Some(id(3))], Badge::Pending);
        assert_eq!(badges[&Some(id(1))], Badge::Synced);
        assert_eq!(badges[&None], Badge::Synced);
    }

    #[test]
    fn a_problem_shows_on_every_folder_above_it() {
        let nodes = nodes();
        let all = HashSet::from([id(1), id(2), id(3)]);
        let problems = [PathBuf::from("/sync/dir/a.txt")];
        let badges = badges(&layout(&nodes), &nodes, &all, &[], &problems);
        assert_eq!(badges[&Some(id(2))], Badge::Problem);
        assert_eq!(badges[&Some(id(1))], Badge::Problem);
        assert_eq!(badges[&None], Badge::Problem);
        assert_eq!(badges[&Some(id(3))], Badge::Synced);
    }

    #[test]
    fn a_new_local_file_marks_its_folder_pending() {
        let nodes = nodes();
        let all = HashSet::from([id(1), id(2), id(3)]);
        let pending = [PathBuf::from("/sync/dir/new.txt")];
        let badges = badges(&layout(&nodes), &nodes, &all, &pending, &[]);
        assert_eq!(badges[&Some(id(1))], Badge::Pending);
        assert_eq!(badges[&Some(id(2))], Badge::Synced);
    }

    #[test]
    fn a_folder_set_to_not_sync_is_disabled_and_outside_nodes_have_no_badge() {
        let mut nodes = nodes();
        nodes.insert(id(5), node(5, Some(4), File, "x.txt"));
        let badges = badges(&layout(&nodes), &nodes, &HashSet::new(), &[], &[]);
        assert_eq!(badges[&Some(id(4))], Badge::Disabled);
        assert!(!badges.contains_key(&Some(id(5))));
    }
}
