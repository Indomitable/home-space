use serde::{Deserialize, Serialize};

use crate::{Node, NodeId};

/// A change to a user's tree, pushed over the websocket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    NodeCreated {
        node: Node,
    },
    NodeUpdated {
        node: Node,
    },
    NodeDeleted {
        id: NodeId,
        parent_id: Option<NodeId>,
    },
    /// Files entered or left the recycle bin.
    TrashChanged,
    /// Events were missed; the client should refetch what it displays.
    Resync,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn resync_serializes_as_a_bare_tag() {
        assert_eq!(
            serde_json::to_string(&Event::Resync).expect("json"),
            r#"{"type":"resync"}"#
        );
    }

    #[test]
    fn trash_changed_serializes_as_a_bare_tag() {
        assert_eq!(
            serde_json::to_string(&Event::TrashChanged).expect("json"),
            r#"{"type":"trash_changed"}"#
        );
    }

    #[test]
    fn deleted_event_round_trips() {
        let event = Event::NodeDeleted {
            id: NodeId(Uuid::nil()),
            parent_id: None,
        };
        let json = serde_json::to_string(&event).expect("json");
        assert!(json.contains(r#""type":"node_deleted""#));
        assert_eq!(serde_json::from_str::<Event>(&json).expect("back"), event);
    }
}
