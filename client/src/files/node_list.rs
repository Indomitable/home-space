use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;

use home_space_contracts::files::DisplayFileNode;

use super::node_list_header::NodeListHeader;
use super::files_view_component::{FileViewActions, NodeItems};
use super::node_state::NodesState;
use super::node_row::NodeRow;

#[derive(Properties, PartialEq)]
pub struct NodeListProps {
    pub node_items: NodeItems,
    pub node_states: Rc<RefCell<NodesState>>,
    pub action_callback: Callback<FileViewActions>
}

#[function_component(NodeList)]
pub fn node_list(props: &NodeListProps) -> Html {
    let states = &props.node_states.borrow().states;
    let all_nodes_selected = props.node_items.0.len() > 0 
    && props.node_items.0.iter().all(|n| match states.get(&n.id) {
        Some(n) => n.is_selected,
        None => false
    });
    html! {
        <div class="node-list">
            <NodeListHeader is_all_nodes_selected={all_nodes_selected} node_type={props.node_items.1} action_callback={props.action_callback.clone()} />
            {
                props.node_items.0.iter().map(|node: &DisplayFileNode| {
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
