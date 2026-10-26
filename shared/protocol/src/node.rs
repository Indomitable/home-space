use serde::{Deserialize, Deserializer, Serialize};
use time::OffsetDateTime;

use crate::NodeId;

/// Whether a node is a folder or a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(sqlx::Type),
    sqlx(type_name = "text", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Folder,
    File,
}

/// A file or folder in a user's tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub parent_id: Option<NodeId>,
    pub kind: NodeKind,
    pub name: String,
    pub size: i64,
    pub sha256: Option<String>,
    pub mime: String,
    pub version: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// A previous version of a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileVersion {
    pub version: i32,
    pub size: i64,
    pub sha256: String,
    pub mime: String,
    #[serde(with = "time::serde::rfc3339")]
    pub modified_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub archived_at: OffsetDateTime,
}

/// Query of `GET /nodes`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<NodeId>,
}

/// Body of `POST /folders`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateFolderRequest {
    pub parent_id: Option<NodeId>,
    pub name: String,
}

/// Query of `POST /files`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<NodeId>,
    pub name: String,
}

/// Body of `PATCH /nodes/{id}`.
///
/// `parent_id` distinguishes absent (leave unchanged), `null` (move to root)
/// and a value (move into that folder).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub parent_id: Option<Option<NodeId>>,
}

fn present<'de, D>(deserializer: D) -> Result<Option<Option<NodeId>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<NodeId>::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn update_distinguishes_absent_null_and_value() {
        let absent: UpdateRequest = serde_json::from_str(r#"{"name":"a"}"#).expect("absent");
        let null: UpdateRequest = serde_json::from_str(r#"{"parent_id":null}"#).expect("null");
        let id = Uuid::nil();
        let value: UpdateRequest =
            serde_json::from_str(&format!(r#"{{"parent_id":"{id}"}}"#)).expect("value");
        assert_eq!(absent.parent_id, None);
        assert_eq!(null.parent_id, Some(None));
        assert_eq!(value.parent_id, Some(Some(NodeId(id))));
    }

    #[test]
    fn update_serialization_round_trips_move_to_root() {
        let request = UpdateRequest {
            name: None,
            parent_id: Some(None),
        };
        let json = serde_json::to_string(&request).expect("json");
        assert_eq!(json, r#"{"parent_id":null}"#);
        assert_eq!(
            serde_json::from_str::<UpdateRequest>(&json).expect("back"),
            request
        );
    }

    #[test]
    fn node_json_uses_rfc3339_and_lowercase_kind() {
        let node = Node {
            id: NodeId(Uuid::nil()),
            parent_id: None,
            kind: NodeKind::File,
            name: "a.txt".to_owned(),
            size: 1,
            sha256: Some("0aff".to_owned()),
            mime: "text/plain".to_owned(),
            version: 1,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };
        let json = serde_json::to_value(&node).expect("json");
        assert_eq!(json["kind"], "file");
        assert_eq!(json["created_at"], "1970-01-01T00:00:00Z");
        assert_eq!(serde_json::from_value::<Node>(json).expect("back"), node);
    }
}
