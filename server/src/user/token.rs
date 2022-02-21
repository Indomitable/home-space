use hmac::{Hmac, Mac};
use jwt::{AlgorithmType, Header, Claims, SignWithKey, Token, RegisteredClaims, VerifyWithKey};
use sha2::Sha384;
use std::time::{SystemTime, Duration, UNIX_EPOCH};

use crate::config::{get_host_url, get_jwt_secret};

const TOKEN_EXPIRE_TIME: u64 = 2 * 3600; // 2 hours

pub fn create_access_token(user_id: i64, user_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let key: Hmac<Sha384> = Hmac::new_from_slice(get_secret().as_bytes()).unwrap();
    let header = Header {
        algorithm: AlgorithmType::Hs384,
        ..Default::default()
    };
    let exp_time = SystemTime::now().duration_since(UNIX_EPOCH)? + Duration::from_secs(TOKEN_EXPIRE_TIME);
    let mut claims = Claims::new(RegisteredClaims {
        expiration: Some(exp_time.as_secs()),
        issuer: Some(get_issuer()),
        ..Default::default()
    });
    claims.private.insert("user_id".to_string(), user_id.into());
    claims.private.insert("user_name".to_string(), user_name.into());

    let token = Token::new(header, claims).sign_with_key(&key)?;
    Ok(token.into())
}

pub enum VerifyAccessTokenError {
    NotVerified
}

pub fn verify_access_token(token: &str) -> Result<i64, VerifyAccessTokenError> {
    let key: Hmac<Sha384> = Hmac::new_from_slice(get_secret().as_bytes()).unwrap();
    let token: Token<Header, Claims, _> = token.verify_with_key(&key).unwrap();
    //let header = token.header();
    let claims = token.claims();
    let from_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(); 
    match claims.registered.expiration {
        Some(exp) if exp - from_epoch > 0 => {
            if let Some(issuer) = &claims.registered.issuer {
                if *issuer == get_issuer() {
                    if let Some(user_id) = claims.private.get("user_id") {
                        return Ok(user_id.as_i64().unwrap())
                    }
                }
            } else {
                return Err(VerifyAccessTokenError::NotVerified)
            }
        },
        _ => { return Err(VerifyAccessTokenError::NotVerified) }
    }
    Err(VerifyAccessTokenError::NotVerified)
}

#[inline]
fn get_issuer() -> String {
    return get_host_url();
}

#[inline]
fn get_secret() -> String {
    return get_jwt_secret();
}
