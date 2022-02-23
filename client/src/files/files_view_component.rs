
use home_space_contracts::files::FileNode;
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;

use crate::app_context::{AppContext, AuthContext};
use super::file_list_component::FileList;
use super::actions::file_actions_component::FileActions;
use super::breadcrumbs::breadcrumbs_component::BreadcumbsFileNav;
use super::file_repository::load_file_nodes;

#[derive(Properties, PartialEq)]
pub struct FilesViewProps {
    pub parent_id: i64,
}

pub struct FilesView {
    pub current_parent_id: i64,
    pub nodes: Option<Vec<FileNode>>
}

pub enum FileViewActions {
    FetchFileNodes(i64, String),
    FileNodesFetched(Vec<FileNode>),
    FileNodesFetchedFailed
}

impl FilesView {

    fn get_props(&self, ctx: &Context<Self>) -> (i64, String) {
        let context = ctx.link().context::<AppContext>(Callback::noop()).unwrap_throw();
        let token = match &context.0.auth_context {
            AuthContext::Authenticated(user) => user.access_token.token.clone(),
            AuthContext::NotAuthenticated => panic!("Should be authenticated")
        };

        let parent_id = ctx.props().parent_id;

        (parent_id, token)
    }

    fn load_nodes(&self, ctx: &Context<Self>) {
        let (parent_id, token) = self.get_props(ctx);
        let cb = ctx.link().callback(|x: (i64, String)| FileViewActions::FetchFileNodes(x.0, x.1));
        cb.emit((parent_id, token.clone()));
    }
}

impl Component for FilesView {
    type Message = FileViewActions;
    type Properties = FilesViewProps;

    fn create(ctx: &Context<Self>) -> Self {
        let obj = Self {
            current_parent_id: ctx.props().parent_id,
            nodes: None
        };
        obj.load_nodes(ctx);
        obj
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            FileViewActions::FetchFileNodes( id, token ) => {
                let cb = ctx.link().callback_future(|props: (i64, String)| async move {
                    let nodes = load_file_nodes(props.0, &props.1).await;
                    match nodes {
                        Ok(nodes) => FileViewActions::FileNodesFetched(nodes),
                        Err(_) => FileViewActions::FileNodesFetchedFailed
                    }
                });
                cb.emit((id, token));
                false
            },
            FileViewActions::FileNodesFetched(nodes) => {
                self.nodes = Some(nodes);
                true
            },
            FileViewActions::FileNodesFetchedFailed => {
                false
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        if self.current_parent_id != ctx.props().parent_id {
            // Parent id is changed
            self.load_nodes(ctx);
            self.current_parent_id = ctx.props().parent_id
        }
        false // Do not render, it will happen in the update
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        log::debug!("Render Files View");

        let (parent_id, token) = self.get_props(ctx);            

        html!{
            <>
                <FileActions parent_id={parent_id} />
                <BreadcumbsFileNav parent_id={parent_id} access_token={token.clone()} />
                if let Some(nodes) = &self.nodes {
                   <FileList nodes={nodes.clone()} />
                }
            </>
        }
    }
}
