use std::borrow::Cow;
use wasm_bindgen_futures::spawn_local;

use super::file_repository as repo;

#[derive(Debug, PartialEq, Clone)]
pub struct NodeActions {
    token: String
}

impl NodeActions {
    pub fn new<'a>(token: Cow<'a, str>) -> Self {
        Self {
            token: token.into_owned()
        }
    }

    pub fn create_folder(&self, parent_id: i64, name: String) {
        let token = self.token.clone();
        //let name = name.to_owned();
        spawn_local(async move {
            repo::create_folder(parent_id, &token, &name).await;
        });
    }

    pub fn toggle_favorite(&self, node_id: i64, value: bool) {
        let token = self.token.clone();
        spawn_local(async move {
            repo::toggle_favorite(node_id, &token, value).await;
        });
    }
}