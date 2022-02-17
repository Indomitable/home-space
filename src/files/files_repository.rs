use actix_web::web;
use deadpool_postgres::{Pool, tokio_postgres::Row};
use serde::Serialize;

use crate::db::{query, query_one, execute, DbResult};

#[derive(Serialize)]
pub struct FileNode {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub parent_id: Option<i64>,
    pub node_type: i16,
    pub filesystem_path: String,
    pub mime_type: Option<String>
}

pub async fn fetch_top_nodes(pool: &web::Data<Pool>, user_id: i64) -> DbResult<Vec<FileNode>> {
    let rows= query(pool, "select id, user_id, title, parent_id, node_type, filesystem_path, mime_type from file_nodes fn where fn.parent_id is null and user_id = $1", &[&user_id]).await?;
    let nodes = read_file_nodes(rows);
    return Ok(nodes);
}

pub async fn fetch_nodes(pool: &web::Data<Pool>, parent_id: i64, user_id: i64) -> DbResult<Vec<FileNode>> {
    let rows= query(pool, "select id, user_id, title, parent_id, node_type, filesystem_path, mime_type from file_nodes fn where fn.parent_id = $2 and user_id = $1", &[&user_id, &parent_id]).await?;
    let nodes = read_file_nodes(rows);
    return Ok(nodes);
}

pub async fn fetch_node(pool: &web::Data<Pool>, id: i64, user_id: i64) -> DbResult<FileNode> {
    let row= query_one(pool, "select id, user_id, title, parent_id, node_type, filesystem_path, mime_type from file_nodes fn where fn.id = $2 and user_id = $1", &[&user_id, &id]).await?;
    let node = read_file_node(&row);
    return Ok(node);
}

pub async fn add_node(pool: &web::Data<Pool>, file_node: FileNode) -> DbResult<u64>  {
    let FileNode {
        user_id,
        title,
        parent_id,
        node_type,
        filesystem_path,
        mime_type,
        ..
    } = file_node;
    let sql = r#"insert into file_nodes (user_id, title, parent_id, node_type, filesystem_path, mime_type)
    values ($1, $2, $3, $4, $5, $6)"#;
    let affected = execute(pool, sql, &[&user_id, &title, &parent_id, &node_type, &filesystem_path, &mime_type]).await?;
    Ok(affected)
}

fn read_file_nodes(rows: Vec<Row>) -> Vec<FileNode> {
    rows.iter().map(|r| read_file_node(r)).collect()
}

fn read_file_node(row: &Row) -> FileNode {
    FileNode {
        id: row.get(0),
        user_id: row.get(1),
        title: row.get(2),
        parent_id: row.get(3),
        node_type: row.get(4),
        filesystem_path: row.get(5),
        mime_type: row.get(6)
    }
}
