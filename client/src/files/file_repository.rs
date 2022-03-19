use wasm_bindgen::UnwrapThrowExt;

use home_space_contracts::files::{
    FileNode,
    ParentNode,
    CreateFolderRequest
};
use home_space_contracts::favorites::UpdateFavoriteRequest;

use crate::api::api_service::{
    RequestInitBuilder, 
    ResponseReader,
    ResponseReadError,
    METHOD_GET, 
    METHOD_POST,
    METHOD_PUT
};

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


pub async fn create_folder<'a>(parent_id: i64, token: &'a str, name: &'a str) {
    let url = "/api/files/create_folder";
    let payload = CreateFolderRequest { parent_id, name: name.to_owned() };
    RequestInitBuilder::<CreateFolderRequest>::new()
        .set_method(METHOD_PUT)
        .set_url(&url)
        .set_access_token(&token)
        .set_data(&payload)
        .fetch()
        .await;
}

pub async fn toggle_favorite(node_id: i64, token: &str, value: bool) {
    let url = if value { "/api/files/set_favorite" } else { "/api/files/unset_favorite" };
    RequestInitBuilder::<UpdateFavoriteRequest>::new()
        .set_method(METHOD_POST)
        .set_url(&url)
        .set_access_token(&token)
        .set_data(&UpdateFavoriteRequest { id: node_id })
        .fetch()
        .await;
}
