use crate::files::db::file_node::FileNodeDto;
use super::DbModel;

pub(crate) struct FileVersionDto {
    pub id: i64,
    pub user_id: i64,
    pub node_version: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub node_size: i64,
    pub file_name: String,
}

impl DbModel for FileVersionDto {
    fn read_node(row: &deadpool_postgres::tokio_postgres::Row) -> Self {
        FileVersionDto {
            id: row.get(0),
            user_id: row.get(1),
            node_version: row.get(2),
            created_at: row.get(3),
            node_size: row.get(4),
            file_name: row.get(5),        
        }
    }

    fn column_list() -> &'static str {
        "id, user_id, node_version, created_at, node_size, file_name"
    }
}

impl FileVersionDto {
    pub(crate) fn from(node: &FileNodeDto, file_name: &str) -> Self {
        Self {
            id: node.id,
            user_id: node.user_id,
            node_version: node.node_version,
            created_at: chrono::Utc::now(),
            node_size: node.node_size,
            file_name: file_name.to_owned(),
        }
    }
}
