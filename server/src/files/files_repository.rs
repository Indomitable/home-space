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
    order by node_type"#, FileNodeDto::column_list("fn"));
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
        let sql = format!(r#"select {} from file_nodes fn where user_id = $1 and id = $2"#, FileNodeDto::column_list("fn"));
        let row = self.db.query_one(&sql, &[&self.user_id, &id]).await?;
        let node = FileNodeDto::read_node(&row);
        Ok(node)
    }

    pub(crate) async fn get_nodes(&self, node_ids: &Vec<i64>) -> DbResult<Vec<FileNodeDto>> {
        let sql = format!(r#"select {} from file_nodes fn where user_id = $1 and id = ANY($2)"#, FileNodeDto::column_list("fn"));
        let nodes = self.db.query(&sql, &[&self.user_id, &node_ids])
            .await?
            .iter()
            .map(FileNodeDto::read_node)
            .collect();
        Ok(nodes)
    }

    pub(crate) async fn get_node_by_path(&self, path: &str) -> DbResult<Option<FileNodeDto>> {
        let sql = format!(r#"select {} from file_nodes fn where user_id = $1 and filesystem_path = $2"#, FileNodeDto::column_list("fn"));
        let node =
            self.db.query_opt(&sql, &[&self.user_id, &path])
                .await?
                .map(|row| FileNodeDto::read_node(&row));
        Ok(node)
    }

    pub(crate) async fn get_node_by_title(&self, parent_id: i64, title: &str) -> DbResult<Option<FileNodeDto>> {
        let sql = format!(r#"select {}
            from file_nodes fn
            where fn.user_id = $1 and fn.parent_id = $2 and fn.title = $3"#, FileNodeDto::column_list("fn"));

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
        let sql = format!(r#"insert into file_nodes (id, user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size, node_version)
        values (nextval('{}'), $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id"#,
            get_file_node_id_sequence(self.user_id));
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
        select {} from breadcrumbs_query fn
        order by lev
        "#, FileNodeDto::column_list("fn"));
        let rows = self.db.query(&sql, &[&self.user_id, &id]).await?;
        let nodes = rows.iter().map(FileNodeDto::read_node).collect();
        Ok(nodes)
    }

    pub(crate) async fn get_child_nodes(&self, parent_id: i64) -> DbResult<Vec<FileNodeDto>> {
        let sql = format!("select {} from file_nodes fn where user_id = $1 and parent_id = $2", FileNodeDto::column_list("fn"));
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
        select {} from breadcrumbs_query fn
        order by lev
        "#, FileNodeDto::column_list("fn"));
        let rows = self.db.query(&sql, &[&self.user_id, &parent_id]).await?;
        let nodes = rows.iter().map(FileNodeDto::read_node).collect();
        Ok(nodes)
    }

    pub(crate) async fn rename_node(&self, node: &FileNodeDto, path: &str, title: &str) -> DbResult<()> {
        let update_sql = r#"update file_nodes
        set filesystem_path = $3,
            title = $4
        where user_id = $1 and id = $2"#;
        self.db.execute(update_sql, &[&node.user_id, &node.id, &path, &title]).await?;
        Ok(())
    }
}

fn get_file_node_id_sequence(user_id: i64) -> String {
    format!("file_nodes_user_{}", user_id)
}
