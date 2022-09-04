#![allow(dead_code)]
use super::{DbModel, file_node::FileNodeDto};

pub(crate) struct DeletedNodeDto {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub parent_id: Option<i64>,
    pub node_type: i16,
    pub filesystem_path: String,
    pub mime_type: String,
    pub deleted_at: chrono::DateTime<chrono::Utc>,
    pub node_size: i64,
    pub node_version: i32,
    pub file_name: String,
}

impl DeletedNodeDto {
    pub(crate) fn new(node: &FileNodeDto, file_name: &str) -> Self {
        Self {
            id: node.id,
            user_id: node.user_id,
            title: node.title.clone(),
            parent_id: node.parent_id,
            node_type: node.node_type,
            filesystem_path: node.filesystem_path.clone(),
            mime_type: node.mime_type.clone(),
            deleted_at: chrono::Utc::now(),
            node_size: node.node_size,
            node_version: node.node_version,
            file_name: file_name.to_owned(),
        }
    }
}

impl DbModel for DeletedNodeDto {
    fn read_node(row: &deadpool_postgres::tokio_postgres::Row) -> Self {
        DeletedNodeDto {
            id: row.get(0),
            user_id: row.get(1),
            title: row.get(2),
            parent_id: row.get(3),
            node_type: row.get(4),
            filesystem_path: row.get(5),
            mime_type: row.get(6),
            deleted_at: row.get(7),
            node_size: row.get(8),
            node_version: row.get(9),
            file_name: row.get(10),
        }
    }

    fn column_list(table_alias: &str) -> String {
        format!(r#"{table_alias}.id,
        {table_alias}.user_id,
        {table_alias}.title,
        {table_alias}.parent_id,
        {table_alias}.node_type,
        {table_alias}.filesystem_path,
        {table_alias}.mime_type,
        {table_alias}.deleted_at,
        {table_alias}.node_size,
        {table_alias}.node_version,
        {table_alias}.file_name"#, table_alias=table_alias)
    }
}