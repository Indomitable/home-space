//! Authentication for home-space: password hashing, JWT access tokens and
//! rotating refresh tokens.

mod error;
mod password;
mod service;
mod token;

pub use error::AuthError;
pub use protocol::TokenPair;
pub use service::{AuthConfig, AuthService};
pub use token::Claims;
