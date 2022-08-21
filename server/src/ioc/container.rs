use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::db::new_pool;
use crate::files;
use crate::user;
use crate::user::user_repository::user_repository_new;

pub struct Contrainer {
    pool: Arc<Pool>,
}

impl Contrainer {
    pub fn new() -> Self {
        let pool = Arc::new(new_pool());
        Self {
            pool: pool, 
        }
    }

    pub(crate) fn get_db_access(&self) -> impl crate::db::DatabaseAccess {
        crate::db::data_access_new()
    }

    pub(crate) fn get_file_repository(&self) -> impl files::files_repository::FileRepository {
        let db_access = self.get_db_access();
        files::files_repository::file_repository_new(self.pool.clone(), Arc::new(db_access))
    }

    pub(crate) fn get_path_manager(&self) -> impl files::paths_manager::PathManager {
        files::paths_manager::path_manager_new()
    }

    pub(crate) fn get_trash_mover(&self, user_id: i64) -> impl files::trash_mover::TrashMover {
        let path_manager = self.get_path_manager();
        files::trash_mover::trash_mover_new(user_id, path_manager)
    }

    pub(crate) fn get_versions_mover(&self, user_id: i64) -> impl files::versions_mover::VersionsMover {
        let path_manager = self.get_path_manager();
        files::versions_mover::versions_mover_new(user_id, path_manager)
    }

    pub(crate) fn get_user_repository(&self) -> impl user::user_repository::UserRepository {
        let path_manager = self.get_path_manager();
        let db_access = self.get_db_access();
        user_repository_new(self.pool.clone(), Arc::new(path_manager), Arc::new(db_access))
    }
}
