// use std::future::Future;

// use wasm_bindgen_futures::spawn_local;
// use yew::{suspense::{Suspension, SuspensionResult}, Reducible, use_reducer};

// use home_space_contracts::files::FileNode;
// use crate::api::api_service::{get, ApiError};

// #[derive(PartialEq)]
// pub struct NodesState {
//     s: Suspension,
//     file_nodes: Vec<FileNode>
// }

// pub enum NodesFetch {
//     NodesFetched(Vec<FileNode>)
// }

// impl NodesState {
//     fn new(parent_id: i64) -> Self {
//         let (s, handle) = Suspension::new();
//         Self {
//             s,
//             file_nodes: Vec::default()
//         }

//         // state
//     }
// }

// impl Reducible for NodesState {
//     type Action = NodesFetch;

//     fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
//         match action {
//             NodesFetch::NodesFetched(nodes) => {
//                 Self {
//                     file_nodes: nodes,
//                     s: self.s.clone()
//                 }.into()
//             },
//         }
//     }
// }

// async fn use_nodes(parent_id: i64) -> SuspensionResult<Vec<FileNode>> {
//     let (suspention, handle) = Suspension::new();
//     let url = format!("/api/files/get_nodes/{}", parent_id);
//     let future = get::<Vec<FileNode>>(url.as_str());
//     future
//     Err(suspention)
// }