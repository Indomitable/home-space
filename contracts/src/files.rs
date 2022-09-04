use serde::{Serialize, Deserialize};

pub const NODE_TYPE_FOLDER: i16 = 0;
pub const NODE_TYPE_FILE: i16 = 1;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DisplayFileNode {
    pub id: i64,
    pub title: String,
    pub parent_id: Option<i64>,
    pub node_type: i16,
    pub mime_type: String,
    pub modified_at: String,
    pub node_size: i64,
    pub node_version: i32,
    pub is_favorite: bool
}

impl PartialEq for DisplayFileNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileNode {
    pub id: i64,
    pub title: String,
    pub parent_id: Option<i64>,
    pub node_type: i16,
    pub mime_type: String,
    pub modified_at: String,
    pub node_size: i64,
    pub node_version: i32,
}

impl PartialEq for FileNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParentNode {
    pub id: i64,
    pub title: String,
}

impl PartialEq for ParentNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateNodeResponse {
    pub id: i64
}

impl PartialEq for CreateNodeResponse {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct CreateFolderRequest {
    pub parent_id: i64,
    pub name: String
}

pub const PASTE_OPERATION_MOVE: i16 = 1;
pub const PASTE_OPERATION_COPY: i16 = 2;

#[derive(Serialize, Deserialize, Clone)]
pub struct PasteNodesRequest {
    pub nodes: Vec<i64>,
    pub operation: i16,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RenameNodeRequest {
    pub node_id: i64,
    pub name: String,
}
