use std::sync::Arc;
use actix_files::NamedFile;
use home_space_contracts::files::{DisplayFileNode, NODE_TYPE_FOLDER, ParentNode};
use crate::files::files_repository::FileRepository;
use crate::files::paths_manager::PathManager;
use crate::files::service_result::{ServiceError, ServiceResult};
use crate::sorting::Sorting;

pub(crate) struct NodeProvideService {
    user_id: i64,
    file_repository: Arc<FileRepository>,
    path_manager: Arc<PathManager>
}

impl NodeProvideService {
    pub(crate) fn new(user_id: i64,
                      file_repository: &Arc<FileRepository>,
                      path_manager: &Arc<PathManager>) -> Self {
        Self {
            user_id,
            file_repository: Arc::clone(file_repository),
            path_manager: Arc::clone(path_manager),
        }
    }

    pub(crate) async fn list_nodes(&self, parent_id: i64, sorting: &Sorting) -> ServiceResult<Vec<DisplayFileNode>> {
        let nodes = self.file_repository.get_file_list(parent_id, sorting)
            .await?
            .iter()
            .map(|(f, favorite)| DisplayFileNode {
                id: f.id,
                title: f.title.clone(),
                parent_id: f.parent_id,
                node_type: f.node_type,
                mime_type: f.mime_type.clone(),
                modified_at: f.modified_at.to_rfc3339(),
                node_size: f.node_size,
                node_version: f.node_version,
                is_favorite: *favorite
            })
            .collect();
        Ok(nodes)
    }

    pub(crate) async fn get_file(&self, id: i64) -> ServiceResult<NamedFile> {
        let node = self.file_repository.get_node(id).await?;
        if node.node_type == NODE_TYPE_FOLDER {
            return Err(ServiceError::new("NodeProvideService", "Can get only file nodes"));
        }
        let absolute_path = self.path_manager.get_top_save_folder(self.user_id).join(node.filesystem_path);
        let file = NamedFile::open_async(absolute_path).await?;
        Ok(file)
    }

    pub(crate) async fn get_parent_nodes(&self, id: i64) -> ServiceResult<Vec<ParentNode>> {
        if id == 0 {
            // If requested node is the top one no need to call the db.
            return Ok(Vec::new())
        }
        let nodes = self.file_repository.get_parent_nodes(id)
            .await?
            .iter()
            .map(|f| ParentNode {
                id: f.id,
                title: f.title.clone(),
            })
            .collect();
        Ok(nodes)
    }
}
