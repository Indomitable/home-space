
use wasm_bindgen_futures::spawn_local;
use yew::{prelude::*, suspense::Suspension};

use home_space_contracts::files::FileNode;

use crate::api::api_service::{RequestInitBuilder, ResponseReader, METHOD_GET};

#[hook]
pub fn use_nodes(
    parent_id: i64,
    access_token: &str,
) -> yew::suspense::SuspensionResult<Vec<FileNode>> {
    let state = use_state(|| None as Option<Vec<FileNode>>);
    
    match &*state {
        Some(nodes) => Ok(nodes.clone()),
        None => {
            let (s, handler) = Suspension::new();
            let token = access_token.to_owned();
            {
                spawn_local(async move {
                    let url = format!("/api/files/get_nodes/{}", parent_id);
                    let reader: ResponseReader = RequestInitBuilder::<()>::new()
                        .set_method(METHOD_GET)
                        .set_url(&url)
                        .set_access_token(&token)
                        .fetch()
                        .await
                        .into();
                    if let Ok(nodes) = reader.as_obj::<Vec<FileNode>>().await {
                        state.set(Some(nodes))
                    }
                    handler.resume();
                });
            }
            Err(s)
        }
    }
}
