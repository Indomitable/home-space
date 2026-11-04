use serde::{Deserialize, Serialize};

/// Body of `GET /api/v1/health`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    /// `ok` when the server and its database are up.
    pub status: String,
}

impl Health {
    pub const OK: &str = "ok";

    pub fn ok() -> Self {
        Self {
            status: Self::OK.to_owned(),
        }
    }

    pub fn is_ok(&self) -> bool {
        self.status == Self::OK
    }
}
