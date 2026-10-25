use std::ops::Range;

use protocol::{Event, Node, NodeId, NodeKind};

/// How many breadcrumb folders are shown before the middle ones collapse.
pub const MAX_VISIBLE_CRUMBS: usize = 3;

/// Orders nodes folders first, then by case-insensitive name.
pub fn sort_nodes(nodes: &mut [Node]) {
    nodes.sort_by_cached_key(|node| (node.kind != NodeKind::Folder, node.name.to_lowercase()));
}

/// Applies a live event to the listing of `folder`.
pub fn apply_event(nodes: &mut Vec<Node>, folder: Option<NodeId>, event: &Event) {
    match event {
        Event::NodeCreated { node } | Event::NodeUpdated { node } => {
            nodes.retain(|existing| existing.id != node.id);
            if node.parent_id == folder {
                nodes.push(node.clone());
                sort_nodes(nodes);
            }
        }
        Event::NodeDeleted { id, .. } => nodes.retain(|existing| existing.id != *id),
        Event::Resync => {}
    }
}

/// Whether `event` changes a folder on the breadcrumb path, which needs a refetch.
pub fn affects_path(path: &[Node], event: &Event) -> bool {
    match event {
        Event::NodeUpdated { node } => path.iter().any(|folder| folder.id == node.id),
        Event::NodeDeleted { id, .. } => path.iter().any(|folder| folder.id == *id),
        Event::NodeCreated { .. } | Event::Resync => false,
    }
}

/// The indices of breadcrumb folders hidden behind an overflow menu once a path of
/// `len` folders is deeper than [`MAX_VISIBLE_CRUMBS`]; the first and last stay visible.
pub fn collapsed_crumbs(len: usize) -> Option<Range<usize>> {
    (len > MAX_VISIBLE_CRUMBS).then(|| 1..len - 1)
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;

    fn node(name: &str, kind: NodeKind, parent: Option<NodeId>) -> Node {
        Node {
            id: NodeId::from(uuid_from(name)),
            parent_id: parent,
            kind,
            name: name.to_owned(),
            size: 0,
            sha256: None,
            mime: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    fn uuid_from(name: &str) -> uuid::Uuid {
        let mut bytes = [0_u8; 16];
        for (slot, byte) in bytes.iter_mut().zip(name.bytes()) {
            *slot = byte;
        }
        uuid::Uuid::from_bytes(bytes)
    }

    fn names(nodes: &[Node]) -> Vec<&str> {
        nodes.iter().map(|node| node.name.as_str()).collect()
    }

    #[test]
    fn folders_sort_before_files_and_names_ignore_case() {
        let mut nodes = vec![
            node("b.txt", NodeKind::File, None),
            node("Zeta", NodeKind::Folder, None),
            node("alpha", NodeKind::Folder, None),
            node("A.txt", NodeKind::File, None),
        ];
        sort_nodes(&mut nodes);
        assert_eq!(names(&nodes), ["alpha", "Zeta", "A.txt", "b.txt"]);
    }

    #[test]
    fn created_node_in_the_open_folder_is_inserted_in_order() {
        let mut nodes = vec![node("b.txt", NodeKind::File, None)];
        let event = Event::NodeCreated {
            node: node("a.txt", NodeKind::File, None),
        };
        apply_event(&mut nodes, None, &event);
        assert_eq!(names(&nodes), ["a.txt", "b.txt"]);
    }

    #[test]
    fn created_node_elsewhere_is_ignored() {
        let mut nodes = Vec::new();
        let other = Some(NodeId::from(uuid_from("other")));
        let event = Event::NodeCreated {
            node: node("a.txt", NodeKind::File, other),
        };
        apply_event(&mut nodes, None, &event);
        assert!(nodes.is_empty());
    }

    #[test]
    fn node_moved_out_of_the_open_folder_disappears() {
        let mut nodes = vec![node("a.txt", NodeKind::File, None)];
        let other = Some(NodeId::from(uuid_from("other")));
        let event = Event::NodeUpdated {
            node: node("a.txt", NodeKind::File, other),
        };
        apply_event(&mut nodes, None, &event);
        assert!(nodes.is_empty());
    }

    #[test]
    fn renamed_node_replaces_its_old_row() {
        let mut nodes = vec![node("a.txt", NodeKind::File, None)];
        let mut renamed = node("a.txt", NodeKind::File, None);
        renamed.name = "z.txt".to_owned();
        apply_event(&mut nodes, None, &Event::NodeUpdated { node: renamed });
        assert_eq!(names(&nodes), ["z.txt"]);
    }

    #[test]
    fn deleted_node_is_removed() {
        let file = node("a.txt", NodeKind::File, None);
        let event = Event::NodeDeleted {
            id: file.id,
            parent_id: None,
        };
        let mut nodes = vec![file];
        apply_event(&mut nodes, None, &event);
        assert!(nodes.is_empty());
    }

    #[test]
    fn deleting_a_breadcrumb_folder_needs_a_refetch() {
        let folder = node("docs", NodeKind::Folder, None);
        let event = Event::NodeDeleted {
            id: folder.id,
            parent_id: None,
        };
        assert!(affects_path(&[folder], &event));
        assert!(!affects_path(&[], &event));
    }

    #[test]
    fn short_paths_are_not_collapsed() {
        assert_eq!((collapsed_crumbs(0), collapsed_crumbs(3)), (None, None));
    }

    #[test]
    fn deep_paths_hide_the_middle_folders() {
        assert_eq!(
            (collapsed_crumbs(4), collapsed_crumbs(6)),
            (Some(1..3), Some(1..5))
        );
    }
}
