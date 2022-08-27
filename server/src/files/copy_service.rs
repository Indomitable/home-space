use async_recursion::async_recursion;
use std::sync::Arc;
use home_space_contracts::files::{NODE_TYPE_FILE, NODE_TYPE_FOLDER};
use crate::db::DatabaseAccess;
use crate::files::db::file_node::FileNodeDto;
use crate::files::file_system::FileSystemManager;
use crate::files::files_repository::FileRepository;
use crate::files::service_result::{ServiceError, ServiceResult};
use crate::files::version_service::VersionService;

pub(crate) struct CopyService {
    user_id: i64,
    db: Arc<DatabaseAccess>,
    file_repository: Arc<FileRepository>,
    file_system: Arc<FileSystemManager>,
    version_service: Arc<VersionService>,
}

impl CopyService {
    pub(crate) fn new(user_id: i64,
                      db: &Arc<DatabaseAccess>,
                      file_repository: &Arc<FileRepository>,
                      file_system: &Arc<FileSystemManager>,
                      version_service: &Arc<VersionService>,) -> Self {
        Self {
            user_id,
            db: Arc::clone(db),
            file_repository: Arc::clone(file_repository),
            file_system: Arc::clone(file_system),
            version_service: Arc::clone(version_service)
        }
    }

    pub(crate) async fn copy_node(&self, source_node_id: i64, destination_parent_id: i64) -> ServiceResult<()> {
        let source_node = self.file_repository.get_node(source_node_id).await?;
        let destination_parent_node = self.file_repository.get_node(destination_parent_id).await?;
        if source_node.node_type == NODE_TYPE_FILE {
            self.copy_file_node(&source_node, &destination_parent_node).await
        } else {
            self.copy_folder_node_recursive(&source_node, &destination_parent_node).await
        }
    }

    async fn copy_file_node(&self, source_file_node: &FileNodeDto, destination_parent_node: &FileNodeDto) -> ServiceResult<()> {
        match self.file_repository.get_node_by_title(destination_parent_node.id, &source_file_node.title).await {
            Ok(Some(same_title_node)) => {
                if same_title_node.node_type != NODE_TYPE_FILE {
                    // We can't store on the filesystem file and folder with same name.
                    return Err(ServiceError::new("CopyService", "Destination has node with same name, but it is a folder."));
                }
                self.version_service.version_file_node(&same_title_node).await?;
                // We have already the path so copy to it.
                self.file_system.copy_node_to_destination(&source_file_node.filesystem_path, &same_title_node.filesystem_path).await?;
                // The node is inserted. Update it.
                self.file_repository.update_node(&same_title_node, source_file_node).await?;
            },
            _ => {
                // No node with same title in the destination.
                // Create the destination node where filesystem_path is combination of parent path and source title.
                let destination_node = FileNodeDto::copy(source_file_node, destination_parent_node);
                self.file_system.copy_node_to_destination(&source_file_node.filesystem_path, &destination_node.filesystem_path).await?;
                self.file_repository.add_node(&destination_node).await?;
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
        for child_node in child_nodes.iter() {
            if child_node.node_type == NODE_TYPE_FILE {
                self.copy_file_node(child_node, &current_parent).await?;
            } else {
                self.copy_folder_node_recursive(child_node, &current_parent).await?;
            }
        }
        Ok(())
    }

    async fn copy_folder_node(&self, source_folder_node: &FileNodeDto, destination_parent_node: &FileNodeDto) -> ServiceResult<FileNodeDto> {
        match self.file_repository.get_node_by_title(destination_parent_node.id, &source_folder_node.title).await {
            Ok(Some(same_title_node)) => {
                if same_title_node.node_type != NODE_TYPE_FOLDER {
                    // We can't store on the filesystem file and folder with same name.
                    return Err(ServiceError::new("CopyService", "Destination has node with same name, but it is a file."));
                }
                // Do nothing we have already the folder
                Ok(same_title_node)
            },
            _ => {
                // No node with same title in the destination.
                // Create the destination node where filesystem_path is combination of parent path and source title.
                let destination_node = FileNodeDto::copy(source_folder_node, destination_parent_node);
                self.file_system.create_dir(&destination_node.filesystem_path).await?;
                self.file_repository.add_node(&destination_node).await?;
                Ok(destination_node.clone())
            }
        }
    }
}
