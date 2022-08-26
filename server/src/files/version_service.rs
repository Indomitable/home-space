use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use deadpool_postgres::Pool;
use uuid::Uuid;
use home_space_contracts::files::NODE_TYPE_FILE;
use crate::db::DatabaseAccess;
use crate::files::db::DbModel;
use crate::files::db::file_node::FileNodeDto;
use crate::files::db::file_version::FileVersionDto;
use crate::files::file_system::FileSystemManager;
use crate::files::files_repository::FileRepository;
use crate::files::service_result::{ServiceError, ServiceResult};

#[async_trait]
pub(crate) trait VersionService {
    async fn version_file_node(&self, node: &FileNodeDto) -> ServiceResult<()>;
}

pub(crate) struct VersionServiceImpl {
    user_id: i64,
    pool: Arc<Pool>,
    db: Arc<dyn DatabaseAccess + Send + Sync>,
    file_system: Arc<dyn FileSystemManager + Send + Sync>,
}

impl VersionServiceImpl {
    pub(crate) fn new(user_id: i64,
                      pool: Arc<Pool>,
                      db: Arc<dyn DatabaseAccess + Send + Sync>,
                      file_system: Arc<dyn FileSystemManager + Send + Sync>) -> Self {
        Self {
            user_id,
            pool,
            db,
            file_system,
        }
    }

    async fn insert_version_node(&self, file_version: &FileVersionDto) -> ServiceResult<()> {
        let sql = format!("insert into file_versions ({}) values ($1, $2, $3, $4, $5, $5);", FileVersionDto::column_list());
        self.db.execute(&self.pool, &sql, &[
            &file_version.id,
            &file_version.user_id,
            &file_version.node_version,
            &file_version.created_at,
            &file_version.node_size,
            &file_version.file_name,
        ]).await?;
        Ok(())
    }
}

#[async_trait]
impl VersionService for VersionServiceImpl {
    async fn version_file_node(&self, node: &FileNodeDto) -> ServiceResult<()> {
        if node.node_type != NODE_TYPE_FILE {
            return Err(ServiceError::new("VersionService", "Only files can be versioned!"));
        }
        // Generate name
        let version_file_name = Uuid::new_v4().as_hyphenated().to_string();
        let file_version = FileVersionDto::from(node, &version_file_name);

        // Copy file to versions folder
        self.file_system.copy_to_versions(&node.filesystem_path, &version_file_name).await?;
        self.insert_version_node(&file_version).await?;
        Ok(())
    }
}
