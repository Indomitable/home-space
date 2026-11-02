use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

use db::UserId;

use crate::AuthError;

const REFRESH_TOKEN_BYTES: usize = 32;
const KEY_ID_BYTES: usize = 6;

/// Signing keys: tokens are signed with the current secret and verified against the current
/// and any previous secrets, so the secret can be rotated without ending sessions.
#[derive(Clone)]
pub struct Keys {
    current: Vec<u8>,
    previous: Vec<Vec<u8>>,
}

impl std::fmt::Debug for Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keys")
            .field("previous", &self.previous.len())
            .finish_non_exhaustive()
    }
}

impl Keys {
    pub fn new(current: Vec<u8>, previous: Vec<Vec<u8>>) -> Self {
        Self { current, previous }
    }

    /// Identifier stored in the token header: a truncated hash, never the secret itself.
    fn key_id(secret: &[u8]) -> String {
        URL_SAFE_NO_PAD.encode(&Sha256::digest(secret)[..KEY_ID_BYTES])
    }

    fn header(&self) -> Header {
        Header {
            kid: Some(Self::key_id(&self.current)),
            ..Header::default()
        }
    }

    /// The secret a token was signed with. A token without a key id predates rotation support
    /// and is checked against the current secret only; an unknown key id is rejected.
    fn secret_for(&self, token: &str) -> Result<&[u8], AuthError> {
        let kid = decode_header(token)
            .map_err(|_| AuthError::InvalidCredentials)?
            .kid;
        match kid {
            None => Ok(&self.current),
            Some(kid) => std::iter::once(&self.current)
                .chain(&self.previous)
                .find(|secret| Self::key_id(secret) == kid)
                .map(Vec::as_slice)
                .ok_or(AuthError::InvalidCredentials),
        }
    }
}

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
pub fn encode_access(keys: &Keys, user_id: UserId, ttl: Duration) -> Result<String, AuthError> {
    let now = OffsetDateTime::now_utc();
    let claims = Claims {
        sub: user_id,
        iat: now.unix_timestamp(),
        exp: (now + ttl).unix_timestamp(),
    };
    Ok(encode(
        &keys.header(),
        &claims,
        &EncodingKey::from_secret(&keys.current),
    )?)
}

/// Validates signature and expiry of an access token.
pub fn decode_access(keys: &Keys, token: &str) -> Result<Claims, AuthError> {
    let validation = Validation {
        leeway: 0,
        ..Validation::default()
    };
    let secret = keys.secret_for(token)?;
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
    keys: &Keys,
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
        &keys.header(),
        &claims,
        &EncodingKey::from_secret(&keys.current),
    )?)
}

/// Validates a ticket's signature, expiry and scope, returning its user.
pub fn decode_ticket(keys: &Keys, token: &str, scope: &str) -> Result<UserId, AuthError> {
    let validation = Validation {
        leeway: 0,
        ..Validation::default()
    };
    let secret = keys.secret_for(token)?;
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

    fn keys(secret: &[u8]) -> Keys {
        Keys::new(secret.to_vec(), Vec::new())
    }

    #[test]
    fn access_token_round_trips() {
        let user = UserId::new();
        let token = encode_access(&keys(SECRET), user, Duration::minutes(5)).expect("encode");
        let claims = decode_access(&keys(SECRET), &token).expect("decode");
        assert_eq!(claims.sub, user);
    }

    #[test]
    fn expired_access_token_is_rejected() {
        let token =
            encode_access(&keys(SECRET), UserId::new(), Duration::seconds(-10)).expect("encode");
        assert!(decode_access(&keys(SECRET), &token).is_err());
    }

    #[test]
    fn access_token_with_other_secret_is_rejected() {
        let token =
            encode_access(&keys(SECRET), UserId::new(), Duration::minutes(5)).expect("encode");
        assert!(decode_access(&keys(b"other"), &token).is_err());
    }

    #[test]
    fn ticket_round_trips_for_its_scope() {
        let user = UserId::new();
        let ticket =
            encode_ticket(&keys(SECRET), user, "events", Duration::seconds(60)).expect("encode");
        assert_eq!(
            decode_ticket(&keys(SECRET), &ticket, "events").expect("decode"),
            user
        );
    }

    #[test]
    fn ticket_is_rejected_for_another_scope() {
        let ticket = encode_ticket(
            &keys(SECRET),
            UserId::new(),
            "events",
            Duration::seconds(60),
        )
        .expect("encode");
        assert!(decode_ticket(&keys(SECRET), &ticket, "download:x").is_err());
    }

    #[test]
    fn expired_ticket_is_rejected() {
        let ticket = encode_ticket(
            &keys(SECRET),
            UserId::new(),
            "events",
            Duration::seconds(-10),
        )
        .expect("encode");
        assert!(decode_ticket(&keys(SECRET), &ticket, "events").is_err());
    }

    #[test]
    fn ticket_is_not_an_access_token() {
        let ticket = encode_ticket(
            &keys(SECRET),
            UserId::new(),
            "events",
            Duration::seconds(60),
        )
        .expect("encode");
        assert!(decode_access(&keys(SECRET), &ticket).is_err());
    }

    #[test]
    fn access_token_is_not_a_ticket() {
        let token =
            encode_access(&keys(SECRET), UserId::new(), Duration::minutes(5)).expect("encode");
        assert!(decode_ticket(&keys(SECRET), &token, "events").is_err());
    }

    #[test]
    fn token_signed_with_a_previous_secret_is_accepted() {
        let user = UserId::new();
        let token =
            encode_access(&keys(b"old-secret"), user, Duration::minutes(5)).expect("encode");
        let rotated = Keys::new(b"new-secret".to_vec(), vec![b"old-secret".to_vec()]);
        assert_eq!(decode_access(&rotated, &token).expect("decode").sub, user);
    }

    #[test]
    fn token_signed_with_a_dropped_secret_is_rejected() {
        let token = encode_access(&keys(b"old-secret"), UserId::new(), Duration::minutes(5))
            .expect("encode");
        assert!(decode_access(&keys(b"new-secret"), &token).is_err());
    }

    #[test]
    fn new_tokens_use_the_current_secret() {
        let rotated = Keys::new(b"new-secret".to_vec(), vec![b"old-secret".to_vec()]);
        let token = encode_access(&rotated, UserId::new(), Duration::minutes(5)).expect("encode");
        assert!(decode_access(&keys(b"new-secret"), &token).is_ok());
        assert!(decode_access(&keys(b"old-secret"), &token).is_err());
    }

    #[test]
    fn refresh_tokens_are_unique() {
        assert_ne!(new_refresh_token(), new_refresh_token());
    }
}
