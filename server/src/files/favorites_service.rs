use std::rc::Rc;
use crate::db::DatabaseAccess;
use crate::files::service_result::ServiceResult;

pub(crate) struct FavoritesService {
    user_id: i64,
    db: Rc<DatabaseAccess>,
}

impl FavoritesService {
    pub(crate) fn new(user_id: i64,
                      db: &Rc<DatabaseAccess>) -> Self {
        Self {
            user_id,
            db: Rc::clone(db),
        }
    }

    /// Make file node favorite
    pub(crate) async fn set_favorite(&self, id: i64) -> ServiceResult<()> {
        let insert_favorite_sql = r#"INSERT INTO favorite_nodes (id, user_id) VALUES($1, $2)"#;
        self.db.execute(insert_favorite_sql, &[&id, &self.user_id]).await?;
        Ok(())
    }

    /// Unset file not as favorite
    pub(crate) async fn unset_favorite(&self, id: i64) -> ServiceResult<()> {
        let delete_favorite_sql = r#"DELETE FROM favorite_nodes where user_id = $1 and id = $2"#;
        self.db.execute(delete_favorite_sql, &[&self.user_id, &id]).await?;
        Ok(())
    }
}
