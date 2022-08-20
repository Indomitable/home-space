use std::path::{Path, PathBuf};
use actix_web::HttpRequest;
use actix_web::{web, Responder, Result, delete, get, put};
use futures_util::TryStreamExt;

use home_space_contracts::files::{CreateNode, CreateFolderRequest, NODE_TYPE_FILE, NODE_TYPE_FOLDER};
use crate::files::trash_mover::TrashMover;
use crate::files::versions_mover::VersionsMover;
use crate::ioc::container::Contrainer;
use crate::response::*;
use crate::auth::AuthContext;
use crate::sorting::Sorting;
use super::paths_manager::PathManager;
use super::{file_system::*};
use super::files_repository::{FileRepository, FileNodeDto};

///
/// Method: GET 
/// `/api/files/nodes/{parent_id}`
/// parent_id = 0 => top node
/// parent_id > 0 -> sub nodes
/// 
#[get("/nodes/{parent_id}")]
pub(crate) async fn get_nodes(provider: web::Data<Contrainer>, path: web::Path<i64>, query: web::Query<Sorting>, user: AuthContext) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    let sorting: Sorting = query.into_inner();
    let repo = provider.get_file_repository();
    if let Ok(nodes) = repo.get_file_list(parent_id, user.user_id, &sorting).await {
        return Ok(web::Json(nodes));
    }
    error_internal_server_error()
}

///
/// Downloads file with id.
/// 
#[get("/file/{id}")]
pub async fn get_file(provider: web::Data<Contrainer>, path: web::Path<i64>, user: AuthContext) -> Result<impl Responder> {
    let id = path.into_inner();
    let repo = provider.get_file_repository();
    if let Ok(node) = repo.get_node(id, user.user_id).await {
        if node.node_type == NODE_TYPE_FILE {
            let file = actix_files::NamedFile::open_async(node.filesystem_path).await?;
            return Ok(file);
        }
    }
    error_not_found() // file not found
}

///
/// Method: PUT
/// Creates folder, when parent_id is 0 then it is top folder.
#[put("/create-folder")]
pub async fn create_folder(provider: web::Data<Contrainer>, user: AuthContext, body: web::Json<CreateFolderRequest>) -> Result<impl Responder> {
    let CreateFolderRequest { parent_id, name } = body.into_inner();
    let repo = provider.get_file_repository();
    let path_manager = provider.get_path_manager();
    let node_path = get_path(&repo, &path_manager, parent_id, user.user_id, &name).await;
    let file_node = FileNodeDto {
        id: 0,
        user_id: user.user_id,
        title: name,
        parent_id: Some(parent_id),
        node_type: NODE_TYPE_FOLDER,
        filesystem_path: node_path.relative_path,
        mime_type: "inode/directory".to_owned(),
        modified_at: chrono::Utc::now(),
        node_size: 0
    };
    match repo.add_node(file_node).await {
        Ok(node_id) => {
            match execute_file_system_operation(move || create_dir(node_path.absolute_path)).await {
                Ok(_) => return created(CreateNode { id: node_id }),
                _ => {
                    // When there is a problem creating folder delete created node.                    
                    let _ = repo.permanent_delete(node_id, user.user_id).await;
                }
            }
        },
        _ => {
            log::error!("Error creating folder");
        }
    }
    error_internal_server_error()
}

///
/// Method: DELETE
/// `/api/files/delete_node/{id}` delete node - if folder delete all contents
/// 
#[delete("/delete-node/{id}")]
pub async fn delete_node(provider: web::Data<Contrainer>, path: web::Path<i64>, user: AuthContext) -> Result<impl Responder> {
    let id = path.into_inner();
    let repo = provider.get_file_repository();
    if let Ok(node) = repo.get_node(id, user.user_id).await {
        if let Ok(_) = repo.move_to_trash(id, node.node_type, user.user_id).await {
            // delete file from the file system only if it was deleted.
            let trash_mover = provider.get_trash_mover(user.user_id);
            if node.node_type == NODE_TYPE_FILE {
                if execute_file_system_operation(move || trash_mover.move_file_to_trash(node.filesystem_path.into())).await.is_ok() {
                    return no_content()
                }
            } else {
                if execute_file_system_operation(move || trash_mover.move_dir_to_trash(node.filesystem_path.into())).await.is_ok() {
                    return no_content()
                }
            }
        }
    }
    error_not_found()
}


// #[post("/move_node/{id}/{parent_id}")]
// pub async fn move_node(request: HttpRequest, provider: web::Data<Contrainer>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
//     todo!("Implement move node");
//     // let id = path.into_inner();
//     no_content()
// }

