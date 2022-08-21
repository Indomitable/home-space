#![allow(dead_code)]
use super::DbModel;

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

    fn column_list() -> &'static str {
        "id, user_id, title, parent_id, node_type, filesystem_path, mime_type, deleted_at, node_size, node_version, file_name"
    }
}