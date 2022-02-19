use hmac::{Hmac, Mac};
use jwt::{AlgorithmType, Header, Claims, SignWithKey, Token, RegisteredClaims};
use sha2::Sha384;
use std::{time::{SystemTime, Duration, UNIX_EPOCH}, ops::Add};

const TOKEN_EXPIRE_TIME: u64 = 2 * 3600; // 2 hours

pub fn create_access_token(user_id: i64, user_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let key: Hmac<Sha384> = Hmac::new_from_slice(b"some-secret").unwrap();
    let header = Header {
        algorithm: AlgorithmType::Hs384,
        ..Default::default()
    };
    let exp_time = SystemTime::now().duration_since(UNIX_EPOCH)?.add(Duration::from_secs(TOKEN_EXPIRE_TIME));
    let mut claims = Claims::new(RegisteredClaims {
        expiration: Some(exp_time.as_secs()),
        issuer: Some("http://localhost:7070".to_string()),
        ..Default::default()
    });
    claims.private.insert("user_id".to_string(), user_id.into());
    claims.private.insert("user_name".to_string(), user_name.into());

    let token = Token::new(header, claims).sign_with_key(&key)?;
    Ok(token.into())
}
