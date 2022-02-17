use std::{path::Path, io::Write};
use actix_web::{web, get, Responder, Result, HttpRequest};
use deadpool_postgres::Pool;
use futures_util::TryStreamExt;
use serde::Deserialize;

use crate::response::{error_server_unavailable, created, error_bad_request};
use super::files_repository::{self as repo, FileNode, fetch_node};

#[derive(Deserialize)]
pub struct User {
    user_id: i64
}

#[get("/get_top_nodes")]
pub async fn get_top_nodes(pool: web::Data<Pool>, user: web::Query<User>) -> Result<impl Responder> {
    if let Ok(nodes) = repo::fetch_top_nodes(&pool, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_server_unavailable()
}

#[get("/get_nodes/{parent_id}")]
pub async fn get_nodes(pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    if let Ok(nodes) = repo::fetch_nodes(&pool, parent_id, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_server_unavailable()
}

pub async fn upload_file(request: HttpRequest, pool: web::Data<Pool>, user: web::Query<User>, mut body: web::Payload) -> Result<impl Responder> {
    let parent_id = request.match_info().get("parent_id").and_then(|id| id.parse::<i64>().ok());
    if let Some(file_name_encoded) = request.headers().get("X-File-Name").map(|h| {
        // let bytes = h.as_bytes().to_vec();
        // let as_str = bytes.iter().map(|b| b.to_string()).fold(String::from(""), |mut a, c| { a.push_str(c.as_str()); a.push_str("-"); a });
        // println!("{:?}", as_str);
        // String::from_utf8(bytes).ok()
        h.as_bytes()
    }) {
        let file_name = percent_encoding::percent_decode(file_name_encoded).decode_utf8().unwrap().to_string();
        let user_id = user.user_id;
        let default_path = "/mnt/storage/files/1".to_string();
        let parent = if let Some(id) = parent_id {
            let node = fetch_node(&pool, id, user_id).await;
            node.map_or(default_path, |n| n.filesystem_path)
        } else { default_path };
       
        let path = Path::new(&parent);
        let output = path.join(&file_name);
        let filesystem_path = output.clone().to_str().unwrap().to_string(); // Clone to filesystem_path because output will be moved on file create
        let mut f = web::block(|| std::fs::File::create(output)).await??;
        while let Some(item) = body.try_next().await? {
            //bytes.extend(item);
            f = web::block(move || f.write_all(&item).map(|_| f)).await??;
        }
        let file_node = FileNode {
            id: 0,
            user_id: user.user_id,
            title: file_name,
            parent_id: parent_id,
            node_type: 1,
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
