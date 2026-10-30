//! Wire types shared by the home-space server and its clients.

mod auth;
mod change;
mod error;
mod event;
mod ids;
mod node;
pub mod paths;
mod search;
mod ticket;
mod trash;

pub use auth::{LoginRequest, Me, RefreshRequest, TokenPair};
pub use change::{Change, ChangesQuery, ChangesResponse};
pub use error::{ErrorBody, ErrorCode};
pub use event::Event;
pub use ids::{NodeId, UserId};
pub use node::{
    Conflict, ContentQuery, CreateFolderRequest, FileVersion, ListQuery, MoveRequest, MoveResponse,
    Node, NodeKind, OnConflict, UpdateRequest, UploadQuery,
};
pub use search::{SearchHit, SearchQuery};
pub use ticket::{TicketRequest, TicketResponse};
pub use trash::{RestoreRequest, TrashItem};
