use serde::{Deserialize, Serialize};

use crate::Node;

/// Query of `GET /search`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// One search result: the matching node and the names of its ancestors, root first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub node: Node,
    pub location: Vec<String>,
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use crate::{NodeId, NodeKind};

    use super::*;

    #[test]
    fn hit_round_trips_through_json() {
        let hit = SearchHit {
            node: Node {
                id: NodeId(Uuid::nil()),
                parent_id: None,
                kind: NodeKind::File,
                name: "a.txt".to_owned(),
                size: 1,
                sha256: None,
                mime: "image/jpeg".to_owned(),
                version: 1,
                created_at: OffsetDateTime::UNIX_EPOCH,
                updated_at: OffsetDateTime::UNIX_EPOCH,
            },
            location: vec!["docs".to_owned(), "2024".to_owned()],
        };
        let json = serde_json::to_string(&hit).expect("json");
        assert_eq!(serde_json::from_str::<SearchHit>(&json).expect("back"), hit);
    }

    #[test]
    fn query_omits_absent_limit() {
        let query = SearchQuery {
            q: "holiday".to_owned(),
            limit: None,
        };
        assert_eq!(
            serde_json::to_string(&query).expect("json"),
            r#"{"q":"holiday"}"#
        );
    }
}
