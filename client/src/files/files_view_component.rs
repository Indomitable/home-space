
use std::cell::RefCell;
use std::rc::Rc;
use yew::prelude::*;

use home_space_contracts::files::DisplayFileNode;

use crate::dispatcher::Subscriber;
use crate::utils::auth_helpers::get_user_context;
use crate::utils::dispatcher_helpers::{subscribe, unsubscribe};
use super::file_list_component::FileList;
use super::node_state::NodesState;
use super::toolbox::file_actions::FileActions;
use super::breadcrumbs::breadcrumbs_file_navigation::BreadcumbsFileNav;
use super::file_repository::{load_file_nodes, create_folder, toggle_favorite};

#[derive(Properties, PartialEq)]
pub struct FilesViewProps {
    pub parent_id: i64,
}

pub struct FilesView {
    pub current_parent_id: i64,
    pub nodes: Option<Vec<DisplayFileNode>>,
    
    node_states: Rc<RefCell<NodesState>>,

    refresh_subsriber: Rc<Subscriber>
}

pub enum FileViewActions {
    FetchFileNodes,
    FileNodesFetched(Vec<DisplayFileNode>),
    FileNodesFetchedFailed,
    FileNodeSelectionChanged((i64, bool)),
    FileNodesCreateFolder(String),
    FileNodeFavoriteChanged((i64, bool)),
}

impl FilesView {

    fn get_token(ctx: &Context<Self>) -> String {
        let user_context = get_user_context(&ctx);
        user_context.access_token.token
    }

    fn get_props(&self, ctx: &Context<Self>) -> (i64, String) {
        let token = FilesView::get_token(&ctx);
        let parent_id = ctx.props().parent_id;

        (parent_id, token)
    }

    fn load_nodes(&self, ctx: &Context<Self>) {
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
            node_states: RefCell::new(NodesState::new()).into(),
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
                self.node_states.borrow_mut().fill_default(self.nodes.as_ref().unwrap());
                true
            },
            FileViewActions::FileNodesFetchedFailed => {
                false
            },
            FileViewActions::FileNodeSelectionChanged((node_id, selected)) => {
                if let Some(state) = self.node_states.borrow_mut().states.get_mut(&node_id) {
                    state.is_selected = selected;
                }
                true
            },
            FileViewActions::FileNodesCreateFolder(name) => {
                let (parent_id, token) = self.get_props(ctx);
                let cb = ctx.link().callback_future(|props: (i64, String, String)| async move {
                    let (parent_id, token, name) = props;
                    create_folder(parent_id, &token, &name).await;
                    FileViewActions::FetchFileNodes
                });
                cb.emit((parent_id, token, name));
                false
            },
            FileViewActions::FileNodeFavoriteChanged((node_id, is_favorite)) => {
                let (_, token) = self.get_props(ctx);
                let cb = ctx.link().callback_future(|props: (i64, String, bool)| async move {
                    let (node_id, token, is_favorite) = props;
                    toggle_favorite(node_id, &token, is_favorite).await;
                    FileViewActions::FetchFileNodes
                });
                cb.emit((node_id, token, is_favorite));
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
        
        let (favorite_nodes, regular_nodes) = match &self.nodes {
            Some(nodes) => {
                let mut favorite = Vec::new();
                let mut regular = Vec::new();
                for node in nodes.iter() {
                    if node.is_favorite {
                        favorite.push(node.clone());
                    } else {
                        regular.push(node.clone());
                    }
                }
                (favorite, regular)
            },
            None => (Vec::new(), Vec::new())
        };
        let has_favorites = favorite_nodes.len() > 0;


        let action_callback = ctx.link().callback(|action: FileViewActions| action);
        
        let selected_nodes = self.node_states.borrow().states.values().fold(0, |c, item| { 
            if item.is_selected {
                return c + 1;
            }
            return c;
        } );

        html!{
            <>
                <FileActions parent_id={parent_id} {selected_nodes} action_callback={action_callback.clone()} />
                <BreadcumbsFileNav parent_id={parent_id} access_token={token.clone()} />
                if has_favorites {
                    <div class="file_view__favorite_file_list">
                        <div class="file_view__favorite_file_list__header header">{"Favorites"}</div>
                        <FileList nodes={favorite_nodes} node_states={&self.node_states.clone()} action_callback={action_callback.clone()} />
                    </div>                    
                }
                if regular_nodes.len() > 0 {
                    if has_favorites {
                        <div class="file_view__file_list__header header">{"Files"}</div>
                    }
                    <FileList nodes={regular_nodes} node_states={&self.node_states.clone()} {action_callback} />
                }
            </>
        }
    }

    fn destroy(&mut self, ctx: &Context<Self>) {
        unsubscribe(ctx, &self.refresh_subsriber);
    }

}
