use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub user_id: i64,
    pub user_name: String,
    pub access_token: String,
}
