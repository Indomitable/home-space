use std::{path::Path, io::Write};
use actix_web::{web, get, put, Responder, Result, HttpRequest};
use deadpool_postgres::Pool;
use futures_util::TryStreamExt;
use serde::Deserialize;

use crate::response::{error_server_unavailable, error_bad_request, created};
use super::files_repository as repo;

#[derive(Deserialize)]
pub struct User {
    user_id: i64
}

#[get("/get_top_nodes")]
pub async fn get_top_nodes(pool: web::Data<Pool>, user: web::Query<User>) -> Result<impl Responder> {
    if let Ok(nodes) = repo::fetch_top_nodes(pool, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_server_unavailable()
}

#[get("/get_nodes/{parent_id}")]
pub async fn get_nodes(pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    if let Ok(nodes) = repo::fetch_nodes(pool, parent_id, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_server_unavailable()
}

#[put("/upload_file")]
pub async fn upload_file(request: HttpRequest, mut body: web::Payload) -> Result<impl Responder> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.try_next().await? {
        //bytes.extend(item);
        bytes.extend_from_slice(&item);
    }
    let path = Path::new("/mnt/storage/files/1/");
    let output = path.join("test");
    let mut f = web::block(|| std::fs::File::create(output)).await??;
    f = web::block(move || f.write_all(&bytes).map(|_| f)).await??;
    created()
}
