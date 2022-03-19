
use std::rc::Rc;
use log::debug;
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;

use home_space_contracts::files::DisplayFileNode;

use crate::app_context::{AppContext, AuthContext};
use crate::dispatcher::Subscriber;
use crate::utils::dispatcher_helpers::{subscribe, unsubscribe};
use super::file_list_component::FileList;
use super::toolbox::file_actions::FileActions;
use super::breadcrumbs::breadcrumbs_file_navigation::BreadcumbsFileNav;
use super::file_repository::load_file_nodes;
use super::node_actions::NodeActions;

#[derive(Properties, PartialEq)]
pub struct FilesViewProps {
    pub parent_id: i64,
}

pub struct FilesView {
    pub current_parent_id: i64,
    pub nodes: Option<Vec<DisplayFileNode>>,
    
    node_actions: Rc<NodeActions>,

    refresh_subsriber: Rc<Subscriber>
}

pub enum FileViewActions {
    FetchFileNodes,
    FileNodesFetched(Vec<DisplayFileNode>),
    FileNodesFetchedFailed
}

impl FilesView {

    fn get_token(ctx: &Context<Self>) -> String {
        let context = ctx.link().context::<AppContext>(Callback::noop()).unwrap_throw();
        let token = match &context.0.auth_context {
            AuthContext::Authenticated(user) => user.access_token.token.clone(),
            AuthContext::NotAuthenticated => panic!("Should be authenticated")
        };
        token
    }

    fn get_props(&self, ctx: &Context<Self>) -> (i64, String) {
        let token = FilesView::get_token(&ctx);
        let parent_id = ctx.props().parent_id;

        (parent_id, token)
    }

    fn load_nodes(&self, ctx: &Context<Self>) {
        debug!("Load Nodes");
        let cb = ctx.link().callback(|_| FileViewActions::FetchFileNodes);
        cb.emit(());
    }
}

impl Component for FilesView {
    type Message = FileViewActions;
    type Properties = FilesViewProps;

    fn create(ctx: &Context<Self>) -> Self {
        let cb = ctx.link().callback(|_: ()| FileViewActions::FetchFileNodes);
        let subscriber = subscribe(&ctx, "refresh-files-view".into(), cb);

        let obj = Self {
            current_parent_id: ctx.props().parent_id,
            nodes: None,
            node_actions: NodeActions::new(FilesView::get_token(&ctx).into()).into(),
            refresh_subsriber: subscriber
        };
        obj.load_nodes(ctx);      
        obj
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            FileViewActions::FetchFileNodes => {
                let (parent_id, token) = self.get_props(ctx);
                let cb = ctx.link().callback_future(|props: (i64, String)| async move {
                    let nodes = load_file_nodes(props.0, &props.1).await;
                    match nodes {
                        Ok(nodes) => FileViewActions::FileNodesFetched(nodes),
                        Err(_) => FileViewActions::FileNodesFetchedFailed
                    }
                });
                cb.emit((parent_id, token));
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
        let (parent_id, token) = self.get_props(ctx);            

        html!{
            <>
                <FileActions parent_id={parent_id} node_actions={&self.node_actions.clone()} />
                <BreadcumbsFileNav parent_id={parent_id} access_token={token.clone()} />
                if let Some(nodes) = &self.nodes {
                   <FileList nodes={nodes.clone()} node_actions={&self.node_actions.clone()} />
                }
            </>
        }
    }

    fn destroy(&mut self, ctx: &Context<Self>) {
        unsubscribe(ctx, &self.refresh_subsriber);
    }

}
