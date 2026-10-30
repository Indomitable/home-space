use serde::{Deserialize, Serialize};

/// Machine-readable error category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unauthorized,
    NotFound,
    Conflict,
    BadRequest,
    PayloadTooLarge,
    /// What was asked for is no longer kept, such as pruned changes.
    Gone,
    Internal,
}

/// Body of every error response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: ErrorCode,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_uses_snake_case() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::PayloadTooLarge).expect("json"),
            r#""payload_too_large""#
        );
    }
}
