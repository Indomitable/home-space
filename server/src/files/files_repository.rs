use std::{borrow::Cow, sync::Arc};

use async_trait::async_trait;
use deadpool_postgres::Pool;

use futures_util::{pin_mut, TryStreamExt};
use home_space_contracts::files::{ParentNode, DisplayFileNode};
use log::error;

use crate::{db::{DbResult, DatabaseAccess, TransactionalDataAccess}, sorting::Sorting, files::db::deleted_node::DeletedNodeDto};

use super::{db::{file_node::FileNodeDto, DbModel, file_version::FileVersionDto}, trash_mover::TrashMover};

#[async_trait]
pub(crate) trait FileRepository {
    async fn get_file_list(&self, parent_id: i64, user_id: i64, sorting: &Sorting) -> DbResult<Vec<DisplayFileNode>>;
    async fn get_node(&self, id: i64, user_id: i64) -> DbResult<FileNodeDto>;
    async fn find_node_by_name(&self, parent_id: i64, user_id: i64, title: Cow<'_, str>) -> DbResult<Option<FileNodeDto>>;
    async fn add_node(&self, file_node: FileNodeDto) -> DbResult<i64>;
    async fn update_node(&self, old_node: &FileNodeDto, version_name: String, new_mime_type: &str, new_node_size: i64) -> DbResult<()>;
    async fn permanent_delete(&self, id: i64, user_id: i64) -> DbResult<u64>;
    async fn get_parent_nodes(&self, parent_id: i64, user_id: i64) -> DbResult<Vec<ParentNode>>;
    async fn move_to_trash<TM>(&self, id: i64, user_id: i64, trash_mover: TM) -> DbResult<()> where TM: TrashMover + std::marker::Send;
    async fn restore_from_trash<TM>(&self, id: i64, user_id: i64, trash_mover: TM) -> DbResult<()> where TM: TrashMover + std::marker::Send;
    async fn set_favorite(&self, id: i64, user_id: i64) -> DbResult<u64>;
    async fn unset_favorite(&self, id: i64, user_id: i64) -> DbResult<u64>;
    async fn get_file_versions(&self, id: i64, user_id: i64) -> DbResult<Vec<FileVersionDto>>;
}

pub(crate) struct FileRepositoryImpl{
    pool: Arc<Pool>,
    db: Arc<dyn DatabaseAccess + Send + Sync>,
}

pub(crate) fn file_repository_new<DA>(pool: Arc<Pool>, db: Arc<DA>) -> impl FileRepository
where DA: DatabaseAccess + Send + Sync + 'static {
    FileRepositoryImpl {
        pool,
        db,
    }
}

#[async_trait]
impl FileRepository for FileRepositoryImpl {
    async fn get_file_list(&self, parent_id: i64, user_id: i64, sorting: &Sorting) -> DbResult<Vec<DisplayFileNode>> {
        let sql = r#"select fn.id, fn.title, 
        fn.parent_id, fn.node_type, fn.mime_type,
        fn.modified_at, fn.node_size, fn.node_version,
        case 
            when ffn.id is null then false
            else true
        end is_favorite
    from file_nodes fn
    left join favorite_nodes ffn on fn.id = ffn.id and fn.user_id = ffn.user_id  
    where fn.user_id = $1 and fn.parent_id = $2
    order by node_type"#;
        let sorted_sql = format!("{}, {}", sql, sorting.build_order_by());
        match self.db.query(&self.pool,  &sorted_sql, &[&user_id, &parent_id]).await {
            Ok(rows) => {
                let nodes = rows.iter().map(|row| DisplayFileNode {
                    id: row.get(0),
                    title: row.get(1),
                    parent_id: row.get(2),
                    node_type: row.get(3),
                    mime_type: row.get(4),
                    modified_at: row.get::<usize, chrono::DateTime<chrono::Utc>>(5).to_rfc3339(),
                    node_size: row.get(6),
                    node_version: row.get(7),
                    is_favorite: row.get(8)
                }).collect();
                return Ok(nodes);
            },
            Err(err) => {
                error!("{:?}", err);
                return Err(err);
            }
        }
    }

