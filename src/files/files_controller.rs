use actix_web::{web, get, Responder, Result};
use deadpool_postgres::Pool;
use serde::Deserialize;

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
    return Err(actix_web::error::ErrorServiceUnavailable("Service down"));
}

#[get("/get_nodes/{parent_id}")]
pub async fn get_nodes(pool: web::Data<Pool>, path: web::Path<i64>, user: web::Query<User>) -> Result<impl Responder> {
    let parent_id = path.into_inner();
    if let Ok(nodes) = repo::fetch_nodes(pool, parent_id, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    return Err(actix_web::error::ErrorServiceUnavailable("Service down"));
}
