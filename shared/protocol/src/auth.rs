use serde::{Deserialize, Serialize};

use crate::{NodeId, UserId};

/// Credentials submitted to log in.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Refresh token submitted to rotate a session or log out.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Access and refresh token returned on login and refresh.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

/// The authenticated user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Me {
    pub id: UserId,
    pub username: String,
    /// The Lost & Found folder, which cannot be deleted.
    pub lost_and_found_id: NodeId,
}

/// How this server lets users sign in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Providers {
    /// Username and password login is enabled.
    pub password: bool,
    /// An OpenID Connect provider (Auth0, Keycloak, ...) is configured.
    pub oidc: Option<OidcProvider>,
}

/// A configured OpenID Connect provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OidcProvider {
    /// Name shown on the sign-in button.
    pub name: String,
}

/// Query of `GET /auth/oidc/start`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OidcStartQuery {
    /// Where to send the user after sign-in: the web client's callback page or a loopback URL.
    pub redirect_uri: String,
    /// Base64url (unpadded) SHA-256 of the `code_verifier` presented later (PKCE `S256`).
    pub code_challenge: String,
}

/// Query the provider sends to `GET /auth/oidc/callback`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OidcCallbackQuery {
    pub state: String,
    pub code: Option<String>,
    pub error: Option<String>,
}

/// Exchanges the one-time code from the redirect for a session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OidcExchangeRequest {
    pub code: String,
    pub code_verifier: String,
}
