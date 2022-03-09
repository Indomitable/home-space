use actix_web::web;
use deadpool_postgres::{Pool, tokio_postgres::Row};

use home_space_contracts::files::{ParentNode, NODE_TYPE_FILE};

use crate::db::{query, query_one, execute, DbResult};

pub struct FileNodeDto {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub parent_id: Option<i64>,
    pub node_type: i16,
    pub filesystem_path: String,
    pub mime_type: String,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub node_size: i64
}

pub async fn fetch_nodes(pool: &web::Data<Pool>, parent_id: i64, user_id: i64) -> DbResult<Vec<FileNodeDto>> {
    let sql = r#"select id, user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size from file_nodes
                      where parent_id = $2 and user_id = $1
                      order by node_type, title"#;
    let rows = query(pool,  sql, &[&user_id, &parent_id]).await?;
    let nodes = read_file_nodes(rows);
    return Ok(nodes);
}

pub async fn fetch_node(pool: &web::Data<Pool>, id: i64, user_id: i64) -> DbResult<FileNodeDto> {
    let sql = r#"select id, user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size
                    from file_nodes
                    where id = $2 and user_id = $1"#;
    let row= query_one(pool, sql, &[&user_id, &id]).await?;
    let node = read_file_node(&row);
    return Ok(node);
}

pub async fn add_node(pool: &web::Data<Pool>, file_node: FileNodeDto) -> DbResult<i64>  {
    let FileNodeDto {
        user_id,
        title,
        parent_id,
        node_type,
        filesystem_path,
        mime_type,
        modified_at,
        node_size,
        ..
    } = file_node;
    let sql = format!(r#"insert into file_nodes (id, user_id, title, parent_id, node_type, filesystem_path, mime_type, modified_at, node_size)
    values (nextval('{}'), $1, $2, $3, $4, $5, $6, $7, $8) RETURNING id"#, get_file_node_id_sequence(user_id));
    let row = query_one(pool, &sql, &[&user_id, &title, &parent_id, &node_type, &filesystem_path, &mime_type, &modified_at, &node_size]).await?;
    let node_id: i64 = row.get(0);
    Ok(node_id)
}

pub async fn move_to_trash(pool: &web::Data<Pool>, id: i64, node_type: i16, user_id: i64) -> DbResult<u64> {
    let trash_insert_sql = r#"insert into trash_box t (id, user_id, title, parent_id, node_type, filesystem_path, mime_type, deleted_at, node_size)
    select fn.id, fn.user_id, fn.parent_id, fn.node_type, fn.filesystem_path, fn.mime_type, now() at time zone 'utc', fn.node_size from file_nodes fn"#;
    let delete_top_sql = "delete from file_nodes fn";
    let where_sql: &str;
    if node_type == NODE_TYPE_FILE {
        where_sql = "where fn.id = $2 and fn.user_id = $1";
    } else {
        where_sql = r#"where fn.id in (
        WITH RECURSIVE breadcrumbs_query AS ( 
            select id, title, parent_id, 0 as lev from file_nodes 
            where id = $2 and user_id = $1
            UNION ALL 
            select n.id, n.title, n.parent_id, lev+1 as lev from file_nodes n
            INNER JOIN breadcrumbs_query p ON p.id = n.parent_id
        )
        select id from breadcrumbs_query);
        "#;
    }
    let _ = execute(pool, &format!("{} {}", trash_insert_sql, where_sql), &[&user_id, &id]).await?;
    let deleted = execute(pool, &format!("{} {}", delete_top_sql, where_sql), &[&user_id, &id]).await?;
    Ok(deleted)
}

/// Delete file node or empty folder.
pub async fn permanent_delete(pool: &web::Data<Pool>, id: i64, user_id: i64) -> DbResult<u64> {    
    let delete_sql = r#"delete from file_nodes where id = $2 and user_id = $1"#;
    let affected = execute(pool, delete_sql, &[&user_id, &id]).await?;
    Ok(affected)
}

pub async fn get_parent_nodes(pool: &web::Data<Pool>, parent_id: i64, user_id: i64) -> DbResult<Vec<ParentNode>> {
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
    let rows = query(pool,  sql, &[&user_id, &parent_id]).await?;
    let nodes = rows.iter().map(|r| ParentNode { id: r.get(0), title: r.get(1) }).collect();
    return Ok(nodes);
}


fn read_file_nodes(rows: Vec<Row>) -> Vec<FileNodeDto> {
    rows.iter().map(|r| read_file_node(r)).collect()
}

fn read_file_node(row: &Row) -> FileNodeDto {
    FileNodeDto {
        id: row.get(0),
        user_id: row.get(1),
        title: row.get(2),
        parent_id: row.get(3),
        node_type: row.get(4),
        filesystem_path: row.get(5),
        mime_type: row.get(6),
        modified_at: row.get(7),
        node_size: row.get(8)
    }
}


fn get_file_node_id_sequence(user_id: i64) -> String {
    format!("file_nodes_user_{}", user_id)
}
