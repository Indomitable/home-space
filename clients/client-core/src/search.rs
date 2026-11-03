use protocol::{NodeId, NodeKind, SearchHit};

/// The folder to open for a hit: the folder itself, or the folder holding a file.
/// `None` is the root.
pub fn destination(hit: &SearchHit) -> Option<NodeId> {
    match hit.node.kind {
        NodeKind::Folder => Some(hit.node.id),
        NodeKind::File => hit.node.parent_id,
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use protocol::Node;

    use super::*;

    fn hit(kind: NodeKind, parent: Option<u128>) -> SearchHit {
        SearchHit {
            node: Node {
                id: NodeId(Uuid::from_u128(1)),
                parent_id: parent.map(|raw| NodeId(Uuid::from_u128(raw))),
                kind,
                name: "n".to_owned(),
                size: 0,
                sha256: None,
                mime: String::new(),
                version: 1,
                created_at: OffsetDateTime::UNIX_EPOCH,
                updated_at: OffsetDateTime::UNIX_EPOCH,
                edited_on: None,
            },
            location: Vec::new(),
        }
    }

    #[test]
    fn a_folder_hit_opens_the_folder_itself() {
        let hit = hit(NodeKind::Folder, Some(7));
        assert_eq!(destination(&hit), Some(hit.node.id));
    }

    #[test]
    fn a_file_hit_opens_its_parent() {
        assert_eq!(
            destination(&hit(NodeKind::File, Some(7))),
            Some(NodeId(Uuid::from_u128(7)))
        );
    }

    #[test]
    fn a_root_file_hit_opens_the_root() {
        assert_eq!(destination(&hit(NodeKind::File, None)), None);
    }
}
