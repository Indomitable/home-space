use std::fs::File;
use std::sync::Arc;
use actix_files::NamedFile;
use home_space_contracts::files::{DisplayFileNode, NODE_TYPE_FILE, NODE_TYPE_FOLDER, ParentNode};
use crate::files::db::file_node::FileNodeDto;
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

    pub(crate) async fn download_nodes(&self, ids: Vec<i64>) -> ServiceResult<NamedFile> {
        if ids.len() == 1 {
            let id = *ids.get(0).unwrap();
            self.get_single_node(id).await
        } else {
            self.get_multiple_nodes(ids).await
        }
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

    async fn get_file(&self, node: &FileNodeDto) -> ServiceResult<NamedFile> {
        let absolute_path = self.path_manager.get_absolute_path(&node);
        let file = NamedFile::open_async(absolute_path).await?;
        Ok(file)
    }

    async fn get_single_node(&self, id: i64) -> ServiceResult<NamedFile> {
        let node = self.file_repository.get_node(id).await?;
        if node.node_type == NODE_TYPE_FOLDER {
            self.get_folder(&node).await
        } else {
            self.get_file(&node).await
        }
    }

    async fn get_multiple_nodes(&self, ids: Vec<i64>) -> ServiceResult<NamedFile> {
        let temp_location = self.path_manager.get_temp_file(self.user_id);
        let f = File::create(&temp_location)?;
        let mut builder = tar::Builder::new(f);
        for id in ids {
            let node = self.file_repository.get_node(id).await?;
            let absolute_path = self.path_manager.get_absolute_path(&node);
            if node.node_type == NODE_TYPE_FILE {
                builder.append_path_with_name(absolute_path, &node.title)?;
            } else {
                builder.append_dir_all(&node.title, absolute_path)?;
            }
        }
        builder.finish()?;
        let named_file = NamedFile::from_file(File::open(&temp_location)?, "archive.tar")?;
        Ok(named_file)
    }

    async fn get_folder(&self, node: &FileNodeDto) -> ServiceResult<NamedFile> {
        // Since the folder can be bigger and the memory can be limited.
        // Create a tar archive in a temp folder and return file.
        // We need to clean files after downloading them.
        let temp_location = self.path_manager.get_temp_file(self.user_id);
        let f = File::create(&temp_location)?;
        let mut builder = tar::Builder::new(f);
        let absolute_path = self.path_manager.get_absolute_path(node);
        builder.append_dir_all(&node.title, &absolute_path)?;
        builder.finish()?;
        let named_file = NamedFile::from_file(File::open(&temp_location)?, format!("{}.tar", node.title))?;
        Ok(named_file)
    }
}
