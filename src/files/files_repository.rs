use actix_web::web;
use deadpool_postgres::{Pool, tokio_postgres::Row};
use serde::Serialize;

use crate::db::query;

#[derive(Serialize)]
pub struct FileNode {
    id: i64,
    user_id: i64,
    title: String,
    parent_id: Option<i64>,
    node_type: i16,
    filesystem_path: String,
    mime_type: Option<String>
}

pub async fn fetch_top_nodes(pool: web::Data<Pool>, user_id: i64) -> Result<Vec<FileNode>, deadpool_postgres::PoolError> {
    let rows= query(pool, "select id, user_id, title, parent_id, node_type, filesystem_path, mime_type from file_nodes fn where fn.parent_id is null and user_id = $1", &[&user_id]).await?;
    let nodes = read_file_node(rows);
    return Ok(nodes);
}

pub async fn fetch_nodes(pool: web::Data<Pool>, parent_id: i64, user_id: i64) -> Result<Vec<FileNode>, deadpool_postgres::PoolError> {
    let rows= query(pool, "select id, user_id, title, parent_id, node_type, filesystem_path, mime_type from file_nodes fn where fn.parent_id = $2 and user_id = $1", &[&user_id, &parent_id]).await?;
    let nodes = read_file_node(rows);
    return Ok(nodes);
}

fn read_file_node(rows: Vec<Row>) -> Vec<FileNode> {
    let mut nodes: Vec<FileNode> = vec!();
    for row in rows {
        let node = FileNode {
            id: row.get(0),
            user_id: row.get(1),
            title: row.get(2),
            parent_id: row.get(3),
            node_type: row.get(4),
            filesystem_path: row.get(5),
            mime_type: row.get(6)
        };
        nodes.push(node);
    }
    nodes
}