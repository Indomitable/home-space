use std::{path::{Path, PathBuf}, borrow::Cow};
use actix_web::{web, Responder, Result, HttpRequest, delete, get, put};
use deadpool_postgres::Pool;
use futures_util::TryStreamExt;
use serde::Deserialize;

use home_space_contracts::files::{FileNode, CreateNode, NODE_TYPE_FILE, NODE_TYPE_FOLDER};
use crate::{response::*, config::get_top_save_folder};
use crate::auth::AuthContext;
use super::file_system::*;
use super::files_repository::{self as repo, FileNodeDto};

///
/// Method: GET 
/// `/api/files/get_nodes/{parent_id}`
/// parent_id = 0 => top node
/// parent_id > 0 -> sub nodes
/// 
#[get("/get_nodes/{parent_id}")]
pub async fn get_nodes(pool: web::Data<Pool>, path: web::Path<i64>, user: AuthContext) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    if let Ok(nodes) = repo::fetch_nodes(&pool, parent_id, user.user_id).await {
        let nodes = nodes_mapper(nodes.iter());
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
    if let Ok(node) = repo::fetch_node(&pool, id, user.user_id).await {
        if node.node_type == NODE_TYPE_FILE {
            let file = actix_files::NamedFile::open_async(node.filesystem_path).await?;
            return Ok(file);
        }
    }
    error_not_found() // file not found
}

#[derive(Deserialize)]
pub struct CreateFolderRequestBody {
    pub name: String,
}

///
/// Method: PUT
/// `/api/files/create_folder/0` for top level folder
/// `/api/files/create_folder/{parent_id}` for sub folder
#[put("/create_folder/{parent_id}")]
pub async fn create_folder(pool: web::Data<Pool>, path: web::Path<i64>, user: AuthContext, body: web::Json<CreateFolderRequestBody>) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    let folder_name = Cow::from(&body.name);
    let path = get_save_path(&pool, parent_id, user.user_id, &folder_name).await;
    let file_node = repo::FileNodeDto {
        id: 0,
        user_id: user.user_id,
        title: folder_name.into_owned(),
        parent_id: Some(parent_id),
        node_type: NODE_TYPE_FOLDER,
        filesystem_path: path.to_str().unwrap().to_owned(),
        mime_type: "inode/directory".to_owned(),
        modified_at: chrono::Utc::now(),
        node_size: 0
    };
    match repo::add_node(&pool, file_node).await {
        Ok(node_id) => {
            match create_dir(path).await {
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
    if let Ok(node) = repo::fetch_node(&pool, id, user.user_id).await {
        if node.node_type == NODE_TYPE_FILE {
            if let Ok(1) = repo::move_to_trash(&pool, id, node.node_type, user.user_id).await {
                // delete file from the file system only if it was deleted.
                if delete_file(node.filesystem_path.into()).await.is_ok() {
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
#[put("/upload_file/{parent_id}")]
pub async fn upload_file(request: HttpRequest, pool: web::Data<Pool>, path: web::Path<i64>, user: AuthContext, mut body: web::Payload) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    if let Some(file_name) = get_file_name(&request) {
        let user_id = user.user_id;       
        let output = get_save_path(&pool, parent_id, user_id, &file_name).await;
        let filesystem_path = output.clone().to_str().unwrap().to_string(); // Clone to filesystem_path because output will be moved on file create
        let mut f = create_file(output).await?;
        let mut size = 0_i64;
        while let Some(bytes) = body.try_next().await? {
            //bytes.extend(item);
            size = size + bytes.len() as i64;
            f = append_file(f, bytes).await?;
            //web::block(move || f.write_all(&item).map(|_| f)).await??;
        }
        let file_node = repo::FileNodeDto {
            id: 0,
            user_id: user.user_id,
            title: file_name,
            parent_id: Some(parent_id),
            node_type: NODE_TYPE_FILE,
            filesystem_path,
            mime_type: "text/plain".to_owned(),
            modified_at: chrono::Utc::now(),
            node_size: size
        };
        if let Ok(node_id) = repo::add_node(&pool, file_node).await {
            return created(CreateNode { id: node_id });
        }
    }
    error_bad_request()
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

fn get_file_name(request: &HttpRequest) -> Option<String> {
    request.headers()
        .get("X-File-Name")
        .map(|h| percent_encoding::percent_decode(h.as_bytes())
                                .decode_utf8()
                                .map(|cow| cow.to_string())
                                .ok())
        .unwrap_or(None)
}

async fn get_save_path(pool: &web::Data<Pool>, parent_id: i64, user_id: i64, name: &str) -> PathBuf {
    let node = repo::fetch_node(&pool, parent_id, user_id).await;
    let path = node.map_or(get_top_save_folder(user_id), |n| n.filesystem_path);
    Path::new(&path).join(name).to_path_buf()
}

fn node_mapper(dto: &FileNodeDto) -> FileNode {
    FileNode {
        id: dto.id,
        title: dto.title.clone(),
        parent_id: dto.parent_id,
        node_type: dto.node_type,
        mime_type: dto.mime_type.clone(),
        modified_at: dto.modified_at.to_rfc3339(),
        node_size: dto.node_size
    }
}

fn nodes_mapper<'a, TInputIter>(dto_nodes: TInputIter) -> Vec<FileNode>
    where TInputIter: Iterator<Item = &'a FileNodeDto>  {
    dto_nodes.map(|dto| node_mapper(dto)).collect::<Vec<FileNode>>()
}