//! Authentication for home-space: password hashing, JWT access tokens and
//! rotating refresh tokens.

mod error;
mod password;
mod service;
mod token;

pub use error::AuthError;
pub use service::{AuthConfig, AuthService, TokenPair};
pub use token::Claims;
