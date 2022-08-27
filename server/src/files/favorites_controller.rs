use actix_web::{post, web, Responder, Result};

use home_space_contracts::favorites::UpdateFavoriteRequest;
use log::error;

use crate::auth::AuthContext;
use crate::ioc::container::Contrainer;
use crate::response::{error_internal_server_error, ok};

#[post("/favorite")]
pub async fn favorite(provider: web::Data<Contrainer>, body: web::Json<UpdateFavoriteRequest>, user: AuthContext) -> Result<impl Responder> {
    let repo = provider.get_favorites_service(user.user_id);
    let result = if body.favorite {
        repo.set_favorite(body.id).await
    } else {
        repo.unset_favorite(body.id).await
    };
    match result {
        Ok(_) => ok(),
        Err(e) => {
            error!("Error has occurred while toggle favorite: {:?}", e);
            error_internal_server_error()
        }
    }
}
