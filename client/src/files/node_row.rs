use std::borrow::Cow;

use wasm_bindgen::{UnwrapThrowExt, JsValue, JsCast};
use yew::prelude::*;
use yew_router::prelude::*;
use js_sys::{Date,Object};

use home_space_contracts::files::{DisplayFileNode, NODE_TYPE_FOLDER};

use super::actions::favorite_action::FavoriteAction;
use super::actions::select_action::SelectAction;
use super::files_view_component::FileViewActions;
use super::node_state::NodeState;

#[derive(Properties, PartialEq)]
pub(crate) struct NodeRowProps {
    pub(crate) action_callback: Callback<FileViewActions>,
    pub(crate) node: DisplayFileNode,
    pub(crate) state: NodeState
}

#[function_component(NodeRow)]
pub(crate) fn node_row(props: &NodeRowProps) -> Html {
    let DisplayFileNode { id, title, parent_id: _, node_type, mime_type, modified_at, node_size, is_favorite } = &props.node;
    let navigator = use_navigator().unwrap_throw();
    let on_node_title_click = {
        let id = *id;
        let node_type = props.node.node_type;
        Callback::from(move |_| {
            if node_type == NODE_TYPE_FOLDER {
                navigator.push(&crate::router::AppRoute::FileList{ parent_id: id });
            }
        })
    };

    let on_favorite = {
        let node_id = id.clone();
        let action_callback = props.action_callback.clone();
        Callback::from(move |is_favorite: bool| {
            action_callback.emit(FileViewActions::FileNodeFavoriteChanged((node_id, is_favorite)));
        })
    };

    let modified_at_local = Date::new(&JsValue::from_str(&modified_at)).unchecked_into::<Object>().to_locale_string();
    
    let on_selection = {
        let node_id = id.clone();
        let action_callback = props.action_callback.clone();
        Callback::from(move |selected: bool| {
            action_callback.emit(FileViewActions::FileNodeSelectionChanged((node_id, selected)));
        })
    };

    let on_node_row_click = {
        let node_id = id.clone();
        let action_callback = props.action_callback.clone();
        Callback::from(move |_| {
            action_callback.emit(FileViewActions::FileNodeSelectionToggle(node_id));
        })
    };

    html!{
        <div class="node-row" onclick={on_node_row_click}>
            <div class="node-row__actions">
                <SelectAction is_selected={props.state.is_selected} {on_selection} />
                <FavoriteAction is_favorite={is_favorite.clone()} {on_favorite} />
            </div>
            <div class="node-row__title">
                <span class="icon-filled">{get_node_icon(*node_type, &mime_type)}</span>
                <span class="node-row__title__name" onclick={on_node_title_click}>{title.clone()}</span>
                <span class="icon-filled file-item-menu node-row-action">{"more_vert"}</span>
            </div>
            <div class="node-row__node-size">{get_node_size(*node_type, *node_size)}</div>
            <div class="node-row__modified_at">{modified_at_local}</div>
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