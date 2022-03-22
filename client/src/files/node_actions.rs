use std::borrow::Cow;

use super::{file_repository as repo, node_selection::NodeSelection};

#[derive(Debug, PartialEq, Clone)]
pub struct NodeActions {
    token: String,
    selection: NodeSelection
}

impl NodeActions {
    pub fn new<'a>(token: Cow<'a, str>) -> Self {
        Self {
            token: token.into_owned(),
            selection: NodeSelection::new()
        }
    }

    pub async fn create_folder(&self, parent_id: i64, name: String) {
        repo::create_folder(parent_id, &self.token, &name).await;
    }

    pub async fn toggle_favorite(&self, node_id: i64, value: bool) {
        repo::toggle_favorite(node_id, &self.token, value).await;
    }
}
