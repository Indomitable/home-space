use std::future::ready;

use actix_web::{HttpMessage, FromRequest};

use crate::{user::token::verify_access_token, response::error_unauthorized};

pub struct AuthContext {
    pub user_id: i64
}

impl FromRequest for AuthContext {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<AuthContext, actix_web::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        if let Some(context) = req.extensions().get::<AuthContext>() {
            let user_id = context.user_id;
            return ready(Ok(AuthContext { user_id }))
        }
        ready(error_unauthorized())
    }
}

pub async fn request_validator(req: actix_web::dev::ServiceRequest, credentials: actix_web_httpauth::extractors::bearer::BearerAuth) -> Result<actix_web::dev::ServiceRequest, (actix_web::Error, actix_web::dev::ServiceRequest)> {
    let token = credentials.token();
    if let Ok(user_id) = verify_access_token(token) {
        req.extensions_mut().insert(AuthContext { user_id });
        return Ok(req);
    }
    Err((actix_web::error::ErrorUnauthorized("Unauthorized! Please login."), req))
}

