//! Typed HTTP client for the home-space API, shared by all Rust clients.

mod client;
mod devices;
mod error;
mod pkce;
mod uploads;

pub use client::{Client, TokenListener};
pub use error::ApiError;
pub use pkce::{Pkce, VERIFIER_BYTES};
pub use uploads::RESUMABLE_THRESHOLD;
