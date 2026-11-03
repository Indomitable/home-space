use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Node, NodeId};

/// Query of `GET /changes`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangesQuery {
    /// The cursor of the previous page; omitted to only learn where the journal ends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// The latest change to one node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Change {
    /// The node was created or changed; this is its current state.
    Upserted {
        seq: i64,
        node: Node,
        /// The device that made the change, when it was made from one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        device_id: Option<Uuid>,
    },
    /// The node was deleted or moved to the recycle bin.
    Deleted {
        seq: i64,
        id: NodeId,
        /// The device that deleted it, when it was deleted from one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        device_id: Option<Uuid>,
    },
}

/// Body of `GET /changes`.
///
/// A `since` older than what the server still keeps is answered with `410 Gone`; the client
/// then reads the whole tree again, starting from a cursor fetched without `since`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangesResponse {
    /// One entry per changed node, in the order of their latest change.
    pub changes: Vec<Change>,
    /// The `since` of the next request.
    pub cursor: i64,
    /// Whether more changes follow `cursor`.
    pub more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleted_change_is_tagged_and_round_trips() {
        let change = Change::Deleted {
            seq: 7,
            id: NodeId(Uuid::nil()),
            device_id: Some(Uuid::nil()),
        };
        let json = serde_json::to_string(&change).expect("json");
        assert!(json.contains(r#""type":"deleted""#));
        assert_eq!(serde_json::from_str::<Change>(&json).expect("back"), change);
    }

    #[test]
    fn a_change_from_an_older_server_has_no_device() {
        let json = r#"{"type":"deleted","seq":7,"id":"00000000-0000-0000-0000-000000000000"}"#;
        let change: Change = serde_json::from_str(json).expect("change");
        assert!(matches!(
            change,
            Change::Deleted {
                device_id: None,
                ..
            }
        ));
    }
}
