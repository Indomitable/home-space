use std::sync::Arc;
use log::error;

use crate::{db::{DbResult, DatabaseAccess}, sorting::Sorting};

use super::{db::{file_node::FileNodeDto, DbModel}};

pub(crate) struct FileRepository {
    user_id: i64,
    db: Arc<DatabaseAccess>
}

impl FileRepository {
    pub(crate) fn new(user_id: i64,
                      db: &Arc<DatabaseAccess>) -> Self {
        Self {
            user_id,
            db: Arc::clone(db),
        }
    }

    pub(crate) async fn get_file_list(&self, parent_id: i64, sorting: &Sorting) -> DbResult<Vec<(FileNodeDto, bool)>> {
        let sql = format!(r#"select {},
        case 
            when ffn.id is null then false
            else true
        end is_favorite
    from file_nodes fn
    left join favorite_nodes ffn on fn.id = ffn.id and fn.user_id = ffn.user_id  
    where fn.user_id = $1 and fn.parent_id = $2
    order by node_type"#, FileNodeDto::column_list());
        let sorted_sql = format!("{}, {}", sql, sorting.build_order_by());
        return match self.db.query(&sorted_sql, &[&self.user_id, &parent_id]).await {
            Ok(rows) => {
                let nodes = rows
                    .iter()
                    .map(|row| (FileNodeDto::read_node(row), row.get(10)))
                    .collect();
                Ok(nodes)
            },
            Err(err) => {
                error!("{:?}", err);
                Err(err)
            }
        }
    }

    pub(crate) async fn get_node(&self, id: i64) -> DbResult<FileNodeDto> {
        let sql = format!(r#"select {} from file_nodes where user_id = $1 and id = $2"#, FileNodeDto::column_list());
        let row = self.db.query_one(&sql, &[&self.user_id, &id]).await?;
        let node = FileNodeDto::read_node(&row);
        Ok(node)
    }

    pub(crate) async fn get_node_by_path(&self, path: &str) -> DbResult<Option<FileNodeDto>> {
        let sql = format!(r#"select {} from file_nodes where user_id = $1 and filesystem_path = $2"#, FileNodeDto::column_list());
        let node =
            self.db.query_opt(&sql, &[&self.user_id, &path])
                .await?
                .map(|row| FileNodeDto::read_node(&row));
        Ok(node)
    }

    pub(crate) async fn get_node_by_title(&self, parent_id: i64, title: &str) -> DbResult<Option<FileNodeDto>> {
        let sql = format!(r#"select {}
            from file_nodes fn
            where fn.user_id = $1 and fn.parent_id = $2 and fn.title = $3"#, FileNodeDto::column_list());

        let node = self.db.query_opt(&sql, &[&self.user_id, &parent_id, &title])
            .await?
            .map(|row| FileNodeDto::read_node(&row));
        Ok(node)
    }

    // async fn search_nodes(&self, search_query: &SearchModel) -> DbResult<Vec<FileNodeDto>> {
    //     todo!("Implement search")
    //     // let (sql, params) = SearchQueryCreatorImpl::get_sql_query()
    //     // let sql = format!(r#"select {} from file_nodes where user_id = $1 and parent_id = $2 and title = $3"#, FileNodeDto::column_list());
    //     // let node = self.db.query_opt(&self.pool, &sql, &[&user_id, &parent_id, &title])
    //     //     .await?
    //     //     .map(|row| FileNodeDto::read_node(&row));
    //     // Ok(node)
    // }

    pub(crate) async fn add_node(&self, file_node: &FileNodeDto) -> DbResult<i64>  {
        let FileNodeDto {
            user_id,
            title,
            parent_id,
            node_type,
            filesystem_path,
            mime_type,
            modified_at,
            node_size,
            node_version,
            ..
        } = file_node;
        let sql = format!(r#"insert into file_nodes ({}) values (nextval('{}'), $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id"#, 
            FileNodeDto::column_list(), get_file_node_id_sequence(self.user_id));
        let row = self.db.query_one(&sql, &[user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size, node_version]).await?;
        let node_id: i64 = row.get(0);
        Ok(node_id)
    }

    pub(crate) async fn update_node(&self, old_node: &FileNodeDto, new_node: &FileNodeDto) -> DbResult<()> {
        // Copy current one to file_versions
        // let copy_sql = r#"insert into file_versions (id, user_id, node_version, created_at, node_size, file_name)
        // select fn.id, fn.user_id, fn.node_version, fn.modified_at, fn.node_size, $3
        // from file_nodes fn
        // where fn.user_id = $1 and fn.id = $2"#;
        // self.db.execute(&self.pool, copy_sql, &[&old_node.user_id, &old_node.id, &version_name]).await?;

        // Keep id, parent_id, title, node_type and filesystem_path
        let update_sql = r#"update file_nodes
        set mime_type = $3,
            modified_at = $4,
            node_size = $5,
            node_version = $6
        where user_id = $1 and id = $2"#;
        self.db.execute(update_sql, &[
            &old_node.user_id,
            &old_node.id,
            &new_node.mime_type,
            &chrono::Utc::now(),
            &new_node.node_size,
            &(old_node.node_version + 1)
        ]).await?;
        Ok(())
    }

   
    /// Delete file node or empty folder.
    pub(crate) async fn permanent_delete(&self, id: i64) -> DbResult<u64> {
        let delete_sql = r#"delete from file_nodes where id = $2 and user_id = $1"#;
        let affected = self.db.execute(delete_sql, &[&self.user_id, &id]).await?;
        Ok(affected)
    }

    pub(crate) async fn get_parent_nodes(&self, id: i64) -> DbResult<Vec<FileNodeDto>> {
        let sql = format!(r#"
        WITH RECURSIVE breadcrumbs_query AS ( 
            select *, 0 as lev from file_nodes
            where user_id = $1 and id = $2
            UNION ALL 
            select n.*, lev-1 as lev from file_nodes n
            INNER JOIN breadcrumbs_query p ON p.parent_id = n.id
        )
        select {} from breadcrumbs_query
        order by lev
        "#, FileNodeDto::column_list());
        let rows = self.db.query(&sql, &[&self.user_id, &id]).await?;
        let nodes = rows.iter().map(FileNodeDto::read_node).collect();
        Ok(nodes)
    }

    pub(crate) async fn get_child_nodes(&self, parent_id: i64) -> DbResult<Vec<FileNodeDto>> {
        let sql = format!("select {} from file_nodes where user_id = $1 and parent_id = $2", FileNodeDto::column_list());
        let rows = self.db.query(&sql, &[&self.user_id, &parent_id]).await?;
        let nodes = rows.iter().map(FileNodeDto::read_node).collect();
        Ok(nodes)
    }

    pub(crate) async fn get_decedent_nodes(&self, parent_id: i64) -> DbResult<Vec<FileNodeDto>> {
        let sql = format!(r#"
        WITH RECURSIVE breadcrumbs_query AS (
            select n0.*, 0 as lvl from file_nodes n0
            where user_id = $1 and id = $2
            UNION ALL
            select n1.*, lvl + 1 as lvl from file_nodes n1
            INNER JOIN breadcrumbs_query p ON p.id = n1.parent_id
        )
        select {} from breadcrumbs_query
        order by lev
        "#, FileNodeDto::column_list());
        let rows = self.db.query(&sql, &[&self.user_id, &parent_id]).await?;
        let nodes = rows.iter().map(FileNodeDto::read_node).collect();
        Ok(nodes)
    }

    // async fn move_to_trash(&self, id: i64) -> DbResult<()> {
    //     let get_nodes_sql = format!(r#"
    //     WITH RECURSIVE breadcrumbs_query AS (
    //         select n0.*, 0 as lvl from file_nodes n0
    //         where user_id = $1 and id = $2
    //         UNION ALL
    //         select n1.*, lvl + 1 as lvl from file_nodes n1
    //         INNER JOIN breadcrumbs_query p ON p.id = n1.parent_id
    //     )
    //     select {}, lvl,
    //           (select count(1) from file_versions fv where fv.id  = b.id and fv.user_id = b.user_id) versions
    //     from breadcrumbs_query b
    //     order by lvl desc
    //     "#, FileNodeDto::column_list());
    //
    //     let row_stream = self.db.query_raw(&self.pool, &get_nodes_sql, &[&self.user_id, &id]).await?;
    //     pin_mut!(row_stream);
    //
    //     // Get second connection for moving files while we read rows from the first.
    //     let mut connection = self.db.create_connection(&self.pool).await?;
    //
    //     while let Some(row) = row_stream.try_next()
    //         .await
    //         .map_err(|_| crate::db::DbError::Fetch("Error reading next row, while deleting nodes".to_owned()))? {
    //         let file_node = FileNodeDto::read_node(&row);
    //         let versions_count: i64 = row.get(11);
    //         if versions_count > 0 {
    //         }
    //
    //         // start new db transaction to move rows
    //         let transaction = connection.create_transaction().await?;
    //         let trash_file_name = Uuid::new_v4().as_hyphenated().to_string();
    //         match self.move_file_node_to_trash(&transaction, &file_node, &trash_file_name).await {
    //             Ok(_) => {
    //                 let move_file_future = self.fs.move_node_to_trash(Path::new(&file_node.filesystem_path), &trash_file_name);
    //                 match move_file_future.await {
    //                     Ok(_) => { transaction.commit().await?; },
    //                     Err(_) => { transaction.rollback().await?; }
    //                 }
    //             },
    //             Err(_) => {
    //                 transaction.rollback().await?;
    //             }
    //         }
    //     }
    //     Ok(())
    // }
    //
    // async fn restore_from_trash(&self, id: i64) -> DbResult<()> {
    //     todo!();
    // }

    // async fn move_file_node_to_trash(&self, transaction: &TransactionalDataAccess<'_>, file_node: &FileNodeDto, trash_file_name: &str) -> DbResult<()> {
    //     let insert_to_trash_sql =
    //     format!(r#"INSERT INTO trash_box ({}) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#, DeletedNodeDto::column_list());
    //
    //     let delete_sql = "delete from file_nodes fn where fn.user_id = $1 and fn.id = $2";
    //     transaction.execute(&insert_to_trash_sql, &[
    //         &file_node.id, &file_node.user_id, &file_node.title, &file_node.parent_id, &file_node.node_type,
    //         &file_node.filesystem_path, &file_node.mime_type, &chrono::Utc::now(), &file_node.node_size,
    //         &file_node.node_version, &trash_file_name
    //     ]).await?;
    //
    //     transaction.execute(delete_sql, &[&file_node.user_id, &file_node.id]).await?;
    //     Ok(())
    // }

    // async fn restore_file_node_from_trash(&self, transaction: &TransactionalDataAccess<'_>, deleted_node: &DeletedNodeDto) -> DbResult<()> {
    //     let check_parent_node_sql = "select * from file_nodes fn where fn.user_id = $1 and fn.id = $2";
    //     let parent_node = transaction.query_opt(check_parent_node_sql, &[&deleted_node.user_id, &deleted_node.parent_id]).await?;
    //     todo!("Check for existing parent");
    //     // match parent_node {
    //     //     Some() => {
    //     //     }
    //     //     None => {
    //     //         // We try to restore item but parent does not exists. We need to create the parent.
    //     //
    //     //     },
    //     // }
    //     Ok(())
    // }

    /*
        Add node to a parent if node with same name exists add version.
     */
    // async fn add_file_node_to_folder(&self, transaction: &TransactionalDataAccess<'_>, parent_id: i64, file_node: &FileNodeDto) -> DbResult<()> {
    //     let check_existing_node_sql = "select * from file_nodes fn where fn.user_id = $1 and fn.parent_id = $2 and title = $3";
    //     match transaction.query_opt(check_existing_node_sql, &[&file_node.user_id, &parent_id, &file_node.title]).await? {
    //         Some(found_node) => todo!("Create new version"),
    //         None => {
    //             let sql = format!(r#"insert into file_nodes ({}) values (nextval('{}'), $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id"#,
    //             FileNodeDto::column_list(), get_file_node_id_sequence(file_node.user_id));
    //             let row = self.db.query_one(&sql, &[&file_node.user_id,
    //                 &file_node.title,
    //                 &file_node.parent_id,
    //                 &file_node.node_type,
    //                 &file_node.filesystem_path,
    //                 &file_node.mime_type,
    //                 &file_node.modified_at,
    //                 &file_node.node_size,
    //                 &file_node.node_version
    //             ]).await?;
    //         },
    //     }
    //     Ok(())
    // }

    /*
        Move node to another parent if node with same name exists add version.
     */
    // async fn change_node_parent(&self, transaction: &TransactionalDataAccess<'_>, parent: &FileNodeDto, node: &FileNodeDto) -> DbResult<()> {
    //     let check_existing_node_sql = "select * from file_nodes fn where fn.user_id = $1 and fn.parent_id = $2 and title = $3";
    //     match transaction.query_opt(check_existing_node_sql, &[&node.user_id, &parent.id, &node.title]).await? {
    //         Some(found_node) => todo!("Create new version"),
    //         None => {
    //             let update_parent_sql = r#"update file_nodes fn set fn.parent_id = $3 where fn.user_id = $1 and fn.id = $2"#;
    //             let row = transaction.execute(&update_parent_sql, &[&parent.id]).await?;
    //         },
    //     }
    //     Ok(())
    // }
}

fn get_file_node_id_sequence(user_id: i64) -> String {
    format!("file_nodes_user_{}", user_id)
}
