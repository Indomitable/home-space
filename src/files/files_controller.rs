use std::{path::{Path, PathBuf}, io::Write, borrow::Cow};
use actix_web::{web, Responder, Result, HttpRequest, delete, post, get, put};
use deadpool_postgres::Pool;
use futures_util::TryStreamExt;
use serde::Deserialize;

use crate::response::*;
use super::files_repository::{self as repo, NODE_TYPE_FILE};

#[derive(Deserialize)]
pub struct User {
    user_id: i64
}

///
/// Method: GET 
/// `/api/files/get_nodes/{parent_id}`
/// parent_id = 0 => top node
/// parent_id > 0 -> sub nodes
/// 
#[get("/get_nodes/{parent_id}")]
pub async fn get_nodes(pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    if let Ok(nodes) = repo::fetch_nodes(&pool, parent_id, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_server_unavailable()
}

///
/// Downloads file with id.
/// 
#[get("/get_file/{id}")]
pub async fn get_file(pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
    let id = path.into_inner();
    if let Ok(node) = repo::fetch_node(&pool, id, user.user_id).await {
        if node.node_type == repo::NODE_TYPE_FILE {
            let file = actix_files::NamedFile::open_async(node.filesystem_path).await?;
            return Ok(file);
        }
        return error_bad_request();
    }
    error_server_unavailable()
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
pub async fn create_folder(pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>, body: web::Json<CreateFolderRequestBody>) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    let folder_name = Cow::from(&body.name);
    let path = get_save_path(&pool, parent_id, user.user_id, &folder_name).await;
    let file_node = repo::FileNode {
        id: 0,
        user_id: user.user_id,
        title: folder_name.into_owned(),
        parent_id: parent_id,
        node_type: repo::NODE_TYPE_FOLDER,
        filesystem_path: path.to_str().unwrap().to_owned(),
        mime_type: None
    };
    if let Ok(a) = repo::add_node(&pool, file_node).await {
        if a == 1 {
            web::block(move || std::fs::create_dir(path) ).await??;
            return created();
        }
    }
    error_server_unavailable()
}

///
/// Method: DELETE
/// `/api/files/delete_node/{id}` delete node - if folder delete all contents
/// 
#[delete("/delete_node/{id}")]
pub async fn delete_node(pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
    let id = path.into_inner();
    if let Ok(node) = repo::fetch_node(&pool, id, user.user_id).await {
        if node.node_type == NODE_TYPE_FILE {
            if let Ok(affected) = repo::delete_node(&pool, id, node.node_type, user.user_id).await {
                if affected == 1 {
                    // delete file from the file system only if it was deleted.
                    web::block(move || std::fs::remove_file(node.filesystem_path) ).await??;
                    return no_content()
                }
            }
        }
    }
    not_found()
}


#[post("/move_node/{id}/{parent_id}")]
pub async fn move_node(request: HttpRequest, pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
    todo!("Implement move node");
    // let id = path.into_inner();
    no_content()
}

#[post("/copy_node/{id}/{parent_id}")]
pub async fn copy_node(request: HttpRequest, pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
    todo!("Implement copy node");
    // let id = path.into_inner();
    created()
}

///
/// Method: PUT 
/// `/api/files/upload_file/0` upload file in top folder
/// `/api/files/upload_file/{parent_id}` parent_id > 0 upload file in sub folder
/// Creates a new file or if file exits it creates a new version of it.
///
#[put("/upload_file/{parent_id}")]
pub async fn upload_file(request: HttpRequest, pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>, mut body: web::Payload) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    if let Some(file_name) = get_file_name(&request) {
        let user_id = user.user_id;       
        let output = get_save_path(&pool, parent_id, user_id, &file_name).await;
        let filesystem_path = output.clone().to_str().unwrap().to_string(); // Clone to filesystem_path because output will be moved on file create
        let mut f = web::block(|| std::fs::File::create(output)).await??;
        while let Some(item) = body.try_next().await? {
            //bytes.extend(item);
            f = web::block(move || f.write_all(&item).map(|_| f)).await??;
        }
        let file_node = repo::FileNode {
            id: 0,
            user_id: user.user_id,
            title: file_name,
            parent_id: parent_id,
            node_type: repo::NODE_TYPE_FILE,
            filesystem_path,
            mime_type: None
        };
        if let Ok(a) = repo::add_node(&pool, file_node).await {
            if a == 1 {
                return created();
            }
        }
    }
    error_bad_request()
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
    let default_path = "/mnt/storage/files/1";
    let parent: Cow<'static, str> = if parent_id > 0 {
        let node = repo::fetch_node(&pool, parent_id, user_id).await;
        node.map_or(default_path.into(), |n| n.filesystem_path.into())
    } else { default_path.into() };
    Path::new(parent.as_ref()).join(name).to_path_buf()
}