// #[post("/copy_node/{id}/{parent_id}")]
// pub async fn copy_node(request: HttpRequest, provider: web::Data<Contrainer>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
//     todo!("Implement copy node");
//     // let id = path.into_inner();
//     created()
// }

///
/// Method: PUT 
/// `/api/files/upload-file/0` upload file in top folder
/// `/api/files/upload-file/{parent_id}` parent_id > 0 upload file in sub folder
/// Creates a new file or if file exits it creates a new version of it.
///
#[put("/upload-file")]
pub async fn upload_file(provider: web::Data<Contrainer>, request: HttpRequest, user: AuthContext, body: web::Payload) -> Result<impl Responder> {
    let user_id = user.user_id;
    let file_name = read_string_header(&request, "X-FILE-NAME").expect("Request should have File name present");
    let parent_id = read_int_header(&request, "X-PARENT-ID").expect("Request should have parent id");
    let repo = provider.get_file_repository();

    let node = repo.get_node_by_name(parent_id, user_id, file_name.clone().into()).await;
    if let Ok(Some(node)) = node {
        let source_path = Path::new(&node.filesystem_path);
        let versions_mover = provider.get_versions_mover(user_id);
        if let Ok(version_name) = versions_mover.move_to_versions(&source_path.to_path_buf()) {
            let written_bytes = write_request_to_file(&source_path.to_path_buf(), body).await?;

            let file_node = FileNodeDto {
                id: node.id,
                user_id: user.user_id,
                title: file_name,
                parent_id: Some(parent_id),
                node_type: NODE_TYPE_FILE,
                filesystem_path: node.filesystem_path.to_owned(),
                mime_type: "text/plain".to_owned(),
                modified_at: chrono::Utc::now(),
                node_size: written_bytes
            };
            if repo.update_node_version(&node, version_name, &file_node).await.is_ok() {
                return created(CreateNode { id: node.id });
            }
        }
    } else {
        let path_manager = provider.get_path_manager();
        let node_path = get_path(&repo, &path_manager, parent_id, user_id, &file_name).await;
        let written_bytes = write_request_to_file(&node_path.absolute_path, body).await?;

        let file_node = FileNodeDto {
            id: 0,
            user_id: user.user_id,
            title: file_name,
            parent_id: Some(parent_id),
            node_type: NODE_TYPE_FILE,
            filesystem_path: node_path.relative_path,
            mime_type: "text/plain".to_owned(),
            modified_at: chrono::Utc::now(),
            node_size: written_bytes
        };
        if let Ok(node_id) = repo.add_node(file_node).await {
            return created(CreateNode { id: node_id });
        }
    }
    error_bad_request()
}

async fn write_request_to_file(output: &PathBuf, mut body: web::Payload) -> std::result::Result<i64, Box<dyn std::error::Error>> {
    let mut size = 0_i64;
    {
        let output = output.clone();
        let mut f = execute_file_system_operation(move || create_file(output)).await?;
        while let Some(chunk) = body.try_next().await? {
            size = size + chunk.len() as i64;
            f = execute_file_system_operation(move || append_file(f, chunk)).await?;
        }
    }
    return Ok(size);
}

#[get("/parents/{parent_id}")]
pub async fn get_parents(provider: web::Data<Contrainer>, path: web::Path<i64>, user: AuthContext )-> Result<impl Responder>  {
    let parent_id = path.into_inner();
    if parent_id == 0 {
        return json(Vec::new());
    } else {
        let repo = provider.get_file_repository();
        match repo.get_parent_nodes(parent_id, user.user_id).await {
            Ok(nodes) => json(nodes),
            Err(e) => {
                log::error!("Error getting parents: {:?}", e);
                error_internal_server_error()
            }
        }
        
    }
}

struct NodePaths {
    absolute_path: PathBuf,
    relative_path: String,
}

async fn get_path<R, PM>(repo: &R, path_manager: &PM, parent_id: i64, user_id: i64, name: &str) -> NodePaths
where R: FileRepository,
      PM: PathManager {
    let top_path = path_manager.get_top_save_folder(user_id);
    return if parent_id == 0 {        
        NodePaths {
            absolute_path: top_path.join(name),
            relative_path: name.to_owned()
        }
    } else {
        let node = repo.get_node(parent_id, user_id).await.unwrap();
        NodePaths {
            absolute_path: top_path.join(&node.filesystem_path).join(name),
            relative_path: PathBuf::from(&node.filesystem_path).join(name).to_str().unwrap().to_owned()
        }
    }
}
