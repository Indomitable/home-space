use std::sync::Arc;
use uuid::Uuid;
use home_space_contracts::files::NODE_TYPE_FILE;
use crate::db::DatabaseAccess;
use crate::files::db::DbModel;
use crate::files::db::file_node::FileNodeDto;
use crate::files::db::file_version::FileVersionDto;
use crate::files::file_system::FileSystemManager;
use crate::files::service_result::{ServiceError, ServiceResult};

pub(crate) struct VersionService {
    user_id: i64,
    db: Arc<DatabaseAccess>,
    file_system: Arc<FileSystemManager>,
}

impl VersionService {
    pub(crate) fn new(user_id: i64,
                      db: &Arc<DatabaseAccess>,
                      file_system: &Arc<FileSystemManager>) -> Self {
        Self {
            user_id,
            db: Arc::clone(db),
            file_system: Arc::clone(file_system),
        }
    }

    pub(crate) async fn version_file_node(&self, node: &FileNodeDto) -> ServiceResult<()> {
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

    pub(crate) async fn file_versions(&self, id: i64) -> ServiceResult<Vec<FileVersionDto>> {
        let sql = format!(r#"select {} from file_versions fv where fn.user_id = $1 and fv.id = $2 order by fn.node_version desc"#,
                          FileVersionDto::column_list());
        let versions = self.db.query(&sql, &[&self.user_id, &id])
            .await?
            .iter()
            .map(FileVersionDto::read_node)
            .collect::<Vec<FileVersionDto>>();
        Ok(versions)
    }

    async fn insert_version_node(&self, file_version: &FileVersionDto) -> ServiceResult<()> {
        let sql = format!("insert into file_versions ({}) values ($1, $2, $3, $4, $5, $5);", FileVersionDto::column_list());
        self.db.execute(&sql, &[
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
