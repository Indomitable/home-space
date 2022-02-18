use actix_web::{web, Responder, Result, post};
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};

use crate::response::{error_unauthorized, json};

use super::user_repository as repo;

#[derive(Deserialize)]
pub struct LoginRequest {
    user_name: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    user_name: String,
}

#[post("/login")]
pub async fn login(pool: web::Data<Pool>, login: web::Json<LoginRequest>) -> Result<impl Responder> {
    if let Ok(_) = repo::verify_password(&pool, &login.user_name, &login.password).await {
        if let Ok(user) = repo::fetch_user(&pool, &login.user_name).await {
            return json(LoginResponse {
                user_name: user.name.to_string()
            });
        }
    }
    error_unauthorized()
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    user_name: String,
    password: String,
}

#[post("/register")]
pub async fn register(pool: web::Data<Pool>, register: web::Json<RegisterRequest>) -> Result<impl Responder> {
    if let Ok(user) = repo::register_user(&pool, &register.user_name, &register.password).await {
        return json(LoginResponse {
            user_name: user.name.to_string()
        });
    }
    error_unauthorized()
}
