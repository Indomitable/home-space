use argon2::{Argon2, PasswordHasher, PasswordVerifier, password_hash::phc::PasswordHash};

use crate::AuthError;

/// Hashes a password with argon2id and a random salt, returning the PHC string.
pub fn hash(password: &str) -> Result<String, AuthError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|err| AuthError::Hashing(err.to_string()))
}

/// Checks a password against a PHC hash string; malformed hashes never verify.
pub fn verify(password: &str, phc_hash: &str) -> bool {
    PasswordHash::new(phc_hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_correct_password() {
        let hashed = hash("hunter2").expect("hash");
        assert!(verify("hunter2", &hashed));
    }

    #[test]
    fn rejects_wrong_password() {
        let hashed = hash("hunter2").expect("hash");
        assert!(!verify("hunter3", &hashed));
    }

    #[test]
    fn rejects_malformed_hash() {
        assert!(!verify("hunter2", "not-a-hash"));
    }
}
