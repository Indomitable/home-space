use actix_web::{web, Responder, Result, post};
use deadpool_postgres::Pool;

use home_space_contracts::user::{ LoginRequest, LoginResponse, RegisterRequest };
use log::debug;
use crate::response::error_internal_server_error;
use crate::response::{error_unauthorized, json};

use super::user_repository as repo;
use super::token;

#[post("/login")]
pub async fn login(pool: web::Data<Pool>, login: web::Json<LoginRequest>) -> Result<impl Responder> {
    debug!("Start login");
    match repo::verify_password(&pool, &login.user_name, &login.password).await {
        Ok(_) => {
            debug!("Password verified");
            match repo::fetch_user(&pool, &login.user_name).await {
                Ok(user) => {
                    debug!("User fetched");
                    return json(LoginResponse {
                        user_id: user.id,
                        user_name: user.name.to_string(),
                        access_token: token::create_access_token(user.id, &user.name)?
                    });
                }
                _ => return Err(actix_web::error::ErrorUnauthorized("User not found!")),
            }
        },
        Err(repo::ErrorVerifyPassword::UserNotFound) => {
            return Err(actix_web::error::ErrorUnauthorized("User not found!"))
        },
        Err(repo::ErrorVerifyPassword::PasswordHasError(_)) => {
            return error_internal_server_error();
        }
    }
}

#[post("/register")]
pub async fn register(pool: web::Data<Pool>, register: web::Json<RegisterRequest>) -> Result<impl Responder> {
    if let Ok(user) = repo::register_user(&pool, &register.user_name, &register.password).await {
        return json(LoginResponse {
            user_id: user.id,
            user_name: user.name.to_string(),
            access_token: token::create_access_token(user.id, &user.name)?
        });
    }
    error_unauthorized()
}
