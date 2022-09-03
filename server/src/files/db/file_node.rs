use std::path::PathBuf;
use super::DbModel;

#[derive(Debug, Clone)]
pub(crate) struct FileNodeDto {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub parent_id: Option<i64>,
    pub node_type: i16,
    pub filesystem_path: String,
    pub mime_type: String,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub node_size: i64,
    pub node_version: i32,
}

impl DbModel for FileNodeDto {
    fn read_node(row: &deadpool_postgres::tokio_postgres::Row) -> Self {
        FileNodeDto {
            id: row.get(0),
            user_id: row.get(1),
            title: row.get(2),
            parent_id: row.get(3),
            node_type: row.get(4),
            filesystem_path: row.get(5),
            mime_type: row.get(6),
            modified_at: row.get(7),
            node_size: row.get(8),
            node_version: row.get(9),
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
        {table_alias}.modified_at,
        {table_alias}.node_size,
        {table_alias}.node_version"#, table_alias=table_alias)
    }
}

impl FileNodeDto {
    pub(crate) fn copy(other: &FileNodeDto, parent_node: &FileNodeDto) -> Self {
        let path = PathBuf::from(&parent_node.filesystem_path.trim_start_matches('/'))
            .join(&other.title)
            .to_str()
            .expect("Node title should be utf-8")
            .to_owned();
        Self {
            id: 0, // Set it to zero we don't know what will be.
            user_id: other.user_id,
            title: other.title.clone(),
            parent_id: Some(parent_node.id),
            node_type: other.node_type,
            filesystem_path: path,
            mime_type: other.mime_type.clone(),
            modified_at: other.modified_at,
            node_size: other.node_size,
            node_version: 1, // Loose the version history when copy. It will remain on the original file.
            // TODO: copy the versions in future.
        }
    }
}
