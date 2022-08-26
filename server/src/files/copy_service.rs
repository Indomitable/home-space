use std::sync::Arc;
use async_trait::async_trait;
use async_recursion::async_recursion;
use deadpool_postgres::Pool;
use home_space_contracts::files::{NODE_TYPE_FILE, NODE_TYPE_FOLDER};
use crate::db::DatabaseAccess;
use crate::files::db::DbModel;
use crate::files::db::file_node::FileNodeDto;
use crate::files::file_system::FileSystemManager;
use crate::files::files_repository::FileRepository;
use crate::files::service_result::{ServiceError, ServiceResult};
use crate::files::version_service::VersionService;

#[async_trait]
pub(crate) trait CopyService {
    async fn copy_node(&self, source_node_id: i64, destination_parent_id: i64) -> ServiceResult<()>;
}

pub(crate) struct CopyServiceImpl {
    user_id: i64,
    pool: Arc<Pool>,
    db: Arc<dyn DatabaseAccess + Send + Sync>,
    file_repository: Arc<dyn FileRepository + Send + Sync>,
    file_system: Arc<dyn FileSystemManager + Send + Sync>,
    version_service: Arc<dyn VersionService + Send + Sync>,
}

impl CopyServiceImpl {
    pub(crate) fn new(user_id: i64,
                      pool: Arc<Pool>,
                      db: Arc<dyn DatabaseAccess + Send + Sync>,
                      file_repository: Arc<dyn FileRepository + Send + Sync>,
                      file_system: Arc<dyn FileSystemManager + Send + Sync>,
                      version_service: Arc<dyn VersionService + Send + Sync>) -> Self {
        Self {
            user_id,
            pool,
            db,
            file_repository,
            file_system,
            version_service
        }
    }


    async fn copy_file_node(&self, source_file_node: &FileNodeDto, destination_parent_node: &FileNodeDto) -> ServiceResult<()> {
        match self.get_same_title_node_at_destination(source_file_node, destination_parent_node.id).await? {
            None => {
                // No node with same title in the destination.
                // Create the destination node where filesystem_path is combination of parent path and source title.
                let destination_node = FileNodeDto::copy(source_file_node, destination_parent_node);
                self.file_system.copy_node_to_destination(&source_file_node.filesystem_path, &destination_node.filesystem_path).await?;
                self.file_repository.add_node(&destination_node).await?;
            }
            Some(same_title_node) => {
                if same_title_node.node_type != NODE_TYPE_FILE {
                    // We can't store on the filesystem file and folder with same name.
                    return Err(ServiceError::new("CopyService", "Destination has node with same name, but it is a folder."));
                }
                self.version_service.version_file_node(&same_title_node).await?;
                // We have already the path so copy to it.
                self.file_system.copy_node_to_destination(&source_file_node.filesystem_path, &same_title_node.filesystem_path).await?;
                // The node is inserted. Update it.
                self.file_repository.update_node(&same_title_node, source_file_node).await?;
            }
        }
        Ok(())
    }

    #[async_recursion]
    async fn copy_folder_node_recursive(&self, source_folder_node: &FileNodeDto, destination_parent_node: &FileNodeDto) -> ServiceResult<()> {
        // Copy current node item to the destination
        let current_parent = self.copy_folder_node(source_folder_node, destination_parent_node).await?;
        // Get all children
        let child_nodes = self.file_repository.get_child_nodes(source_folder_node.id).await?;

        // let destination_folder_path = PathBuf::from(&destination_parent_node.filesystem_path);
        // let mut current_parent = destination_parent_node.clone();
        for child_node in child_nodes.iter() {
            // let destination_path = destination_folder_path.join(
            //     PathBuf::from(&source_node.filesystem_path)
            //     .strip_prefix(&source_folder_node.filesystem_path)
            //     .expect("Node path should start with source folder path")
            // ); // Replace source path with destination path.
            // let destination_parent_path = destination_path
            //     .parent()
            //     .expect("The destination should have a parent path")
            //     .to_str()
            //     .expect("Path should be UTF-8");
            // // when we copy file we should already copied the parent.
            // let parent = self.file_repository.get_node_by_path(destination_parent_path).await?;
            if child_node.node_type == NODE_TYPE_FILE {
                self.copy_file_node(child_node, &current_parent).await?;
            } else {
                self.copy_folder_node_recursive(child_node, &current_parent).await?;
            }
        }
        Ok(())
    }

    async fn copy_folder_node(&self, source_folder_node: &FileNodeDto, destination_parent_node: &FileNodeDto) -> ServiceResult<FileNodeDto> {
        match self.get_same_title_node_at_destination(source_folder_node, destination_parent_node.id).await? {
            None => {
                // No node with same title in the destination.
                // Create the destination node where filesystem_path is combination of parent path and source title.
                let destination_node = FileNodeDto::copy(source_folder_node, destination_parent_node);
                self.file_system.create_dir(&destination_node.filesystem_path).await?;
                self.file_repository.add_node(&destination_node).await?;
                Ok(destination_node.clone())
            }
            Some(same_title_node) => {
                if same_title_node.node_type != NODE_TYPE_FOLDER {
                    // We can't store on the filesystem file and folder with same name.
                    return Err(ServiceError::new("CopyService", "Destination has node with same name, but it is a file."));
                }
                // Do nothing we have already the folder
                Ok(same_title_node)
            }
        }
    }

    /// Get node with same title in the destination folder.
    async fn get_same_title_node_at_destination(&self, source_file_node: &FileNodeDto, destination_node_id: i64) -> ServiceResult<Option<FileNodeDto>> {
        let sql = format!(r#"select {}
            from file_nodes fn
            where fn.user_id = $1 and fn.parent_id = $2 and fn.title = $3"#, FileNodeDto::column_list());

        let node = self.db.query_opt(&self.pool, &sql, &[&self.user_id, &destination_node_id, &source_file_node.title])
            .await?
            .map(|row| FileNodeDto::read_node(&row));
        Ok(node)
    }
}

#[async_trait]
impl CopyService for CopyServiceImpl {
    async fn copy_node(&self, source_node_id: i64, destination_parent_id: i64) -> ServiceResult<()> {
        let source_node = self.file_repository.get_node(source_node_id).await?;
        let destination_parent_node = self.file_repository.get_node(destination_parent_id).await?;
        if source_node.node_type == NODE_TYPE_FILE {
            self.copy_file_node(&source_node, &destination_parent_node).await
        } else {
            self.copy_folder_node_recursive(&source_node, &destination_parent_node).await
        }
    }
}
