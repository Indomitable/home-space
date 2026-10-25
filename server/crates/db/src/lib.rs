//! Postgres access for home-space: pool setup, migrations, models and repositories.

mod error;
mod ids;
mod models;
mod nodes;
mod refresh_tokens;
mod users;

pub use error::DbError;
pub use ids::{NodeId, UserId};
pub use models::{Node, NodeKind, RefreshToken, User};
pub use nodes::{NewNode, NodeUpdate};
pub use sqlx::PgPool;

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
