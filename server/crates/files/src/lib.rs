//! File management for home-space: blob storage on disk plus ownership-scoped
//! operations on the folder/file tree.

mod error;
mod service;
mod store;

pub use error::FilesError;
pub use service::FileService;
pub use store::BlobStore;
