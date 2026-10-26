use uuid::Uuid;

use crate::{Db, DbError, FileVersion, NodeId, UserId};

macro_rules! version_columns {
    () => {
        "id, node_id, owner_id, version, size, sha256, mime, modified_at, archived_at"
    };
}

impl Db {
    /// Lists the previous versions of a file, newest first.
    pub async fn list_versions(
        &self,
        owner: UserId,
        node: NodeId,
    ) -> Result<Vec<FileVersion>, DbError> {
        self.get_node(owner, node).await?;
        let versions = sqlx::query_as::<_, FileVersion>(concat!(
            "SELECT ",
            version_columns!(),
            " FROM file_versions \
             WHERE owner_id = $1 AND node_id = $2 ORDER BY version DESC"
        ))
        .bind(owner)
        .bind(node)
        .fetch_all(self.pool())
        .await?;
        Ok(versions)
    }

    /// Fetches one previous version of a file.
    pub async fn get_version(
        &self,
        owner: UserId,
        node: NodeId,
        version: i32,
    ) -> Result<FileVersion, DbError> {
        let version = sqlx::query_as::<_, FileVersion>(concat!(
            "SELECT ",
            version_columns!(),
            " FROM file_versions \
             WHERE owner_id = $1 AND node_id = $2 AND version = $3"
        ))
        .bind(owner)
        .bind(node)
        .bind(version)
        .fetch_one(self.pool())
        .await?;
        Ok(version)
    }

    /// Deletes a previous version; returns its id so the stored content can be removed.
    pub async fn delete_version(
        &self,
        owner: UserId,
        node: NodeId,
        version: i32,
    ) -> Result<Uuid, DbError> {
        let id = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM file_versions \
             WHERE owner_id = $1 AND node_id = $2 AND version = $3 RETURNING id",
        )
        .bind(owner)
        .bind(node)
        .bind(version)
        .fetch_one(self.pool())
        .await?;
        Ok(id)
    }
}
