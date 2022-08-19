use actix_web::{post, web, Responder, Result};
use deadpool_postgres::Pool;

use home_space_contracts::favorites::UpdateFavoriteRequest;
use log::error;

use crate::auth::AuthContext;
use crate::response::error_internal_server_error;
use super::files_repository as repo;

#[post("/favorite")]
pub async fn favorite(pool: web::Data<Pool>, body: web::Json<UpdateFavoriteRequest>, user: AuthContext) -> Result<impl Responder> {
    let result = if body.favorite {
        repo::set_favorite(&pool, body.id, user.user_id).await
    } else {
        repo::unset_favorite(&pool, body.id, user.user_id).await
    };
    match result {
        Ok(nodes) => Ok(web::Json(nodes)),
        Err(e) => {
            error!("Error while setting favorite: {:?}", e);
            error_internal_server_error()
        }
    }
}
