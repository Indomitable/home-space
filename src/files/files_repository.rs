use actix_web::web;
use deadpool_postgres::{Pool, tokio_postgres::Row};

use crate::db::{query, query_one, execute, DbResult};

pub const NODE_TYPE_FOLDER: i16 = 0;
pub const NODE_TYPE_FILE: i16 = 1;

pub struct FileNodeDto {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub parent_id: Option<i64>,
    pub node_type: i16,
    pub filesystem_path: String,
    pub mime_type: Option<String>
}

pub async fn fetch_nodes(pool: &web::Data<Pool>, parent_id: i64, user_id: i64) -> DbResult<Vec<FileNodeDto>> {
    let sql = r#"select id, user_id, title, parent_id, node_type, filesystem_path, mime_type from file_nodes
                      where parent_id = $2 and user_id = $1"#;    
    let rows = query(pool,  sql, &[&user_id, &parent_id]).await?;
    let nodes = read_file_nodes(rows);
    return Ok(nodes);
}

pub async fn fetch_node(pool: &web::Data<Pool>, id: i64, user_id: i64) -> DbResult<FileNodeDto> {
    let sql = r#"select id, user_id, title, parent_id, node_type, filesystem_path, mime_type 
                    from file_nodes
                    where id = $2 and user_id = $1"#;
    let row= query_one(pool, sql, &[&user_id, &id]).await?;
    let node = read_file_node(&row);
    return Ok(node);
}

pub async fn add_node(pool: &web::Data<Pool>, file_node: FileNodeDto) -> DbResult<u64>  {
    let FileNodeDto {
        user_id,
        title,
        parent_id,
        node_type,
        filesystem_path,
        mime_type,
        ..
    } = file_node;
    let sql = format!(r#"insert into file_nodes (id, user_id, title, parent_id, node_type, filesystem_path, mime_type)
    values (nextval('{}'), $1, $2, $3, $4, $5, $6)"#, get_file_node_id_sequence(user_id));
    let affected = execute(pool, &sql, &[&user_id, &title, &parent_id, &node_type, &filesystem_path, &mime_type]).await?;
    Ok(affected)
}

pub async fn delete_node(pool: &web::Data<Pool>, id: i64, node_type: i16, user_id: i64) -> DbResult<u64> {
    let delete_sql = r#"delete from file_nodes where id = $2 and user_id = $1"#;
    let affected: u64;
    if node_type == NODE_TYPE_FILE {
        affected = execute(pool, delete_sql, &[&user_id, &id]).await?;
    } else {
        todo!("Add folder delete");
    }
    Ok(affected)
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
        mime_type: row.get(6)
    }
}


fn get_file_node_id_sequence(user_id: i64) -> String {
    format!("file_nodes_user_{}", user_id)
}
