use home_space_contracts::files::{
    DisplayFileNode,
    ParentNode,
    CreateFolderRequest
};
use home_space_contracts::favorites::UpdateFavoriteRequest;

use crate::api::api_service::{
    RequestInitBuilder, 
    ResponseReader,
    FetchError,
    METHOD_GET, 
    METHOD_POST,
    METHOD_PUT
};

pub async fn load_file_nodes(parent_id: i64, token: &str) -> Result<Vec<DisplayFileNode>, FetchError>  {
    let url = format!("/api/files/get_nodes?parent_id={}", parent_id);
    let reader: ResponseReader = RequestInitBuilder::<()>::new()
        .set_method(METHOD_GET)
        .set_url(url)
        .set_access_token(&token)
        .fetch()
        .await
        .into();
    return reader.as_obj::<Vec<DisplayFileNode>>().await
}

pub async fn load_breadcrumbs(parent_id: i64, token: &str) -> Result<Vec<ParentNode>, FetchError>  {
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
        .set_url(url)
        .set_access_token(&token)
        .set_data(&payload)
        .fetch()
        .await;
}

pub async fn toggle_favorite(node_id: i64, token: &str, value: bool) {
    let url = if value { "/api/files/set_favorite" } else { "/api/files/unset_favorite" };
    RequestInitBuilder::<UpdateFavoriteRequest>::new()
        .set_method(METHOD_POST)
        .set_url(url)
        .set_access_token(&token)
        .set_data(&UpdateFavoriteRequest { id: node_id })
        .fetch()
        .await;
}

pub async fn get_file(node_id: i64, token: &str) -> Result<(), FetchError> {
    let url = format!("/api/files/get_file/{}", node_id);
    let reader: ResponseReader = RequestInitBuilder::<()>::new()
        .set_method(METHOD_GET)
        .set_url(&url)
        .set_access_token(&token)
        .fetch()
        .await
        .into();
    let blob = reader.as_binary().await?;

    Ok(())
    // reader.as_obj::<Vec<ParentNode>>().await
}