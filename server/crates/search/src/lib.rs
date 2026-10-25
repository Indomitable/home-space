//! Full-text search for home-space: a per-node index over names, extended with file content
//! through pluggable [`ContentProvider`]s.

mod index;
mod provider;
mod terms;

pub use index::{CatchUp, SearchError, SearchIndex, Worker};
pub use provider::{ContentProvider, ExtractError};
