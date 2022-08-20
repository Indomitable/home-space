use actix_web::{post, web, Responder, Result};

use home_space_contracts::favorites::UpdateFavoriteRequest;
use log::error;

use crate::auth::AuthContext;
use crate::files::files_repository::FileRepository;
use crate::ioc::container::Contrainer;
use crate::response::error_internal_server_error;

#[post("/favorite")]
pub async fn favorite(provider: web::Data<Contrainer>, body: web::Json<UpdateFavoriteRequest>, user: AuthContext) -> Result<impl Responder> {
    let repo = provider.get_file_repository();
    let result = if body.favorite {
        repo.set_favorite(body.id, user.user_id).await
    } else {
        repo.unset_favorite(body.id, user.user_id).await
    };
    match result {
        Ok(nodes) => Ok(web::Json(nodes)),
        Err(e) => {
            error!("Error while setting favorite: {:?}", e);
            error_internal_server_error()
        }
    }
}
