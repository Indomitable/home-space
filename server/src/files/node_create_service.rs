use std::path::PathBuf;
use std::sync::Arc;
use actix_web::web::Bytes;
use futures_util::Stream;
use home_space_contracts::files::{NODE_TYPE_FILE, NODE_TYPE_FOLDER};
use crate::results::service_result::{ServiceError, ServiceResult};
use crate::files::db::file_node::FileNodeDto;
use crate::files::file_system::FileSystemManager;
use crate::files::files_repository::FileRepository;
use crate::files::paths_manager::PathManager;
use crate::files::version_service::VersionService;

pub(crate) struct NodeCreateService {
    user_id: i64,
    path_manager: Arc<PathManager>,
    file_repository: Arc<FileRepository>,
    file_system: Arc<FileSystemManager>,
    version_service: Arc<VersionService>,
}

impl NodeCreateService {
    pub(crate) fn new(
        user_id: i64,
        path_manager: &Arc<PathManager>,
        file_repository: &Arc<FileRepository>,
        file_system: &Arc<FileSystemManager>,
        version_service: &Arc<VersionService>,
    ) -> Self {
        Self {
            user_id,
            path_manager: Arc::clone(path_manager),
            file_repository: Arc::clone(file_repository),
            file_system: Arc::clone(file_system),
            version_service: Arc::clone(version_service),
        }
    }

    pub(crate) async fn create_folder_node(&self, parent_id: i64, name: &str) -> ServiceResult<i64> {
        let node_paths = self.get_path(parent_id, name).await?;
        let file_node = FileNodeDto {
            id: 0,
            user_id: self.user_id,
            title: name.to_owned(),
            parent_id: Some(parent_id),
            node_type: NODE_TYPE_FOLDER,
            filesystem_path: node_paths.relative_path,
            mime_type: "inode/directory".to_owned(),
            modified_at: chrono::Utc::now(),
            node_size: 0,
            node_version: 1,
        };
        self.file_system.create_dir(&node_paths.absolute_path).await?;
        let node_id = self.file_repository.add_node(&file_node).await?;
        Ok(node_id)
    }

    pub(crate) async fn create_file_node<TStream, TError>(&self, parent_id: i64, name: String, contents: TStream) -> ServiceResult<i64>
        where TStream: Stream<Item=Result<Bytes, TError>> + Unpin {
        let same_node = self.file_repository.get_node_by_title(parent_id, &name).await;
        let node_paths = self.get_path(parent_id, &name).await?;
        match same_node {
            Ok(Some(same_title_node)) => {
                if same_title_node.node_type != NODE_TYPE_FILE {
                    // We can't store on the filesystem file and folder with same name.
                    return Err(ServiceError::new("NodeCreateService", "Destination has node with same name, but it is a folder."));
                }
                self.version_service.version_file_node(&same_title_node).await?;
                let file_size = self.file_system.write_stream_to_file(&PathBuf::from(&node_paths.absolute_path), contents).await?;
                let file_node = FileNodeDto {
                    id: 0,
                    user_id: self.user_id,
                    title: name.to_owned(),
                    parent_id: Some(parent_id),
                    node_type: NODE_TYPE_FILE,
                    filesystem_path: node_paths.relative_path,
                    mime_type: "application/octed-stream".to_owned(),
                    modified_at: chrono::Utc::now(),
                    node_size: file_size as i64,
                    node_version: same_title_node.node_version + 1,
                };
                self.file_repository.update_node(&same_title_node, &file_node).await?;
                Ok(same_title_node.id)
            },
            _ => {
                let file_size = self.file_system.write_stream_to_file(&PathBuf::from(&node_paths.absolute_path), contents).await?;
                let file_node = FileNodeDto {
                    id: 0,
                    user_id: self.user_id,
                    title: name.to_owned(),
                    parent_id: Some(parent_id),
                    node_type: NODE_TYPE_FILE,
                    filesystem_path: node_paths.relative_path,
                    mime_type: "application/octed-stream".to_owned(),
                    modified_at: chrono::Utc::now(),
                    node_size: file_size as i64,
                    node_version: 1,
                };
                let id = self.file_repository.add_node(&file_node).await?;
                Ok(id)
            }
        }
    }

    async fn get_path(&self, parent_id: i64, name: &str) -> ServiceResult<NodePaths> {
        let top_path = self.path_manager.get_top_save_folder(self.user_id);
        if parent_id == 0 {
            Ok(NodePaths {
                absolute_path: self.path_manager.path_to_string(&top_path.join(name)),
                relative_path: name.to_owned()
            })
        } else {
            let node = self.file_repository.get_node(parent_id).await?;
            Ok(NodePaths {
                absolute_path: self.path_manager.path_to_string(&top_path.join(&node.filesystem_path).join(name)),
                relative_path: self.path_manager.path_to_string(&PathBuf::from(&node.filesystem_path).join(name))
            })
        }
    }
}

struct NodePaths {
    absolute_path: String,
    relative_path: String,
}

