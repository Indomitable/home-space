use auth::AuthService;
use db::Db;
use files::FileService;
use notifications::Hub;

/// Shared services handed to every handler.
#[derive(Clone, Debug)]
pub struct AppState {
    pub db: Db,
    pub auth: AuthService,
    pub files: FileService,
    pub hub: Hub,
}
