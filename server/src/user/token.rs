use jsonwebtoken::{encode, decode, Algorithm, Header, EncodingKey, DecodingKey, Validation, };
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, Duration, UNIX_EPOCH};

use crate::config::{get_host_url, get_jwt_secret};

const TOKEN_EXPIRE_TIME: u64 = 2 * 3600; // 2 hours

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: u64,
    iss: String,
    user_id: i64,
    user_name: String
}

pub fn create_access_token(user_id: i64, user_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let header = Header::new(Algorithm::HS512);
    let exp_time = SystemTime::now().duration_since(UNIX_EPOCH)? + Duration::from_secs(TOKEN_EXPIRE_TIME);
    let claims = Claims {
        exp: exp_time.as_secs(),
        iss: get_issuer(),
        user_id: user_id,
        user_name: user_name.to_owned()
    };
    let token = encode(&header, &claims, &EncodingKey::from_secret(get_secret().as_bytes()))?;
    Ok(token.into())
}

pub enum VerifyAccessTokenError {
    NotVerified
}

pub fn verify_access_token(token: &str) -> Result<i64, VerifyAccessTokenError> {
    let mut validation = Validation::new(Algorithm::HS512);
    validation.set_issuer(&[get_issuer()]);
    match decode::<Claims>(token, &DecodingKey::from_secret(get_secret().as_bytes()), &validation) {
        Ok(token) => {
            Ok(token.claims.user_id)
        },
        Err(_) => {
            Err(VerifyAccessTokenError::NotVerified)
        }
    }
}

#[inline]
fn get_issuer() -> String {
    return get_host_url();
}

#[inline]
fn get_secret() -> String {
    return get_jwt_secret();
}
