use log::debug;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use crate::{user::secure_component::use_user_context, api::api_service::{RequestInitBuilder, ResponseReader, METHOD_GET}};

use home_space_contracts::files::FileNode;

pub struct HomeState {
    fetching: bool,
    file_nodes: Vec<FileNode>
}

pub enum HomeActions {
    FetchFileNodes(i64),
    FileNodesFetched(Vec<FileNode>)
}

impl Reducible for HomeState {
    type Action = HomeActions;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            HomeActions::FetchFileNodes(parent_id) => {
                HomeState {
                    fetching: true,
                    file_nodes: self.file_nodes.clone()
                }.into()
            },
            HomeActions::FileNodesFetched(nodes) => {
                HomeState {
                    fetching: false,
                    file_nodes: nodes
                }.into()
            },
        }
    }
}


#[function_component(Home)]
pub fn home() -> Html {
//    html!(<div>{"hello"}</div>)
    let user = use_user_context();
    // let state = use_reducer(|| HomeState {
    //     fetching: false,
    //     file_nodes: Vec::new()
    // });
    let state = use_state(|| vec![]);
    {
        let state = state.clone();
        let access_token = user.access_token;
        use_effect_with_deps(move |_| {
            let state = state.clone();
            spawn_local(async move {
                let url = format!("/api/files/get_nodes/{}", 0);
                let reader: ResponseReader = RequestInitBuilder::<()>::new()
                                .set_method(METHOD_GET)
                                .set_url(&url)
                                .set_access_token(&access_token.token)
                                .fetch().await.into();
                
                if let Ok(nodes) = reader.as_obj().await {
                    state.set(nodes);
                }
            });
            || ()
        }, ());    
    }
    let message = format!("Wellcome {}", user.user_name);
    // state.dispatch(HomeActions::FetchFileNodes(0));
    html! {
        <div>
            { message }
            if state.len() == 0 {
                { "Fetching nodes" }
            } else {
                <ul>
                    {
                        state.iter().map(|node: &FileNode| {
                            html!(<li key={node.id}>{node.title.clone()}</li>)
                        }).collect::<Html>()
                    }
                </ul>
            }
        </div>
    }
}


