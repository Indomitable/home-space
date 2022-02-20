use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct FileNode {
    pub id: i64,
    pub title: String,
    pub parent_id: Option<i64>,
    pub node_type: i16,
    pub mime_type: Option<String>
}
