use actix_web::{post, web, Responder, Result};
use deadpool_postgres::Pool;

use home_space_contracts::favorites::UpdateFavoriteRequest;

use crate::auth::AuthContext;
use crate::response::error_internal_server_error;
use super::files_repository as repo;

#[post("/set_favorite")]
pub async fn set_favorite(pool: web::Data<Pool>, body: web::Json<UpdateFavoriteRequest>, user: AuthContext) -> Result<impl Responder> {
    if let Ok(nodes) = repo::set_favorite(&pool, body.id, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_internal_server_error()
}

#[post("/unset_favorite")]
pub async fn unset_favorite(pool: web::Data<Pool>, body: web::Json<UpdateFavoriteRequest>, user: AuthContext) -> Result<impl Responder> {
    if let Ok(nodes) = repo::unset_favorite(&pool, body.id, user.user_id).await {
        return Ok(web::Json(nodes));
    }
    error_internal_server_error()
}
