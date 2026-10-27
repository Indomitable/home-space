use db::DbError;

/// Errors returned by file operations.
#[derive(Debug, thiserror::Error)]
pub enum FilesError {
    /// The node does not exist or belongs to another user.
    #[error("not found")]
    NotFound,
    /// A node with the same name already exists in the folder.
    #[error("a node with this name already exists")]
    Conflict,
    /// The name is empty, too long or contains forbidden characters.
    #[error("invalid name")]
    InvalidName,
    /// A folder cannot be moved into itself or its descendants.
    #[error("cannot move a folder into itself")]
    Cycle,
    /// Lost & Found, or a folder containing it, cannot be deleted.
    #[error("Lost & Found cannot be deleted")]
    LostAndFound,
    /// The operation requires a file but the node is a folder.
    #[error("node is not a file")]
    NotAFile,
    /// The upload exceeded the configured size limit.
    #[error("upload exceeds the maximum size of {limit} bytes")]
    TooLarge { limit: u64 },
    #[error("storage error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Db(DbError),
}

impl From<DbError> for FilesError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound => Self::NotFound,
            DbError::Conflict => Self::Conflict,
            DbError::Invalid => Self::InvalidName,
            DbError::Cycle => Self::Cycle,
            DbError::LostAndFound => Self::LostAndFound,
            other => Self::Db(other),
        }
    }
}
