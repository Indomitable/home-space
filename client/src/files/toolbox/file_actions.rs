use std::rc::Rc;

use yew::prelude::*;

use super::file_action_create::CreateAction;
use super::super::node_actions::NodeActions;

#[derive(Properties, PartialEq)]
pub struct FileActionsProps {
    pub parent_id: i64,
    pub node_actions: Rc<NodeActions>
}

#[function_component(FileActions)]
pub fn file_actions(props: &FileActionsProps) -> Html {
    html! {
        <ul class="file-actions">
            <li class="file-actions-create-container">
                <CreateAction parent_id={props.parent_id} node_actions={props.node_actions.clone()} />
            </li>
        </ul>
    }
}
