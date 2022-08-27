use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::db::{DatabaseAccess, new_pool};
use crate::files;
use crate::files::copy_service::CopyService;
use crate::files::files_repository::FileRepository;
use crate::files::node_create_service::NodeCreateService;
use crate::files::version_service::VersionService;
use crate::user::user_repository::UserRepository;

pub struct Contrainer {
    pool: Arc<Pool>,
}

impl Contrainer {
    pub fn new() -> Self {
        let pool = Arc::new(new_pool());
        Self {
            pool,
        }
    }

    pub(crate) fn copy_service(&self, user_id: i64) -> files::copy_service::CopyService {
        let db = Arc::new(DatabaseAccess::new(&self.pool));
        let path_manager = Arc::new(files::paths_manager::PathManager::new());
        let file_system = Arc::new(files::file_system::FileSystemManager::new(user_id, &path_manager));
        let file_repository = Arc::new(FileRepository::new(user_id, &db, &file_system));
        let version_service = Arc::new(VersionService::new(user_id, &db, &file_system));
        CopyService::new(user_id, &db, &file_repository, &file_system, &version_service)
    }

    pub(crate) fn get_user_repository(&self) -> UserRepository {
        let db = Rc::new(RefCell::new(DatabaseAccess::new(&self.pool)));
        let path_manager = Rc::new(files::paths_manager::PathManager::new());
        UserRepository::new(&path_manager, &db)
    }

    pub(crate) fn get_node_create_service(&self, user_id: i64) -> NodeCreateService {
        let db = Arc::new(DatabaseAccess::new(&self.pool));
        let path_manager = Arc::new(files::paths_manager::PathManager::new());
        let file_system = Arc::new(files::file_system::FileSystemManager::new(user_id, &path_manager));
        let file_repository = Arc::new(FileRepository::new(user_id, &db, &file_system));
        let version_service = Arc::new(VersionService::new(user_id, &db, &file_system));
        NodeCreateService::new(user_id, &path_manager, &file_repository, &file_system, &version_service)
    }
}
