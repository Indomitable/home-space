//! Postgres access for home-space: pool setup, migrations, models and repositories.

/// Restricts a query on `nodes` (under `$alias`) to what the node API exposes: live nodes, so
/// neither binned files nor a user's recycle bin folder.
macro_rules! visible {
    ($alias:literal) => {
        concat!(
            $alias,
            ".trashed_at IS NULL AND NOT EXISTS \
             (SELECT 1 FROM user_folders uf WHERE uf.trash_id = ",
            $alias,
            ".id)"
        )
    };
}

mod changes;
mod error;
mod models;
mod nodes;
mod refresh_tokens;
mod search;
mod trash;
mod users;
mod versions;

pub use changes::{ChangeBatch, NodeChange};
pub use error::DbError;
pub use models::{FileVersion, Node, RefreshToken, User};
pub use nodes::{
    BlobOp, DeletedNodes, FOLDER_MIME, MoveBatch, MoveConflict, MoveOutcome, NewContent, NewNode,
    NodeUpdate, StoreOutcome,
};
pub use protocol::{NodeId, NodeKind, OnConflict, UserId};
pub use search::{ContentState, RankedNode};
pub use sqlx::PgPool;
pub use trash::{LOST_AND_FOUND, SpecialFolders, TrashOutcome, TrashedNode};

use sqlx::postgres::PgPoolOptions;

const MAX_CONNECTIONS: u32 = 10;

/// Handle to the database; cheap to clone.
#[derive(Clone, Debug)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    /// Connects to Postgres at `database_url`.
    pub async fn connect(database_url: &str) -> Result<Self, DbError> {
        let pool = PgPoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    /// Wraps an existing pool (used by tests).
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Applies all pending migrations.
    pub async fn migrate(&self) -> Result<(), DbError> {
        sqlx::migrate!("../../migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Verifies the database answers queries.
    pub async fn ping(&self) -> Result<(), DbError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }
}
