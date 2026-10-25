//! Typed HTTP client for the home-space API, shared by all Rust clients.

mod client;
mod error;
pub mod format;
pub mod listing;

pub use client::{Client, TokenListener};
pub use error::ApiError;
