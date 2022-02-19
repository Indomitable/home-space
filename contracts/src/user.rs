use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    pub user_name: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub user_id: i64,
    pub user_name: String,
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub user_name: String,
    pub password: String,
}
