use home_space_contracts::{files::{FileNode, ParentNode}, favorites::UpdateFavoriteRequest};
use wasm_bindgen::UnwrapThrowExt;

use crate::api::api_service::{RequestInitBuilder, ResponseReader, METHOD_GET, ResponseReadError, METHOD_POST};

pub async fn load_file_nodes(parent_id: i64, token: &str) -> Result<Vec<FileNode>, ResponseReadError>  {
    let url = format!("/api/files/get_nodes?parent_id={}", parent_id);
    let reader: ResponseReader = RequestInitBuilder::<()>::new()
        .set_method(METHOD_GET)
        .set_url(&url)
        .set_access_token(&token)
        .fetch()
        .await
        .into();
    return reader.as_obj::<Vec<FileNode>>().await
}

pub async fn load_breadcrumbs(parent_id: i64, token: &str) -> Result<Vec<ParentNode>, ResponseReadError>  {
    let url = format!("/api/files/parents/{}", parent_id);
    let reader: ResponseReader = RequestInitBuilder::<()>::new()
        .set_method(METHOD_GET)
        .set_url(&url)
        .set_access_token(&token)
        .fetch()
        .await
        .into();
    reader.as_obj::<Vec<ParentNode>>().await
}


pub async fn toggle_favorite(node_id: i64, token: &str, value: bool) {
    let url = if value { "/api/files/set_favorite" } else { "/api/files/unset_favorite" };
    RequestInitBuilder::<String>::new()
        .set_method(METHOD_POST)
        .set_url(&url)
        .set_access_token(&token)
        .set_data(&serde_json::to_string(&UpdateFavoriteRequest { id: node_id }).unwrap_throw())
        .fetch()
        .await;
}
