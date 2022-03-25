use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::{UnwrapThrowExt, JsValue, JsCast};
use yew::prelude::*;
use yew_router::prelude::*;
use js_sys::{Date,Object};

use home_space_contracts::files::{DisplayFileNode, NODE_TYPE_FOLDER};

use super::file_list_header_component::FileListHeader;
use super::actions::favorite_action::FavoriteAction;
use super::actions::select_action::SelectAction;
use super::files_view_component::FileViewActions;
use super::node_state::{NodesState, NodeState};

#[derive(Properties, PartialEq)]
pub struct FileListProps {
    pub nodes: Vec<DisplayFileNode>,
    pub node_states: Rc<RefCell<NodesState>>,
    pub action_callback: Callback<FileViewActions>
}

#[function_component(FileList)]
pub fn file_nodes_component(props: &FileListProps) -> Html {
    html! {
        <div class="file-list">
            <FileListHeader />
            {
                props.nodes.iter().map(|node: &DisplayFileNode| {
                    let node_state = props.node_states.borrow();
                    let state = node_state.states.get(&node.id).unwrap_throw();
                    html!{
                        <NodeRow key={node.id} node={node.clone()} state={state.clone()} action_callback={props.action_callback.clone()} />
                    }
                }).collect::<Html>()
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct NodeRowProps {
    pub action_callback: Callback<FileViewActions>,
    node: DisplayFileNode,
    state: NodeState
}

#[function_component(NodeRow)]
fn node_row(props: &NodeRowProps) -> Html {
    let DisplayFileNode { id, title, parent_id: _, node_type, mime_type, modified_at, node_size, is_favorite } = &props.node;
    let navigator = use_navigator().unwrap_throw();
    let onclick = {
        let id = *id;
        let node_type = props.node.node_type;
        Callback::from(move |_| {
            if node_type == NODE_TYPE_FOLDER {
                navigator.push(crate::router::AppRoute::FileList{ parent_id: id });
            }
        })
    };

    let on_favorite = {
        let node_id = id.clone();
        let action_callback = props.action_callback.clone();
        Callback::from(move |is_favorite: bool| {
            action_callback.emit(FileViewActions::FileNodeFavoriteChanged((node_id, is_favorite)))
        })
    };

    let modified_at_local = Date::new(&JsValue::from_str(&modified_at)).unchecked_into::<Object>().to_locale_string();
    
    let on_selection = {
        let node_id = id.clone();
        let action_callback = props.action_callback.clone();
        Callback::from(move |selected: bool| {
            action_callback.emit(FileViewActions::FileNodeSelectionChanged((node_id, selected)))
        })
    };

    html!{
        <div class="file-list-row" {onclick}>
            <div class="file-item-actions">
                <SelectAction is_selected={props.state.is_selected} {on_selection} />
                <FavoriteAction is_favorite={is_favorite.clone()} {on_favorite} />
            </div>
            <div class="file-list__title">
                <span class="icon-filled">{get_node_icon(*node_type, &mime_type)}</span>
                <span>{title.clone()}</span>
                <span class="icon-filled file-item-menu file-item-action">{"more_vert"}</span>
            </div>
            <div class="file-list__node-size">{get_node_size(*node_type, *node_size)}</div>
            <div class="file-list__modified_at">{modified_at_local}</div>
        </div>
    }
}

fn get_node_icon<'a>(node_type: i16, mime_type: &'a str) -> &'a str {
    if node_type == 0 {
        return "folder";
    }
    match mime_type {
        _ => "insert_drive_file"
    }
}

const KBYTES: f64 = 1024_f64;
const MBYTES: f64 = 1048576_f64;
const GBYTES: f64 = 1073741824_f64;

fn get_node_size<'a>(node_type: i16, node_size: i64) -> Cow<'a, str> {
    if node_type == 0 {
        return "".into();
    } else {
        match node_size as f64 {
            bytes if bytes > GBYTES => format!("{:.2} GiB", bytes / GBYTES),
            bytes if bytes > MBYTES => format!("{:.2} MiB", bytes / MBYTES),
            bytes if bytes > KBYTES => format!("{:.2} KiB", bytes / KBYTES),
            bytes => bytes.to_string()
        }.into()
    }
}