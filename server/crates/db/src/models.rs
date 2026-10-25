use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{NodeId, UserId};

/// A registered user.
#[derive(Clone, Debug, FromRow)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: String,
    pub created_at: OffsetDateTime,
}

/// A stored refresh token (only its hash is kept).
#[derive(Clone, Debug, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: UserId,
    pub family_id: Uuid,
    pub token_hash: Vec<u8>,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

/// Whether a node is a folder or a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum NodeKind {
    Folder,
    File,
}

/// A file or folder in a user's tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct Node {
    pub id: NodeId,
    #[serde(skip)]
    pub owner_id: UserId,
    pub parent_id: Option<NodeId>,
    pub kind: NodeKind,
    pub name: String,
    pub size: i64,
    #[serde(with = "hex_option")]
    pub sha256: Option<Vec<u8>>,
    pub mime: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

mod hex_option {
    use std::fmt::Write;

    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(bytes) => serializer.serialize_str(&bytes.iter().fold(
                String::with_capacity(bytes.len() * 2),
                |mut hex, byte| {
                    let _ = write!(hex, "{byte:02x}");
                    hex
                },
            )),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|hex| {
                (0..hex.len())
                    .step_by(2)
                    .map(|start| {
                        hex.get(start..start + 2)
                            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                            .ok_or_else(|| D::Error::custom("invalid hex"))
                    })
                    .collect()
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(sha256: Option<Vec<u8>>) -> Node {
        let now = OffsetDateTime::UNIX_EPOCH;
        Node {
            id: NodeId::new(),
            owner_id: UserId::new(),
            parent_id: None,
            kind: NodeKind::File,
            name: "a.txt".to_owned(),
            size: 1,
            sha256,
            mime: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn sha256_serializes_as_lowercase_hex() {
        let json = serde_json::to_value(node(Some(vec![0x0a, 0xff]))).expect("json");
        assert_eq!(json["sha256"], "0aff");
    }

    #[test]
    fn missing_sha256_serializes_as_null() {
        let json = serde_json::to_value(node(None)).expect("json");
        assert!(json["sha256"].is_null());
    }
}
