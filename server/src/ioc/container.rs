use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::db::{DatabaseAccessImpl, new_pool};
use crate::files;
use crate::user;

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

    pub(crate) fn copy_service(&self, user_id: i64) -> impl files::copy_service::CopyService {
        let db = Arc::new(DatabaseAccessImpl::new());
        let path_manager = files::paths_manager::PathManagerImpl::new();
        let fs = Arc::new(files::file_system::FileSystemManagerImpl::new(user_id, path_manager));
        let repo = Arc::new(
            files::files_repository::FileRepositoryImpl::new(
                user_id,
                self.pool.clone(),
                db.clone(),
                fs.clone()
            ));
        let version_service = Arc::new(
            files::version_service::VersionServiceImpl::new(
                user_id,
                self.pool.clone(),
                db.clone(),
                fs.clone()
            ));
        files::copy_service::CopyServiceImpl::new(
            user_id,
            self.pool.clone(),
            db.clone(),
            repo.clone(),
            fs.clone(),
            version_service
        )
    }

    pub(crate) fn get_user_repository(&self) -> impl user::user_repository::UserRepository {
        let db = Arc::new(DatabaseAccessImpl::new());
        let path_manager = Arc::new(files::paths_manager::PathManagerImpl::new());
        user::user_repository::UserRepositoryImpl::new(self.pool.clone(), path_manager, db)
    }
}
