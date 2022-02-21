use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct FileNode {
    pub id: i64,
    pub title: String,
    pub parent_id: Option<i64>,
    pub node_type: i16,
    pub mime_type: String,
    pub modified_at: String,
    pub node_size: i64
}


#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ParentNode {
    pub id: i64,
    pub title: String,
}
