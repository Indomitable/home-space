use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;

use home_space_contracts::files::DisplayFileNode;

use super::file_list_header_component::FileListHeader;
use super::files_view_component::FileViewActions;
use super::node_state::NodesState;
use super::node_row::NodeRow;

#[derive(Properties, PartialEq)]
pub struct FileListProps {
    pub nodes: Vec<DisplayFileNode>,
    pub node_states: Rc<RefCell<NodesState>>,
    pub action_callback: Callback<FileViewActions>
}

#[function_component(NodeList)]
pub fn node_list(props: &FileListProps) -> Html {
    html! {
        <div class="node-list">
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
