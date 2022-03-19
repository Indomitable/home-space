use std::rc::Rc;

use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use yew_router::prelude::*;

use home_space_contracts::files::{FileNode, NODE_TYPE_FOLDER};

use super::file_list_header_component::FileListHeader;
use super::actions::favorite_action::FavoriteAction;
use super::node_actions::NodeActions;


#[derive(Properties, PartialEq)]
pub struct FileListProps {
    pub nodes: Vec<FileNode>,
    pub node_actions: Rc<NodeActions>
}

#[function_component(FileList)]
pub fn file_nodes_component(props: &FileListProps) -> Html {
    html! {
        <div class="file-list">
            <FileListHeader />
            {
                props.nodes.iter().map(|node: &FileNode| {
                    html!{
                        <NodeRow key={node.id} node={node.clone()} node_actions={props.node_actions.clone()} />
                    }
                }).collect::<Html>()
            }
        </div>
    }
}



#[derive(Properties, PartialEq)]
struct NodeRowProps {
    pub node_actions: Rc<NodeActions>,
    node: FileNode,
}

#[function_component(NodeRow)]
fn node_row(props: &NodeRowProps) -> Html {
    let FileNode { id, title, parent_id: _, node_type, mime_type, modified_at, node_size } = &props.node;
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
        let node_actions = props.node_actions.clone();
        let node_id = id.clone();
        Callback::from(move |is_favorite: bool| {
            node_actions.toggle_favorite(node_id, is_favorite)
        })
    };

    html!{
        <div class="file-list-row" {onclick}>
            <div class="file-item-actions">
                <span class="icon-outlined file-item-action">{"check_box_outline_blank"}</span>
                <FavoriteAction is_favorite={false} on_favorite={on_favorite} />
            </div>
            <div class="file-list-title">
                <span class="icon-filled">{get_node_icon(*node_type, &mime_type)}</span>
                <span>{title.clone()}</span>
                <span class="icon-filled file-item-menu file-item-action">{"more_vert"}</span>
            </div>
            <div>{node_size.clone()}</div>
            <div>{modified_at.clone()}</div>
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
