use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{Node, NodeId, OnConflict};

/// A file in the recycle bin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrashItem {
    pub node: Node,
    /// Where the file lived when it was deleted, e.g. `/Photos/2024/a.jpg`.
    pub original_path: String,
    #[serde(with = "time::serde::rfc3339")]
    pub deleted_at: OffsetDateTime,
}

/// Body of `POST /trash/restore`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreRequest {
    pub ids: Vec<NodeId>,
    #[serde(default)]
    pub on_conflict: OnConflict,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn restore_defaults_to_abort() {
        let id = Uuid::nil();
        let request: RestoreRequest =
            serde_json::from_str(&format!(r#"{{"ids":["{id}"]}}"#)).expect("request");
        assert_eq!(request.on_conflict, OnConflict::Abort);
        assert_eq!(request.ids, [NodeId(id)]);
    }
}
