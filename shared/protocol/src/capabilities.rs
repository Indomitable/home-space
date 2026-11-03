use serde::{Deserialize, Serialize};

/// What the server supports, from `GET /capabilities`; clients read it after signing in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// The largest request body a file upload may send. Files up to this size go up in one
    /// request; larger ones through `/uploads` in chunks of at most this size.
    pub upload_chunk_bytes: u64,
}
