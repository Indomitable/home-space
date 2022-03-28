use std::path::{Path, PathBuf};
use actix_web::HttpRequest;
use actix_web::{web, Responder, Result, delete, get, put};
use deadpool_postgres::Pool;
use futures_util::TryStreamExt;
use serde::Deserialize;

use home_space_contracts::files::{CreateNode, CreateFolderRequest, NODE_TYPE_FILE, NODE_TYPE_FOLDER};
use crate::response::*;
use crate::config::get_top_save_folder;
use crate::auth::AuthContext;
use super::file_system::*;
use super::files_repository::{self as repo, FileNodeDto};

#[derive(Deserialize)]
pub(crate) struct GetNodesQuery {
    parent_id: i64
}

///
/// Method: GET 
/// `/api/files/get_nodes/{parent_id}`
/// parent_id = 0 => top node
/// parent_id > 0 -> sub nodes
/// 
#[get("/get_nodes")]
pub(crate) async fn get_nodes(pool: web::Data<Pool>, query: web::Query<GetNodesQuery>, user: AuthContext) -> Result<impl Responder> {
    let parent_id = query.parent_id;
    if let Ok(nodes) = repo::get_file_list(&pool, parent_id, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_internal_server_error()
}

///
/// Downloads file with id.
/// 
#[get("/get_file/{id}")]
pub async fn get_file(pool: web::Data<Pool>, path: web::Path<i64>, user: AuthContext) -> Result<impl Responder> {
    let id = path.into_inner();
    if let Ok(node) = repo::get_node(&pool, id, user.user_id).await {
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
#[put("/create_folder")]
pub async fn create_folder(pool: web::Data<Pool>, user: AuthContext, body: web::Json<CreateFolderRequest>) -> Result<impl Responder> {
    let CreateFolderRequest { parent_id, name } = body.into_inner();
    let path = get_save_path(&pool, parent_id, user.user_id, &name).await;
    let file_node = repo::FileNodeDto {
        id: 0,
        user_id: user.user_id,
        title: name,
        parent_id: Some(parent_id),
        node_type: NODE_TYPE_FOLDER,
        filesystem_path: path.to_str().unwrap().to_owned(),
        mime_type: "inode/directory".to_owned(),
        modified_at: chrono::Utc::now(),
        node_size: 0
    };
    match repo::add_node(&pool, file_node).await {
        Ok(node_id) => {
            match execute_file_system_operation(move || create_dir(path)).await {
                Ok(_) => return created(CreateNode { id: node_id }),
                _ => {
                    // When there is a problem creating folder delete created node.                    
                    let _ = repo::permanent_delete(&pool, node_id, user.user_id).await;
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
#[delete("/delete_node/{id}")]
pub async fn delete_node(pool: web::Data<Pool>, path: web::Path<i64>, user: AuthContext) -> Result<impl Responder> {
    let id = path.into_inner();
    if let Ok(node) = repo::get_node(&pool, id, user.user_id).await {
        if node.node_type == NODE_TYPE_FILE {
            if let Ok(1) = repo::move_to_trash(&pool, id, node.node_type, user.user_id).await {
                // delete file from the file system only if it was deleted.
                if execute_file_system_operation(move || delete_file(node.filesystem_path.into())).await.is_ok() {
                    return no_content()
                }
            }
        }
    }
    error_not_found()
}


// #[post("/move_node/{id}/{parent_id}")]
// pub async fn move_node(request: HttpRequest, pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
//     todo!("Implement move node");
//     // let id = path.into_inner();
//     no_content()
// }

// #[post("/copy_node/{id}/{parent_id}")]
// pub async fn copy_node(request: HttpRequest, pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
//     todo!("Implement copy node");
//     // let id = path.into_inner();
//     created()
// }

///
/// Method: PUT 
/// `/api/files/upload_file/0` upload file in top folder
/// `/api/files/upload_file/{parent_id}` parent_id > 0 upload file in sub folder
/// Creates a new file or if file exits it creates a new version of it.
///
#[put("/upload_file")]
pub async fn upload_file(pool: web::Data<Pool>, request: HttpRequest, user: AuthContext, body: web::Payload) -> Result<impl Responder> {
    let user_id = user.user_id;
    let file_name = read_string_header(&request, "X-FILE-NAME").expect("Request should have File name present");
    let parent_id = read_int_header(&request, "X-PARENT-ID").expect("Request should have parent id");

    if let Some(node) = check_existing_file(&pool, &file_name, &parent_id, &user_id).await {
        let source_path = Path::new(&node.filesystem_path);
        if let Ok(version_name) = move_to_versions(&source_path.to_path_buf(), user_id) {
            let written_bytes = write_request_to_file(&source_path.to_path_buf(), body).await?;

            let file_node = repo::FileNodeDto {
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
            if repo::update_node_version(&pool, &node, version_name, &file_node).await.is_ok() {
                return created(CreateNode { id: node.id });
            }

        }
    } else {
        let output = get_save_path(&pool, parent_id, user_id, &file_name).await;
        let written_bytes = write_request_to_file(&output, body).await?;

        let file_node = repo::FileNodeDto {
            id: 0,
            user_id: user.user_id,
            title: file_name,
            parent_id: Some(parent_id),
            node_type: NODE_TYPE_FILE,
            filesystem_path: output.to_str().unwrap().to_owned(),
            mime_type: "text/plain".to_owned(),
            modified_at: chrono::Utc::now(),
            node_size: written_bytes
        };
        if let Ok(node_id) = repo::add_node(&pool, file_node).await {
            return created(CreateNode { id: node_id });
        }
    }
    error_bad_request()
}

async fn check_existing_file(pool: &web::Data<Pool>, file_name: &str, parent_id: &i64, user_id: &i64) -> Option<FileNodeDto> {
    let node = repo::get_node_by_name(&pool, parent_id, user_id, file_name.into()).await;
    node.map_or(None, |n| n)
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
pub async fn get_parents(pool: web::Data<Pool>, path: web::Path<i64>, user: AuthContext )-> Result<impl Responder>  {
    let parent_id = path.into_inner();
    if parent_id == 0 {
        return json(Vec::new());
    } else {
        match repo::get_parent_nodes(&pool, parent_id, user.user_id).await {
            Ok(nodes) => json(nodes),
            Err(e) => {
                log::error!("Error getting parents: {:?}", e);
                error_internal_server_error()
            }
        }
        
    }
}

async fn get_save_path(pool: &web::Data<Pool>, parent_id: i64, user_id: i64, name: &str) -> PathBuf {
    let node = repo::get_node(&pool, parent_id, user_id).await;
    let path = node.map_or(get_top_save_folder(user_id), |n| n.filesystem_path);
    Path::new(&path).join(name).to_path_buf()
}
