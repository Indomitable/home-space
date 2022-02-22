use home_space_contracts::files::{FileNode, ParentNode};

use crate::api::api_service::{RequestInitBuilder, ResponseReader, METHOD_GET, ResponseReadError};

pub async fn load_file_nodes(parent_id: i64, token: &str) -> Result<Vec<FileNode>, ResponseReadError>  {
    let url = format!("/api/files/get_nodes/{}", parent_id);
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
