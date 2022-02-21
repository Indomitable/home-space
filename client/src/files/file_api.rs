
use std::{cell::RefCell, rc::Rc};

use wasm_bindgen_futures::spawn_local;
use yew::{prelude::*, suspense::Suspension};

use home_space_contracts::files::FileNode;

use crate::api::api_service::{RequestInitBuilder, ResponseReader, METHOD_GET};

pub struct NodeState {
    s: Suspension,
    parent_id: i64,
    nodes: Rc<RefCell<Option<Vec<FileNode>>>>
}

impl NodeState {
    fn new(parent_id: i64, access_token: &str) -> Self {
        let (s, handle) = Suspension::new();
        let value: Rc<RefCell<Option<Vec<FileNode>>>> = Rc::default();

        {
            let value = value.clone();
            let token = access_token.to_owned();
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
                    let mut value = value.borrow_mut();
                    *value = Some(nodes);
                }
                handle.resume();
            });
        }

        Self { s, nodes: value, parent_id: parent_id }
    }
}

impl PartialEq for NodeState {
    fn eq(&self, rhs: &Self) -> bool {
        self.s == rhs.s && self.parent_id == rhs.parent_id
    }
}

#[hook]
pub fn use_nodes2(parent_id: i64, access_token: &str) -> yew::suspense::SuspensionResult<Vec<FileNode>> {
    let state = use_state(|| NodeState::new(parent_id, access_token));

    if state.parent_id != parent_id {
        state.set(NodeState::new(parent_id, access_token));
    }

    let result = match *state.nodes.borrow() {
        Some(ref m) => {
            Ok(m.clone())
        }
        None => {
            Err(state.s.clone())
        }
    };

    result
}

struct FileNodesState {
    parent_id: i64,
    nodes: Vec<FileNode>
}

#[hook]
pub fn use_nodes(
    parent_id: i64,
    access_token: &str,
) -> yew::suspense::SuspensionResult<Vec<FileNode>> {
    let state = use_state(|| None as Option<FileNodesState>);

    if let Some(FileNodesState{ parent_id: p, nodes: _ }) = *state {
        // if we state has already loaded some nodes, but current parent is different load new
        if p != parent_id {
            state.set(None);
        }
    }
    
    match *state {
        Some(ref nodes) => Ok(nodes.nodes.clone()),
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
                        state.set(Some(FileNodesState { 
                            nodes,
                            parent_id
                        }))
                    }
                    handler.resume();
                });
            }
            Err(s)
        }
    }
}
