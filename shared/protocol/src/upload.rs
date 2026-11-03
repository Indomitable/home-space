//! Resumable chunked uploads (`/uploads`).
//!
//! A client creates an upload with [`CreateUpload`], sends the bytes in order with
//! `PATCH /uploads/{id}` and an [`UPLOAD_OFFSET`](crate::paths::UPLOAD_OFFSET) header naming
//! where the chunk starts, and asks `GET /uploads/{id}` where to continue after an error.
//! The chunk that brings the offset to `size` stores the file.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Node, NodeId};

/// Body of `POST /uploads`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateUpload {
    /// Folder of a new file (the root when `None`); ignored when `node_id` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<NodeId>,
    /// Name of a new file; ignored when `node_id` is set.
    #[serde(default)]
    pub name: String,
    /// Total size of the content in bytes.
    pub size: u64,
    /// When the content was last changed on the client; omitted means now.
    #[serde(
        default,
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub modified_at: Option<OffsetDateTime>,
    /// Declared MIME type of the content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    /// The existing file whose content the upload replaces, like `PUT /files/{id}/content`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
}

/// An upload in progress, returned by `POST /uploads` and `GET /uploads/{id}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadSession {
    pub id: Uuid,
    /// Bytes received so far; the next chunk starts here.
    pub offset: u64,
    pub size: u64,
    /// The chunk size the server asks for; a larger chunk is rejected.
    pub chunk_size: u64,
}

/// Response of `PATCH /uploads/{id}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChunkResult {
    /// Bytes received so far.
    pub offset: u64,
    /// The stored file, once the last chunk has arrived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<Node>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_upload_defaults_optional_fields() {
        let request: CreateUpload =
            serde_json::from_str(r#"{"name":"a.bin","size":3}"#).expect("json");
        assert_eq!(request.parent_id, None);
        assert_eq!(request.node_id, None);
        assert_eq!(request.size, 3);
    }
}
