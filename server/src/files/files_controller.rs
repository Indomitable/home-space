use std::path::{Path, PathBuf};
use actix_multipart::Multipart;
use actix_web::{web, Responder, Result, HttpRequest, delete, get, put};
use deadpool_postgres::Pool;
use futures_util::stream::StreamExt as _;
use log::error;
use serde::Deserialize;

use home_space_contracts::files::{CreateNode, CreateFolderRequest, NODE_TYPE_FILE, NODE_TYPE_FOLDER};
use crate::response::*;
use crate::config::get_top_save_folder;
use crate::auth::AuthContext;
use super::file_system::*;
use super::files_repository as repo;

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
    if let Ok(node) = repo::fetch_node(&pool, id, user.user_id).await {
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
#[put("/upload_file")]
pub async fn upload_file(pool: web::Data<Pool>, user: AuthContext, mut body: Multipart) -> Result<impl Responder> {
    let user_id = user.user_id;
    let mut parent_id= -1_i64;
    if let Some(item) = body.next().await {
        let mut field = match item {
            Ok(field) => field,
            Err(err) => {
                error!("{:?}", err);
                return error_bad_request();
            }
        };
        if !field.name().eq_ignore_ascii_case("parent_id") {
            return error_bad_request();
        }
        let value = field.next().await.unwrap().unwrap();
        parent_id = String::from_utf8(value.to_vec()).unwrap().parse().unwrap();
    }
    if parent_id == -1 {
        // parent id not set
        return error_bad_request();
    }

    if let Some(item) = body.next().await {
        let mut field = match item {
            Ok(field) => field,
            Err(err) => {
                error!("{:?}", err);
                return error_bad_request();
            }
        };
        if !field.name().eq_ignore_ascii_case("file") {
            return error_bad_request();
        }
        let file_name = field.content_disposition().get_filename().expect("File should have file name").to_owned();

        let output = get_save_path(&pool, parent_id, user_id, &file_name).await;
        let mut size = 0_i64;
        {
            let output = output.clone();
            let mut f = create_file(output).await?;
            while let Some(chunk) = field.next().await {
                if let Ok(bytes) = chunk {
                    size = size + bytes.len() as i64;
                    f = append_file(f, bytes).await?;
                }
            }
        }

        let filesystem_path = output.to_str().unwrap().to_owned(); // Clone to filesystem_path because output will be moved on file create

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

async fn get_save_path(pool: &web::Data<Pool>, parent_id: i64, user_id: i64, name: &str) -> PathBuf {
    let node = repo::fetch_node(&pool, parent_id, user_id).await;
    let path = node.map_or(get_top_save_folder(user_id), |n| n.filesystem_path);
    Path::new(&path).join(name).to_path_buf()
}