    async fn get_node(&self, id: i64, user_id: i64) -> DbResult<FileNodeDto> {
        let sql = format!(r#"select {} from file_nodes where user_id = $1 and id = $2"#, FileNodeDto::column_list());
        let row = self.db.query_one(&self.pool, &sql, &[&user_id, &id]).await?;
        let node = FileNodeDto::read_node(&row);
        Ok(node)
    }

    async fn find_node_by_name(&self, parent_id: i64, user_id: i64, title: Cow<'_, str>) -> DbResult<Option<FileNodeDto>> {
        let sql = format!(r#"select {} from file_nodes where user_id = $1 and parent_id = $2 and title = $3"#, FileNodeDto::column_list());
        let node = self.db.query_opt(&self.pool, &sql, &[&user_id, &parent_id, &title])
            .await?
            .map(|row| FileNodeDto::read_node(&row));
        Ok(node)
    }

    async fn add_node(&self, file_node: FileNodeDto) -> DbResult<i64>  {
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
            FileNodeDto::column_list(), get_file_node_id_sequence(user_id));
        let row = self.db.query_one(&self.pool, &sql, &[&user_id, &title, &parent_id, &node_type, &filesystem_path, &mime_type, &modified_at, &node_size, &node_version]).await?;
        let node_id: i64 = row.get(0);
        Ok(node_id)
    }

    async fn update_node(&self, old_node: &FileNodeDto, version_name: String, new_mime_type: &str, new_node_size: i64) -> DbResult<()> {
        // Copy current one to file_versions
        let copy_sql = r#"insert into file_versions (id, user_id, node_version, created_at, node_size, file_name)
        select fn.id, fn.user_id, fn.node_version, fn.modified_at, fn.node_size, $3
        from file_nodes fn 
        where fn.user_id = $1 and fn.id = $2"#;
        self.db.execute(&self.pool, copy_sql, &[&old_node.user_id, &old_node.id, &version_name]).await?;
    
        let update_sql = r#"update file_nodes
        set mime_type = $3,
            modified_at = $4,
            node_size = $5,
            node_version = $6
        where id = $1 and user_id = $2"#;
        self.db.execute(&self.pool, update_sql, &[
            &old_node.id,
            &old_node.user_id,
            &new_mime_type,
            &chrono::Utc::now(),
            &new_node_size,
            &(old_node.node_version + 1)
        ]).await?;
        Ok(())
    }

   
    /// Delete file node or empty folder.
    async fn permanent_delete(&self, id: i64, user_id: i64) -> DbResult<u64> {    
        let delete_sql = r#"delete from file_nodes where id = $2 and user_id = $1"#;
        let affected = self.db.execute(&self.pool, delete_sql, &[&user_id, &id]).await?;
        Ok(affected)
    }

    async fn get_parent_nodes(&self, parent_id: i64, user_id: i64) -> DbResult<Vec<ParentNode>> {
        let sql = r#"
        WITH RECURSIVE breadcrumbs_query AS ( 
            select id, title, parent_id, 0 as lev from file_nodes 
            where user_id = $1 and id = $2
            UNION ALL 
            select n.id, n.title, n.parent_id, lev-1 as lev from file_nodes n
            INNER JOIN breadcrumbs_query p ON p.parent_id = n.id
        )
        select id, title from breadcrumbs_query
        order by lev
        "#;
        let rows = self.db.query(&self.pool,  sql, &[&user_id, &parent_id]).await?;
        let nodes = rows.iter().map(|r| ParentNode { id: r.get(0), title: r.get(1) }).collect();
        return Ok(nodes);
    }

    async fn move_to_trash<TM>(&self, id: i64, user_id: i64, trash_mover: TM) -> DbResult<()>
    where TM: TrashMover + std::marker::Send {
        let get_nodes_sql = format!(r#"
        WITH RECURSIVE breadcrumbs_query AS (
            select n0.*, 0 as lvl from file_nodes n0 
            where user_id = $1 and id = $2
            UNION ALL 
            select n1.*, lvl + 1 as lvl from file_nodes n1
            INNER JOIN breadcrumbs_query p ON p.id = n1.parent_id
        )
        select {}, lvl,
              (select count(1) from file_versions fv where fv.id  = b.id and fv.user_id = b.user_id) versions 
        from breadcrumbs_query b
        order by lvl desc
        "#, FileNodeDto::column_list());

        let row_stream = self.db.query_raw(&self.pool, &get_nodes_sql, &[&user_id, &id]).await?;
        pin_mut!(row_stream);

        while let Some(row) = row_stream.try_next()
            .await
            .map_err(|_| crate::db::DbError::Fetch("Error reading next row, while deleting nodes".to_owned()))? {
            let file_node = FileNodeDto::read_node(&row);
            let versions_count: i64 = row.get(11);
            if versions_count > 0 {
                let _versions = self.get_file_versions(file_node.id, user_id).await?;
            }
            let trash_file_name_future = trash_mover.move_node_to_trash(&file_node);
            let trash_file_name_res = trash_file_name_future.await;

            match trash_file_name_res {
                Ok(trash_file_name) => {
                    if let Err(err) = self.try_move_file_node_to_trash(&file_node, &trash_file_name).await {
                        // Catch case if can not move record from file_nodes -> trash box to restore file.                        
                        error!("Unable to move file to trash table. [Error: {:?}]", err);
                        todo!("trash_mover.restore_node_from_trash(&file_node).await.unwrap();");
                    }
                }
                Err(e) => {
                    error!("Unable to delete node: {}. [Error: {:?}]", file_node.filesystem_path, e);
                }
            }
        }
        Ok(())
    }

    async fn restore_from_trash<TM>(&self, id: i64, user_id: i64, trash_mover: TM) -> DbResult<()> where TM: TrashMover + std::marker::Send {
        todo!();
    }

    /// Make file node favorite
    async fn set_favorite(&self, id: i64, user_id: i64) -> DbResult<u64> {    
        let insert_favorite_sql = r#"INSERT INTO favorite_nodes (id, user_id) VALUES($1, $2)"#;
        match self.db.execute(&self.pool, insert_favorite_sql, &[&id, &user_id]).await {
            Ok(affected) => {
                Ok(affected)
            },
            Err(err) => {
                error!("{:?}", err);
                Err(err)
            },
        }
    }

    /// Unset file not as favorite
    async fn unset_favorite(&self, id: i64, user_id: i64) -> DbResult<u64> {    
        let delete_favorite_sql = r#"DELETE FROM favorite_nodes where user_id = $1 and id = $2"#;
        match self.db.execute(&self.pool, delete_favorite_sql, &[&user_id, &id]).await {
            Ok(affected) => {
                Ok(affected)
            },
            Err(err) => {
                error!("{:?}", err);
                Err(err)
            },
        }
    }

    async fn get_file_versions(&self, id: i64, user_id: i64) -> DbResult<Vec<FileVersionDto>> {
        let sql = format!(r#"select {} from file_versions fv where fn.user_id = $1 and fv.id = $2 order by fn.node_version"#, 
    FileVersionDto::column_list());
        match self.db.query(&self.pool, &sql, &[&user_id, &id]).await {
            Ok(rows) => {
                let nodes = rows.iter().map(FileVersionDto::read_node).collect();
                return Ok(nodes);
            },
            Err(err) => {
                error!("{:?}", err);
                return Err(err);
            }
        }
    }
}

impl FileRepositoryImpl {
    async fn try_move_file_node_to_trash(&self, file_node: &FileNodeDto, trash_file_name: &str) -> DbResult<()> {
        let mut connection = self.db.create_connection(&self.pool).await?;
        let transaction = connection.create_transaction().await?;
        match self.try_move_file_node_to_trash_tran(&transaction, file_node, trash_file_name).await {
            Ok(_) => transaction.commit().await?,
            Err(_) => transaction.rollback().await?,
        };
        Ok(())
    }

    async fn try_move_file_node_to_trash_tran<'a>(&self, transaction: &TransactionalDataAccess<'a>, file_node: &FileNodeDto, trash_file_name: &str) -> DbResult<()> {
        let insert_to_trash_sql = 
        format!(r#"INSERT INTO trash_box ({}) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#, DeletedNodeDto::column_list());

        let delete_sql = "delete from file_nodes fn where fn.user_id = $1 and fn.id = $2";
        transaction.execute( &insert_to_trash_sql, &[
            &file_node.id, &file_node.user_id, &file_node.title, &file_node.parent_id, &file_node.node_type,
            &file_node.filesystem_path, &file_node.mime_type, &chrono::Utc::now(), &file_node.node_size,
            &file_node.node_version, &trash_file_name
        ]).await?;
        
        transaction.execute(delete_sql, &[&file_node.user_id, &file_node.id]).await?;
        Ok(())
    }
}

fn get_file_node_id_sequence(user_id: i64) -> String {
    format!("file_nodes_user_{}", user_id)
}
