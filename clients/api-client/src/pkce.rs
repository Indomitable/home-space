//! PKCE (RFC 7636, `S256`) for signing in through the server's single sign-on.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// Bytes of randomness in a verifier; 32 bytes encode to the 43 characters PKCE requires.
pub const VERIFIER_BYTES: usize = 32;

/// A one-time secret (the verifier) and the challenge that commits to it.
///
/// The challenge goes to the server when a sign-in starts; the verifier is kept and shown only
/// when exchanging the login code, so an intercepted code is useless without it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pkce {
    verifier: String,
    challenge: String,
}

impl Pkce {
    /// Builds a pair from `bytes`, which must come from a cryptographically secure source
    /// (`crypto.getRandomValues` in the browser, the OS random generator natively).
    pub fn from_random(bytes: [u8; VERIFIER_BYTES]) -> Self {
        Self::from_verifier(URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Rebuilds a pair from a saved verifier.
    pub fn from_verifier(verifier: impl Into<String>) -> Self {
        let verifier = verifier.into();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Self {
            verifier,
            challenge,
        }
    }

    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    pub fn challenge(&self) -> &str {
        &self.challenge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_rfc_7636_example() {
        let pkce = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        assert_eq!(
            pkce.challenge(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn random_verifiers_have_the_required_length() {
        let pkce = Pkce::from_random([7; VERIFIER_BYTES]);
        assert_eq!(pkce.verifier().len(), 43);
        assert_eq!(pkce.challenge().len(), 43);
    }
}
