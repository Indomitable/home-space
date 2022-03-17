use actix_web::{post, web, Responder, Result};
use deadpool_postgres::Pool;

use crate::auth::AuthContext;
use crate::response::error_internal_server_error;
use super::files_repository as repo;

#[post("/set_favorite/{id}")]
pub async fn set_favorite(pool: web::Data<Pool>, path: web::Path<i64>, user: AuthContext) -> Result<impl Responder> {
    let id = path.into_inner();
    if let Ok(nodes) = repo::set_favorite(&pool, id, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_internal_server_error()
}

#[post("/unset_favorite/{id}")]
pub async fn unset_favorite(pool: web::Data<Pool>, path: web::Path<i64>, user: AuthContext) -> Result<impl Responder> {
    let id = path.into_inner();
    if let Ok(nodes) = repo::unset_favorite(&pool, id, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_internal_server_error()
}
