//! Wire types shared by the home-space server and its clients.

mod auth;
mod error;
mod event;
mod ids;
mod node;
pub mod paths;
mod search;
mod ticket;

pub use auth::{LoginRequest, Me, RefreshRequest, TokenPair};
pub use error::{ErrorBody, ErrorCode};
pub use event::Event;
pub use ids::{NodeId, UserId};
pub use node::{
    CreateFolderRequest, FileVersion, ListQuery, Node, NodeKind, UpdateRequest, UploadQuery,
};
pub use search::{SearchHit, SearchQuery};
pub use ticket::{TicketRequest, TicketResponse};
