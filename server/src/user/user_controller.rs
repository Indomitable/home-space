use actix_web::{web, Responder, Result, post};

use home_space_contracts::user::{ LoginRequest, LoginResponse, RegisterRequest };
use log::debug;
use log::info;
use crate::ioc::container::Contrainer;
use crate::response::json;

use super::user_repository as repo;
use super::token;

#[post("/login")]
pub async fn login(provider: web::Data<Contrainer>, login: web::Json<LoginRequest>) -> Result<impl Responder> {
    let repo = provider.get_user_repository();
    return match repo.verify_password(&login.user_name, &login.password).await {
        Ok(_) => {
            debug!("Password verified");
            match repo.fetch_user(&login.user_name).await {
                Ok(user) => {
                    info!("[Auth] User logged in. [User: {}]", user.name);
                    json(LoginResponse {
                        user_id: user.id,
                        user_name: user.name.to_string(),
                        access_token: token::create_access_token(user.id, &user.name)?
                    })
                }
                _ => Err(actix_web::error::ErrorUnauthorized("User not found!")),
            }
        },
        Err(repo::ErrorVerifyPassword::UserNotFound) => {
            Err(actix_web::error::ErrorUnauthorized("User not found!"))
        },
        Err(repo::ErrorVerifyPassword::PasswordHasError(_)) => {
            Err(actix_web::error::ErrorUnauthorized("Wrong password!"))
        }
    }
}

#[post("/register")]
pub async fn register(provider: web::Data<Contrainer>, register: web::Json<RegisterRequest>) -> Result<impl Responder> {
    let mut repo = provider.get_user_repository();
    if let Ok(user) = repo.register_user(&register.user_name, &register.password).await {
        return json(LoginResponse {
            user_id: user.id,
            user_name: user.name.to_string(),
            access_token: token::create_access_token(user.id, &user.name)?
        });
    }
    Err(actix_web::error::ErrorInternalServerError("Registration Failed"))
}
