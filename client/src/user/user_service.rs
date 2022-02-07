use serde::Deserialize;
use serde_json::json;

use crate::api::api_service::ApiService;

struct UserService {
    access_token: String
}

#[derive(Deserialize)]
struct UserLoginResponse {
    access_token: String
}


impl UserService {
    fn new() -> Self {
        Self {
            access_token: "".to_owned()
        }
    }

    async fn login(&mut self, user_name: &str, password: &str) -> bool {
        let user_login = json!({
            "user_name": user_name,
            "password": password
        });
        let response = ApiService::post::<UserLoginResponse, _>("/api/auth/login", &user_login).await;
        match response {
            Ok(val) => {
                self.access_token = val.access_token;
                true
            }
            Err(_) => false
        }
    }
}