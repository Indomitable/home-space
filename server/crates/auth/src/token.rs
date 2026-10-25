use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

use db::UserId;

use crate::AuthError;

const REFRESH_TOKEN_BYTES: usize = 32;

/// Claims carried by an access token.
///
/// Unknown claims are rejected so a ticket can never pass as an access token.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub sub: UserId,
    pub iat: i64,
    pub exp: i64,
}

/// Signs an access token for `user_id` valid for `ttl`.
pub fn encode_access(secret: &[u8], user_id: UserId, ttl: Duration) -> Result<String, AuthError> {
    let now = OffsetDateTime::now_utc();
    let claims = Claims {
        sub: user_id,
        iat: now.unix_timestamp(),
        exp: (now + ttl).unix_timestamp(),
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )?)
}

/// Validates signature and expiry of an access token.
pub fn decode_access(secret: &[u8], token: &str) -> Result<Claims, AuthError> {
    let validation = Validation {
        leeway: 0,
        ..Validation::default()
    };
    decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .map(|data| data.claims)
        .map_err(|_| AuthError::InvalidCredentials)
}

#[derive(Serialize, Deserialize)]
struct TicketClaims {
    sub: UserId,
    iat: i64,
    exp: i64,
    tkt: String,
}

/// Signs a short-lived ticket for `user_id` limited to `scope`.
pub fn encode_ticket(
    secret: &[u8],
    user_id: UserId,
    scope: &str,
    ttl: Duration,
) -> Result<String, AuthError> {
    let now = OffsetDateTime::now_utc();
    let claims = TicketClaims {
        sub: user_id,
        iat: now.unix_timestamp(),
        exp: (now + ttl).unix_timestamp(),
        tkt: scope.to_owned(),
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )?)
}

/// Validates a ticket's signature, expiry and scope, returning its user.
pub fn decode_ticket(secret: &[u8], token: &str, scope: &str) -> Result<UserId, AuthError> {
    let validation = Validation {
        leeway: 0,
        ..Validation::default()
    };
    let claims = decode::<TicketClaims>(token, &DecodingKey::from_secret(secret), &validation)
        .map(|data| data.claims)
        .map_err(|_| AuthError::InvalidCredentials)?;
    if claims.tkt == scope {
        Ok(claims.sub)
    } else {
        Err(AuthError::InvalidCredentials)
    }
}

/// Generates a random opaque refresh token.
pub fn new_refresh_token() -> String {
    let mut bytes = [0u8; REFRESH_TOKEN_BYTES];
    rand::fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Hash under which a refresh token is stored.
pub fn hash_refresh_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"test-secret";

    #[test]
    fn access_token_round_trips() {
        let user = UserId::new();
        let token = encode_access(SECRET, user, Duration::minutes(5)).expect("encode");
        let claims = decode_access(SECRET, &token).expect("decode");
        assert_eq!(claims.sub, user);
    }

    #[test]
    fn expired_access_token_is_rejected() {
        let token = encode_access(SECRET, UserId::new(), Duration::seconds(-10)).expect("encode");
        assert!(decode_access(SECRET, &token).is_err());
    }

    #[test]
    fn access_token_with_other_secret_is_rejected() {
        let token = encode_access(SECRET, UserId::new(), Duration::minutes(5)).expect("encode");
        assert!(decode_access(b"other", &token).is_err());
    }

    #[test]
    fn ticket_round_trips_for_its_scope() {
        let user = UserId::new();
        let ticket = encode_ticket(SECRET, user, "events", Duration::seconds(60)).expect("encode");
        assert_eq!(
            decode_ticket(SECRET, &ticket, "events").expect("decode"),
            user
        );
    }

    #[test]
    fn ticket_is_rejected_for_another_scope() {
        let ticket =
            encode_ticket(SECRET, UserId::new(), "events", Duration::seconds(60)).expect("encode");
        assert!(decode_ticket(SECRET, &ticket, "download:x").is_err());
    }

    #[test]
    fn expired_ticket_is_rejected() {
        let ticket =
            encode_ticket(SECRET, UserId::new(), "events", Duration::seconds(-10)).expect("encode");
        assert!(decode_ticket(SECRET, &ticket, "events").is_err());
    }

    #[test]
    fn ticket_is_not_an_access_token() {
        let ticket =
            encode_ticket(SECRET, UserId::new(), "events", Duration::seconds(60)).expect("encode");
        assert!(decode_access(SECRET, &ticket).is_err());
    }

    #[test]
    fn access_token_is_not_a_ticket() {
        let token = encode_access(SECRET, UserId::new(), Duration::minutes(5)).expect("encode");
        assert!(decode_ticket(SECRET, &token, "events").is_err());
    }

    #[test]
    fn refresh_tokens_are_unique() {
        assert_ne!(new_refresh_token(), new_refresh_token());
    }
}
